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
import type { ValidationError } from './types/generated/ValidationError';

const DEFAULT_PATCH = `// Simple 440 Hz sine wave
const osc = sine('osc1').freq(hz(440));
out.source(osc);
`;

const PATCH_STORAGE_KEY = 'modular_patch_dsl';

const width = 800;
const height = 200;

const drawOscilloscope = (data: Float32Array, canvas: HTMLCanvasElement) => {
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const w = canvas.width;
    const h = canvas.height;

    // Clear canvas background
    ctx.fillStyle = '#1a1a1a';
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
    ctx.strokeStyle = '#00ff00';
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

    const canvasRef = useRef<HTMLCanvasElement>(null);
    const handleMessage = useCallback(
        (msg: OutputMessage) => {
            switch (msg.type) {
                case 'schemas': {
                    console.log('Received schemas:', msg.schemas);
                    setSchemas(msg.schemas);
                    if (typeof window !== 'undefined') {
                        (window as any).__APP_SCHEMAS__ = msg.schemas;
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
                    // console.log('Audio buffer received', msg.samples.length);
                    const canvas = canvasRef.current;
                    if (!canvas) break;
                    drawOscilloscope(msg.samples, canvas);
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
        subscribeAudio,
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
            // Subscribe to root module output for oscilloscope
            subscribeAudio('root', 'output');
        }
    }, [connectionState, getPatch, getSchemas, subscribeAudio]);

    const handleSubmit = useCallback(() => {
        try {
            // Execute DSL script to generate PatchGraph
            console.log(schemas);
            const patch = executePatchScript(patchCode, schemas);
            setPatch(patch);
            setError(null);
            setValidationErrors(null);
        } catch (err) {
            const errorMessage =
                err instanceof Error ? err.message : 'Unknown error';
            setError(errorMessage);
            setValidationErrors(null);
        }
    }, [patchCode, schemas, setPatch, setError, setValidationErrors]);
    console.log('schemas in App:', schemas);

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
                    <h1>Modular Synthesizer</h1>
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
                            schemas={schemas}
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
