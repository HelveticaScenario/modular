import { useCallback, useState, useEffect, useRef } from 'react';
import { useModularWebSocket, type OutputMessage } from './hooks/useWebSocket';
import { PatchEditor } from './components/PatchEditor';
import { Oscilloscope } from './components/Oscilloscope';
import { AudioControls } from './components/AudioControls';
import { ErrorDisplay } from './components/ErrorDisplay';
import type { ValidationError, ModuleSchema } from './types';
import { executePatchScript } from './dsl';
import './App.css';

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

    // Clear canvas
    ctx.fillStyle = '#1a1a1a';
    ctx.fillRect(0, 0, width, height);

    // Draw center line
    ctx.strokeStyle = '#333';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, height / 2);
    ctx.lineTo(width, height / 2);
    ctx.stroke();

    if (!data || data.length === 0) {
        // Draw "No Signal" text
        ctx.fillStyle = '#666';
        ctx.font = '14px monospace';
        ctx.textAlign = 'center';
        ctx.fillText('No Signal', width / 2, height / 2);
        return;
    }

    // Draw waveform
    ctx.strokeStyle = '#00ff00';
    ctx.lineWidth = 2;
    ctx.beginPath();

    const step = width / data.length;
    const midY = height / 2;
    const amplitude = height / 2 - 10;

    for (let i = 0; i < data.length; i++) {
        const x = i * step;
        const y = midY - data[i] * amplitude;

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
    const [patchYaml, setPatchYaml] = useState(() => {
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
    const handleMessage = useCallback((msg: OutputMessage) => {
        // console.log('Received message:', msg);
        switch (msg.type) {
            case 'schemas':
                setSchemas(msg.schemas);
                break;
            case 'error':
                setError(msg.message);
                setValidationErrors(msg.errors ?? null);
                break;
            case 'audioBuffer': {
                // console.log('Audio buffer received', msg.samples.length);
                const canvas = canvasRef.current;
                if (!canvas) break;
                drawOscilloscope(msg.samples, canvas);
                // setOscilloscopeData(msg.samples);
                break;
            }
            case 'fileList':
                // Handle file list if needed
                break;
            case 'fileContent':
                // Handle file content if needed
                break;
        }
    }, []);

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
            window.localStorage.setItem(PATCH_STORAGE_KEY, patchYaml);
        } catch {
            // Ignore storage quota/access issues to avoid breaking editing flow
        }
    }, [patchYaml]);

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
            const patch = executePatchScript(patchYaml, schemas);
            setPatch(patch);
            setError(null);
            setValidationErrors(null);
        } catch (err) {
            const errorMessage =
                err instanceof Error ? err.message : 'Unknown error';
            setError(errorMessage);
            setValidationErrors(null);
        }
    }, [setPatch, patchYaml, schemas]);

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
                        value={patchYaml}
                        onChange={setPatchYaml}
                        onSubmit={handleSubmit}
                        onStop={handleStop}
                        disabled={connectionState !== 'connected'}
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
                                <kbd>Ctrl</kbd>+<kbd>R</kbd> Toggle Recording
                            </li>
                        </ul>
                    </div>
                </div>
            </main>
        </div>
    );
}

export default App;
