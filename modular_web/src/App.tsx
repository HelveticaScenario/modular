import { useCallback, useState, useEffect, useRef } from 'react';
import { useModularWebSocket, type OutputMessage } from './hooks/useWebSocket';
// import { PatchEditor } from './components/PatchEditor';
import { MonacoPatchEditor as PatchEditor } from './components/MonacoPatchEditor';
import { AudioControls } from './components/AudioControls';
import { ErrorDisplay } from './components/ErrorDisplay';
import { executePatchScript } from './dsl';
import { updateTsWorkerSchemas } from './lsp/tsClient';
import { SchemasContext } from './SchemaContext';
import './App.css';
import type { ModuleSchema } from './types/generated/ModuleSchema';
import type { ScopeItem } from './types/generated/ScopeItem';
import type { ValidationError } from './types/generated/ValidationError';
import type { editor } from 'monaco-editor';
import { findScopeCallEndLines } from './utils/findScopeCallEndLines';

declare global {
    interface Window {
        __APP_SCHEMAS__?: ModuleSchema[];
    }
}

const DEFAULT_PATCH = `// Simple 440 Hz sine wave
const osc = sine('osc1').freq(hz(440));
out.source(osc);
`;

const PATCH_STORAGE_KEY = 'modular_patch_dsl';

const width = 800;
const height = 200;

type ScopeView = {
    key: string;
    lineNumber: number;
};

const scopeKeyFromSubscription = (subscription: ScopeItem) => {
    if (subscription.type === 'moduleOutput') {
        const { module_id, port_name } = subscription;
        return `:module:${module_id}:${port_name}`;
    }

    const { track_id } = subscription;
    return `:track:${track_id}`;
};

const drawOscilloscope = (data: Float32Array, canvas: HTMLCanvasElement) => {
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const w = canvas.width;
    const h = canvas.height;

    // Clear canvas background
    ctx.fillStyle = 'rgb(30, 30, 30)';
    ctx.fillRect(0, 0, w, h);

    const midY = h / 2;
    const maxAbsAmplitude = 5; // expected sample range is roughly [-5, 5]
    const pixelsPerUnit = h / 2 / maxAbsAmplitude; // so ±5 spans full height

    // Center line at 0.0
    ctx.strokeStyle = '#333';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, midY);
    ctx.lineTo(w, midY);
    ctx.stroke();

    if (!data || data.length === 0) {
        ctx.fillStyle = '#666';
        ctx.font = '14px monospace';
        ctx.textAlign = 'center';
        ctx.fillText('No Signal', w / 2, midY);
        return;
    }

    const totalSamples = data.length; // typically 512
    const windowSize = 256;

    // 1. Find first positive-going zero-crossing (prev <= 0, curr > 0)
    let startIndex = -1;
    for (let i = 1; i < totalSamples; i++) {
        const prev = data[i - 1];
        const curr = data[i];
        const crossedZero = prev <= 0 && curr > 0;
        if (crossedZero) {
            startIndex = i;
            break;
        }
    }

    // 2. Fallback: if no zero-crossing found, start from midpoint of buffer
    if (startIndex === -1) {
        startIndex = Math.floor(totalSamples / 2); // e.g. 256 for 512-sample buffer
    }

    // 3. Compute window [startIndex, startIndex + windowSize)
    let endExclusive = startIndex + windowSize;
    if (endExclusive > totalSamples) {
        endExclusive = totalSamples;
    }
    const sampleCount = Math.max(0, endExclusive - startIndex);

    if (sampleCount < 2) {
        // Not enough data to draw a waveform
        return;
    }

    // Draw waveform across full width using only the selected window
    ctx.strokeStyle = '#ffffff';
    ctx.lineWidth = 2;
    ctx.beginPath();

    const stepX = w / (sampleCount - 1);

    for (let i = 0; i < sampleCount; i++) {
        const x = stepX * i;
        const rawSample = data[startIndex + i];
        // Clamp to expected range so outliers don't blow up the scale
        const s = Math.max(
            -maxAbsAmplitude,
            Math.min(maxAbsAmplitude, rawSample)
        );
        // Canvas Y grows downward, so positive samples move up (subtract)
        const y = midY - s * pixelsPerUnit;

        if (i === 0) {
            ctx.moveTo(x, y);
        } else {
            ctx.lineTo(x, y);
        }
    }

    ctx.stroke();
};

