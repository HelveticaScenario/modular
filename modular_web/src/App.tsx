import { useCallback, useEffect, useRef, useState } from 'react';
import { useModularWebSocket } from './hooks/useWebSocket';
import { MonacoPatchEditor as PatchEditor } from './components/MonacoPatchEditor';
import { AudioControls } from './components/AudioControls';
import { ErrorDisplay } from './components/ErrorDisplay';
import { executePatchScript } from './dsl';
import { SchemasContext } from './SchemaContext';
import { useMonacoCollabAdapter } from './hooks/useMonacoCollabAdapter';
import './App.css';
import type { ModuleSchema } from './types/generated/ModuleSchema';
import type { ScopeItem } from './types/generated/ScopeItem';
import type { ValidationError } from './types/generated/ValidationError';
import type { editor } from 'monaco-editor';
import { findScopeCallEndLines } from './utils/findScopeCallEndLines';
import { FileExplorer } from './components/FileExplorer';
import type { OutputMessage } from './types/generated/OutputMessage';

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
const COLLAB_CLIENT_KEY = 'modular_collab_client_id';

type ScopeView = {
    key: string;
    lineNumber: number;
};

const scopeKeyFromSubscription = (subscription: ScopeItem) => {
    if (subscription.type === 'moduleOutput') {
        const { moduleId, portName } = subscription;
        return `:module:${moduleId}:${portName}`;
    }

    const { trackId } = subscription;
    return `:track:${trackId}`;
};

const drawOscilloscope = (data: Float32Array, canvas: HTMLCanvasElement) => {
    const ctx = canvas.getContext('2d');
    if (!ctx) return;

    const w = canvas.width;
    const h = canvas.height;

    ctx.fillStyle = 'rgb(30, 30, 30)';
    ctx.fillRect(0, 0, w, h);

    const midY = h / 2;
    const maxAbsAmplitude = 5;
    const pixelsPerUnit = h / 2 / maxAbsAmplitude;

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

    const totalSamples = data.length;
    const windowSize = 256;

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

    if (startIndex === -1) {
        startIndex = Math.floor(totalSamples / 2);
    }

    let endExclusive = startIndex + windowSize;
    if (endExclusive > totalSamples) {
        endExclusive = totalSamples;
    }
    const sampleCount = Math.max(0, endExclusive - startIndex);

    if (sampleCount < 2) {
        return;
    }

    ctx.strokeStyle = '#ffffff';
    ctx.lineWidth = 2;
    ctx.beginPath();

    const stepX = w / (sampleCount - 1);

    for (let i = 0; i < sampleCount; i++) {
        const x = stepX * i;
        const rawSample = data[startIndex + i];
        const s = Math.max(
            -maxAbsAmplitude,
            Math.min(maxAbsAmplitude, rawSample)
        );
        const y = midY - s * pixelsPerUnit;

        if (i === 0) {
            ctx.moveTo(x, y);
        } else {
            ctx.lineTo(x, y);
        }
    }

    ctx.stroke();
};

