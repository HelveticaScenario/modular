import { useCallback, useState, useEffect } from 'react';
import { useModularWebSocket, type OutputMessage } from './hooks/useWebSocket';
import { PatchEditor } from './components/PatchEditor';
import { Oscilloscope } from './components/Oscilloscope';
import { AudioControls } from './components/AudioControls';
import { ErrorDisplay } from './components/ErrorDisplay';
import type { ValidationError, ModuleSchema } from './types';
import './App.css';

const DEFAULT_PATCH = `modules:
  - id: osc1
    module_type: sine-osc
    params:
      freq:
        param_type: hz
        value: 440.0
  - id: root
    module_type: signal
    params:
      source:
        param_type: cable
        module: osc1
        port: output
`;

const PATCH_STORAGE_KEY = 'modular_patch_yaml';

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
    const [oscilloscopeData, setOscilloscopeData] = useState<number[] | null>(
        null
    );
    const [error, setError] = useState<string | null>(null);
    const [validationErrors, setValidationErrors] = useState<
        ValidationError[] | null
    >(null);
    const [, setSchemas] = useState<ModuleSchema[]>([]);
    const handleMessage = useCallback((msg: OutputMessage) => {
        switch (msg.type) {
            case 'patch':
                // Convert patch object back to YAML for editor
                setPatchYaml(msg.patch);
                setError(null);
                setValidationErrors(null);
                break;
            case 'schemas':
                setSchemas(msg.schemas);
                break;
            case 'error':
                setError(msg.message);
                setValidationErrors(msg.errors ?? null);
                break;
            case 'audioBuffer':
                console.log('Audio buffer received', msg.samples.length);
                setOscilloscopeData(msg.samples);
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
        // Send raw YAML to server - server will parse and validate
        setPatch(patchYaml);
    }, [setPatch, patchYaml]);

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
                    />
                </div>

                <div className="visualization-panel">
                    <Oscilloscope data={oscilloscopeData} />
                    <div className="keyboard-shortcuts">
                        <h3>Keyboard Shortcuts</h3>
                        <ul>
                            <li>
                                <kbd>Ctrl</kbd>+<kbd>Enter</kbd> Update Patch
                            </li>
                            <li>
                                <kbd>Ctrl</kbd>+<kbd>.</kbd> Stop Audio
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