// TODO persist yaml code to local storage and load on startup
function App() {
    const [patchCode, setPatchCode] = useState(() => {
        if (typeof window === 'undefined') {
            return DEFAULT_PATCH;
        }

        const storedPatch = window.localStorage.getItem(PATCH_STORAGE_KEY);
        return storedPatch ?? DEFAULT_PATCH;
    });

    const [isMuted, setIsMuted] = useState(true);

    const [isRecording, setIsRecording] = useState(false);

    const [error, setError] = useState<string | null>(null);
    const [validationErrors, setValidationErrors] = useState<
        ValidationError[] | null
    >(null);

    const [schemas, setSchemas] = useState<ModuleSchema[]>([]);
    const [scopeViews, setScopeViews] = useState<ScopeView[]>([]);

    const editorRef = useRef<editor.IStandaloneCodeEditor>(null);

    const canvasRef = useRef<HTMLCanvasElement>(null);
    const scopeCanvasMapRef = useRef<Map<string, HTMLCanvasElement>>(new Map());

    const registerScopeCanvas = useCallback(
        (key: string, canvas: HTMLCanvasElement) => {
            scopeCanvasMapRef.current.set(key, canvas);
        },
        []
    );

    const unregisterScopeCanvas = useCallback((key: string) => {
        scopeCanvasMapRef.current.delete(key);
    }, []);
    const handleMessage = useCallback(
        (msg: OutputMessage) => {
            switch (msg.type) {
                case 'schemas': {
                    console.log('Received schemas:', msg.schemas);
                    setSchemas(msg.schemas);
                    if (typeof window !== 'undefined') {
                        window.__APP_SCHEMAS__ = msg.schemas;
                    }
                    void updateTsWorkerSchemas(msg.schemas);
                    break;
                }
                case 'error':
                    setError(msg.message);
                    setValidationErrors(msg.errors ?? null);
                    break;
                case 'muteState':
                    setIsMuted(msg.muted);
                    break;
                case 'audioBuffer': {
                    const scopeKey = scopeKeyFromSubscription(msg.subscription);
                    const scopedCanvas =
                        scopeCanvasMapRef.current.get(scopeKey);
                    if (scopedCanvas) {
                        drawOscilloscope(msg.samples, scopedCanvas);
                        break;
                    }

                    break;
                }
                case 'fileList':
                    // Handle file list if needed
                    break;
                case 'fileContent':
                    // Handle file content if needed
                    break;
            }
        },
        [setError, setSchemas, setValidationErrors, setIsMuted]
    );

    const {
        connectionState,
        getPatch,
        getSchemas,
        setPatch,
        mute,
        unmute,
        startRecording,
        stopRecording,
    } = useModularWebSocket({ onMessage: handleMessage });

    useEffect(() => {
        if (typeof window === 'undefined') {
            return;
        }
        try {
            window.localStorage.setItem(PATCH_STORAGE_KEY, patchCode);
        } catch {
            // Ignore storage quota/access issues to avoid breaking editing flow
        }
    }, [patchCode]);

    // Request initial state when connected
    useEffect(() => {
        if (connectionState === 'connected') {
            getSchemas();
        }
    }, [connectionState, getPatch, getSchemas]);

    const schemaRef = useRef<ModuleSchema[]>([]);
    useEffect(() => {
        schemaRef.current = schemas;
    }, [schemas]);
    const patchCodeRef = useRef<string>(patchCode);
    useEffect(() => {
        patchCodeRef.current = patchCode;
    }, [patchCode]);
    const handleSubmit = useCallback(() => {
        try {
            const schemas = schemaRef.current;
            const patchCode = patchCodeRef.current;
            // Execute DSL script to generate PatchGraph
            const patch = executePatchScript(patchCode, schemas);
            setPatch(patch);
            setError(null);
            setValidationErrors(null);

            const scopeCalls = findScopeCallEndLines(patchCode);

            const views: ScopeView[] = patch.scopes
                .map((scope, idx) => {
                    const call = scopeCalls[idx];
                    if (!call) return null;
                    if (scope.type === 'moduleOutput') {
                        const { module_id, port_name } = scope;
                        return {
                            key: `:module:${module_id}:${port_name}`,
                            lineNumber: call.endLine,
                        };
                    } else {
                        const { track_id } = scope;
                        return {
                            key: `:track:${track_id}`,
                            lineNumber: call.endLine,
                        };
                    }
                })
                .filter((v): v is ScopeView => v !== null);

            setScopeViews(views);
        } catch (err) {
            const errorMessage =
                err instanceof Error ? err.message : 'Unknown error';
            setError(errorMessage);
            setValidationErrors(null);
        }
    }, [setPatch, setError, setValidationErrors]);

    const handleStop = useCallback(() => {
        mute();
    }, [mute]);

    const dismissError = useCallback(() => {
        setError(null);
        setValidationErrors(null);
    }, []);

    // Global keyboard shortcuts
    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
            // If ctrl or alt + R, toggle recording
            if ((e.ctrlKey || e.altKey) && (e.key === 'r' || e.key === 'R')) {
                if (e.altKey) {
                    e.preventDefault();
                }
                if (isRecording) {
                    stopRecording();
                } else {
                    startRecording();
                }
            }
        };

        window.addEventListener('keydown', handleKeyDown);
        return () => window.removeEventListener('keydown', handleKeyDown);
    }, [isMuted, isRecording, startRecording, stopRecording]);

    return (
        <SchemasContext.Provider value={schemas}>
            <div className="app">
                <header className="app-header">
                    <h1>Jeff</h1>
                    <AudioControls
                        connectionState={connectionState}
                        isPlaying={!isMuted}
                        isRecording={isRecording}
                        onStartAudio={() => {
                            setIsMuted(false);
                            unmute();
                        }}
                        onStopAudio={() => {
                            setIsMuted(true);
                            mute();
                        }}
                        onStartRecording={() => {
                            setIsRecording(true);
                            startRecording();
                        }}
                        onStopRecording={() => {
                            setIsRecording(false);
                            stopRecording();
                        }}
                        onUpdatePatch={handleSubmit}
                    />
                </header>

                <ErrorDisplay
                    error={error}
                    errors={validationErrors}
                    onDismiss={dismissError}
                />

                <main className="app-main">
                    <div className="editor-panel">
                        <PatchEditor
                            value={patchCode}
                            onChange={setPatchCode}
                            onSubmit={handleSubmit}
                            onStop={handleStop}
                            editorRef={editorRef}
                            schemas={schemas}
                            scopeViews={scopeViews}
                            onRegisterScopeCanvas={registerScopeCanvas}
                            onUnregisterScopeCanvas={unregisterScopeCanvas}
                        />
                    </div>

                    <div className="visualization-panel">
                        <div className="oscilloscope">
                            <canvas
                                ref={canvasRef}
                                width={width}
                                height={height}
                                style={{
                                    width: '100%',
                                    height: 'auto',
                                    maxWidth: width,
                                }}
                            />
                        </div>
                        <div className="keyboard-shortcuts">
                            <h3>Keyboard Shortcuts</h3>
                            <ul>
                                <li>
                                    <kbd>Alt</kbd>+<kbd>Enter</kbd> Execute DSL
                                </li>
                                <li>
                                    <kbd>Alt</kbd>+<kbd>.</kbd> Stop Audio
                                </li>
                                <li>
                                    <kbd>Ctrl</kbd>+<kbd>R</kbd> Toggle
                                    Recording
                                </li>
                            </ul>
                        </div>
                    </div>
                </main>
            </div>
        </SchemasContext.Provider>
    );
}

export default App;