function App() {
    const handleMessageRef = useRef<(msg: OutputMessage) => void | null>(null);

    const [patchCode, setPatchCode] = useState('');

    const [isMuted, setIsMuted] = useState(true);
    const [isRecording, setIsRecording] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [validationErrors, setValidationErrors] = useState<
        ValidationError[] | null
    >(null);
    const [schemas, setSchemas] = useState<ModuleSchema[]>([]);
    const [scopeViews, setScopeViews] = useState<ScopeView[]>([]);

    const editorRef = useRef<editor.IStandaloneCodeEditor>(null);
    const scopeCanvasMapRef = useRef<Map<string, HTMLCanvasElement>>(new Map());
    const [files, setFiles] = useState<string[]>([]);
    const [currentFile, setCurrentFile] = useState<string | null>(null);

    const [clientId] = useState(() => {
        if (typeof window === 'undefined') return 'web-client';
        const existing = window.localStorage.getItem(COLLAB_CLIENT_KEY);
        if (existing) return existing;
        const generated = crypto.randomUUID?.() ?? `web-${Date.now()}`;
        window.localStorage.setItem(COLLAB_CLIENT_KEY, generated);
        return generated;
    });

    const registerScopeCanvas = useCallback(
        (key: string, canvas: HTMLCanvasElement) => {
            scopeCanvasMapRef.current.set(key, canvas);
        },
        []
    );

    const unregisterScopeCanvas = useCallback((key: string) => {
        scopeCanvasMapRef.current.delete(key);
    }, []);

    const handleMessageRefWrapper = useCallback(
        (msg: OutputMessage) => handleMessageRef.current?.(msg),
        []
    );

    const {
        connectionState,
        sendMessage,
        getSchemas,
        setPatch,
        mute,
        unmute,
        startRecording,
        stopRecording,
        listFiles,
        readFile,
    } = useModularWebSocket({ onMessage: handleMessageRefWrapper });

    const handleCollabOutputMessage = useMonacoCollabAdapter({
        editorRef,
        docId: currentFile ?? 'dsl-patch',
        clientId,
        sendMessage: (msg) => sendMessage(msg),
        connectionState,
        enabled: true,
        userDisplayName: clientId,
        initialText: patchCode,
        onTextUpdated: setPatchCode,
    });

    const handleMessage = useCallback(
        (msg: OutputMessage) => {
            if (handleCollabOutputMessage(msg)) return;
            switch (msg.type) {
                case 'schemas': {
                    setSchemas(msg.schemas);
                    if (typeof window !== 'undefined') {
                        window.__APP_SCHEMAS__ = msg.schemas;
                    }
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
                    setFiles(msg.files);
                    break;
                case 'fileContent': {
                    if (msg.path === currentFile) {
                        setPatchCode(msg.content);
                    }
                    break;
                }
            }
        },
        [currentFile, handleCollabOutputMessage]
    );

    useEffect(() => {
        handleMessageRef.current = handleMessage;
    }, [handleMessage]);

    useEffect(() => {
        listFiles();
    }, [listFiles]);

    useEffect(() => {
        if (currentFile) {
            readFile(currentFile);
        }
    }, [currentFile, readFile]);

    useEffect(() => {
        if (connectionState === 'connected') {
            getSchemas();
        }
    }, [connectionState, getSchemas]);

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
            const schemasValue = schemaRef.current;
            const patchCodeValue = patchCodeRef.current;
            const patch = executePatchScript(patchCodeValue, schemasValue);
            setPatch(patch);
            setError(null);
            setValidationErrors(null);

            const scopeCalls = findScopeCallEndLines(patchCodeValue);
            const views: ScopeView[] = patch.scopes
                .map((scope, idx) => {
                    const call = scopeCalls[idx];
                    if (!call) return null;
                    if (scope.type === 'moduleOutput') {
                        const { moduleId, portName } = scope;
                        return {
                            key: `:module:${moduleId}:${portName}`,
                            lineNumber: call.endLine,
                        };
                    }
                    const { trackId } = scope;
                    return {
                        key: `:track:${trackId}`,
                        lineNumber: call.endLine,
                    };
                })
                .filter((v): v is ScopeView => v !== null);

            setScopeViews(views);
        } catch (err) {
            const errorMessage =
                err instanceof Error ? err.message : 'Unknown error';
            setError(errorMessage);
            setValidationErrors(null);
        }
    }, [setPatch]);

    const handleStop = useCallback(() => {
        mute();
    }, [mute]);

    const dismissError = useCallback(() => {
        setError(null);
        setValidationErrors(null);
    }, []);

    useEffect(() => {
        const handleKeyDown = (e: KeyboardEvent) => {
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
    }, [isRecording, startRecording, stopRecording]);

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

                    <FileExplorer
                        files={files}
                        currentFile={currentFile}
                        onFileSelect={setCurrentFile}
                        onRefresh={listFiles}
                    />
                </main>
            </div>
        </SchemasContext.Provider>
    );
}

export default App;
