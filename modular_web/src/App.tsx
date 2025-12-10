import { useCallback, useEffect, useRef, useState } from 'react';
import { useModularWebSocket } from './hooks/useWebSocket';
import { MonacoPatchEditor as PatchEditor } from './components/MonacoPatchEditor';
import { AudioControls } from './components/AudioControls';
import { ErrorDisplay } from './components/ErrorDisplay';
import { executePatchScript } from './dsl';
import { SchemasContext } from './SchemaContext';
import './App.css';
import type { ModuleSchema } from './types/generated/ModuleSchema';
import type { ScopeItem } from './types/generated/ScopeItem';
import type { ValidationError } from './types/generated/ValidationError';
import type { editor } from 'monaco-editor';
import { findScopeCallEndLines } from './utils/findScopeCallEndLines';
import { FileExplorer, SCRATCH_FILE } from './components/FileExplorer';
import type { OutputMessage } from './types/generated/OutputMessage';
import { exhaustiveSwitch } from './utils/exhaustingSwitch';

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
const UNSAVED_STORAGE_KEY = 'modular_unsaved_buffers';

type UnsavedBufferSnapshot = {
    content: string;
    isNew?: boolean;
};

type FileBuffer = {
    content: string;
    dirty: boolean;
    isNew?: boolean;
    loaded?: boolean;
};

type ScopeView = {
    key: string;
    lineNumber: number;
    file: string;
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
            Math.min(maxAbsAmplitude, rawSample),
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

const readUnsavedBuffers = (): Record<string, UnsavedBufferSnapshot> => {
    if (typeof window === 'undefined') {
        return {};
    }

    try {
        const raw = window.localStorage.getItem(UNSAVED_STORAGE_KEY);
        const parsed = raw ? JSON.parse(raw) : {};
        const buffers: Record<string, UnsavedBufferSnapshot> = {};

        if (parsed && typeof parsed === 'object') {
            Object.entries(parsed).forEach(([path, value]) => {
                if (
                    value &&
                    typeof value === 'object' &&
                    typeof (value as { content?: unknown }).content === 'string'
                ) {
                    const snapshot = value as UnsavedBufferSnapshot;
                    buffers[path] = {
                        content: snapshot.content,
                        isNew: snapshot.isNew,
                    };
                }
            });
        }

        const legacyScratch = window.localStorage.getItem(PATCH_STORAGE_KEY);
        if (legacyScratch && !buffers[SCRATCH_FILE]) {
            buffers[SCRATCH_FILE] = { content: legacyScratch, isNew: true };
        }

        return buffers;
    } catch {
        return {};
    }
};

const getInitialPatch = (
    unsavedBuffers: Record<string, UnsavedBufferSnapshot>,
) => {
    const cachedScratch = unsavedBuffers[SCRATCH_FILE]?.content;
    if (cachedScratch) {
        return cachedScratch;
    }

    if (typeof window === 'undefined') {
        return DEFAULT_PATCH;
    }

    const storedPatch = window.localStorage.getItem(PATCH_STORAGE_KEY);
    return storedPatch ?? DEFAULT_PATCH;
};

const buildInitialFileBuffers = (
    unsavedBuffers: Record<string, UnsavedBufferSnapshot>,
): Record<string, FileBuffer> => {
    const scratchSnapshot = unsavedBuffers[SCRATCH_FILE];
    const scratchContent = getInitialPatch(unsavedBuffers);

    const initialBuffers: Record<string, FileBuffer> = {
        [SCRATCH_FILE]: {
            content: scratchContent,
            dirty: Boolean(scratchSnapshot),
            isNew: scratchSnapshot?.isNew ?? true,
            loaded: true,
        },
    };

    Object.entries(unsavedBuffers).forEach(([path, snapshot]) => {
        if (path === SCRATCH_FILE) return;

        initialBuffers[path] = {
            content: snapshot.content,
            dirty: true,
            isNew: snapshot.isNew ?? false,
            loaded: false,
        };
    });

    return initialBuffers;
};

const buildInitialOpenFiles = (
    unsavedBuffers: Record<string, UnsavedBufferSnapshot>,
) => {
    const unsavedFiles = Object.keys(unsavedBuffers).filter(
        (file) => file !== SCRATCH_FILE,
    );
    return [SCRATCH_FILE, ...unsavedFiles];
};

function App() {
    const [unsavedSnapshots] = useState<Record<string, UnsavedBufferSnapshot>>(
        () => readUnsavedBuffers(),
    );

    const [patchCode, setPatchCode] = useState<string>(() =>
        getInitialPatch(unsavedSnapshots),
    );
    const [fileBuffers, setFileBuffers] = useState<Record<string, FileBuffer>>(
        () => buildInitialFileBuffers(unsavedSnapshots),
    );

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
    const [openFiles, setOpenFiles] = useState<string[]>(() =>
        buildInitialOpenFiles(unsavedSnapshots),
    );
    const [currentFile, setCurrentFile] = useState<string>(SCRATCH_FILE);
    console.log('Current file in App:', currentFile);
    const registerScopeCanvas = useCallback(
        (key: string, canvas: HTMLCanvasElement) => {
            scopeCanvasMapRef.current.set(key, canvas);
        },
        [],
    );

    const unregisterScopeCanvas = useCallback((key: string) => {
        scopeCanvasMapRef.current.delete(key);
    }, []);

    const handleMessage = useCallback(
        (msg: OutputMessage) => {
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
                    setFileBuffers((prev) => {
                        const existing = prev[msg.path];

                        if (
                            existing &&
                            existing.dirty &&
                            existing.content !== msg.content
                        ) {
                            return prev;
                        }

                        return {
                            ...prev,
                            [msg.path]: {
                                content: msg.content,
                                dirty: false,
                                isNew: false,
                                loaded: true,
                            },
                        };
                    });

                    setOpenFiles((prev) =>
                        prev.includes(msg.path) ? prev : [...prev, msg.path],
                    );

                    if (msg.path === currentFile) {
                        setPatchCode(msg.content);
                    }
                    break;
                }
                default:
                    exhaustiveSwitch(msg);
            }
        },
        [currentFile],
    );

    const {
        connectionState,
        getSchemas,
        setPatch,
        mute,
        unmute,
        startRecording,
        stopRecording,
        listFiles,
        readFile,
        writeFile,
        renameFile,
    } = useModularWebSocket({ onMessage: handleMessage });

    useEffect(() => {
        listFiles();
    }, [listFiles]);

    useEffect(() => {
        if (typeof window === 'undefined') {
            return;
        }

        const unsavedEntries: Record<string, UnsavedBufferSnapshot> = {};
        Object.entries(fileBuffers).forEach(([path, buffer]) => {
            if (buffer?.dirty) {
                unsavedEntries[path] = {
                    content: buffer.content,
                    isNew: buffer.isNew,
                };
            }
        });

        try {
            window.localStorage.setItem(
                UNSAVED_STORAGE_KEY,
                JSON.stringify(unsavedEntries),
            );
            window.localStorage.removeItem(PATCH_STORAGE_KEY);
        } catch {
            // Ignore storage quota/access issues to avoid breaking editing flow
        }
    }, [fileBuffers]);

    // Request initial state when connected
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

    useEffect(() => {
        const active = currentFile || SCRATCH_FILE;
        const buffer = fileBuffers[active];

        if (buffer && buffer.content !== patchCode) {
            setPatchCode(buffer.content);
        }
    }, [currentFile, fileBuffers, patchCode]);

    const formatFileLabel = useCallback(
        (file: string) => (file === SCRATCH_FILE ? 'Scratch Pad' : file),
        [],
    );

    const normalizeFileName = useCallback((name: string) => {
        const trimmed = name.trim();
        if (!trimmed) {
            return trimmed;
        }
        return trimmed.endsWith('.mjs') ? trimmed : `${trimmed}.mjs`;
    }, []);

    const handlePatchChange = useCallback(
        (value: string) => {
            setPatchCode(value);
            const active = currentFile || SCRATCH_FILE;
            setFileBuffers((prev) => {
                const existing = prev[active] ?? { content: '', dirty: false };
                return {
                    ...prev,
                    [active]: {
                        ...existing,
                        content: value,
                        dirty: true,
                        loaded: true,
                    },
                };
            });
        },
        [currentFile],
    );

    const selectFile = useCallback(
        (filename: string) => {
            const target = filename || SCRATCH_FILE;

            setOpenFiles((prev) =>
                prev.includes(target) ? prev : [...prev, target],
            );
            console.log('Selecting file:', target);
            setCurrentFile(target);

            const buffer = fileBuffers[target];

            if (!buffer || buffer.loaded === false) {
                setFileBuffers((prev) => ({
                    ...prev,
                    [target]: buffer ?? {
                        content: '',
                        dirty: false,
                        loaded: false,
                    },
                }));
                if (target !== SCRATCH_FILE) {
                    readFile(target);
                }
                setPatchCode(buffer?.content ?? '');
                return;
            }

            if (buffer.content !== patchCode) {
                setPatchCode(buffer.content);
            }
        },
        [fileBuffers, patchCode, readFile],
    );

    const handleCreateFile = useCallback(() => {
        const input = window.prompt('Create new patch file (.js)');
        if (!input) return;

        const normalized = normalizeFileName(input);
        if (!normalized) return;

        if (files.includes(normalized) || fileBuffers[normalized]) {
            setError(`A file named ${normalized} already exists.`);
            return;
        }

        const initialContent = DEFAULT_PATCH;
        setFileBuffers((prev) => ({
            ...prev,
            [normalized]: {
                content: initialContent,
                dirty: true,
                isNew: true,
                loaded: true,
            },
        }));
        setOpenFiles((prev) =>
            prev.includes(normalized) ? prev : [...prev, normalized],
        );
        setCurrentFile(normalized);
        setPatchCode(initialContent);
    }, [files, fileBuffers, normalizeFileName]);

    const handleRenameFile = useCallback(() => {
        const active = currentFile || SCRATCH_FILE;
        const buffer = fileBuffers[active];
        const suggestion = active === SCRATCH_FILE ? 'untitled.mjs' : active;
        const nextName = window.prompt('Rename file', suggestion);
        if (!nextName) return;

        const normalized = normalizeFileName(nextName);
        if (!normalized || normalized === active) {
            return;
        }

        if (files.includes(normalized) || fileBuffers[normalized]) {
            setError(`A file named ${normalized} already exists.`);
            return;
        }

        if (active !== SCRATCH_FILE && !buffer?.isNew) {
            renameFile(active, normalized);
        }

        setFileBuffers((prev) => {
            const { [active]: currentBuffer, ...rest } = prev;
            const nextBuffer = currentBuffer ?? {
                content: patchCodeRef.current,
                dirty: true,
            };
            return {
                ...rest,
                [normalized]: {
                    ...nextBuffer,
                    dirty: nextBuffer.dirty,
                    isNew: currentBuffer?.isNew,
                    loaded: true,
                },
            };
        });
        setOpenFiles((prev) =>
            prev.map((file) => (file === active ? normalized : file)),
        );
        setCurrentFile(normalized);
        listFiles();
    }, [
        currentFile,
        fileBuffers,
        files,
        listFiles,
        normalizeFileName,
        renameFile,
    ]);

    const handleSaveFileRef = useRef(() => {});
    useEffect(() => {
        handleSaveFileRef.current = () => {
            console.log('Handle save file');
            const active = currentFile || SCRATCH_FILE;
            console.log('Current file:', currentFile);
            console.log('Active file:', active);
            console.log('File buffers:', fileBuffers);
            const buffer = fileBuffers[active];
            let target = active;
            console.log('Buffer:', buffer);

            if (active === SCRATCH_FILE || buffer?.isNew) {
                const nextName = window.prompt(
                    'Save file as (.mjs)',
                    active === SCRATCH_FILE ? 'patch.mjs' : active,
                );
                if (!nextName) return;

                const normalized = normalizeFileName(nextName);
                if (!normalized) return;

                if (files.includes(normalized) && normalized !== active) {
                    setError(`A file named ${normalized} already exists.`);
                    return;
                }

                target = normalized;

                setFileBuffers((prev) => {
                    const { [active]: currentBuffer, ...rest } = prev;
                    const nextBuffer = currentBuffer ?? {
                        content: patchCodeRef.current,
                        dirty: true,
                    };
                    return {
                        ...rest,
                        [normalized]: {
                            ...nextBuffer,
                            content: patchCodeRef.current,
                            dirty: true,
                            isNew: false,
                            loaded: true,
                        },
                    };
                });

                setOpenFiles((prev) =>
                    prev.includes(normalized) ? prev : [...prev, normalized],
                );
                setCurrentFile(normalized);
            }

            writeFile(target, patchCodeRef.current);
            setFileBuffers((prev) => ({
                ...prev,
                [target]: {
                    ...(prev[target] ?? {
                        content: patchCodeRef.current,
                        dirty: false,
                    }),
                    content: patchCodeRef.current,
                    dirty: false,
                    isNew: false,
                    loaded: true,
                },
            }));
            listFiles();
        };
    }, [
        currentFile,
        fileBuffers,
        files,
        listFiles,
        normalizeFileName,
        writeFile,
    ]);

    const handleSubmitRef = useRef(() => {});
    useEffect(() => {
        handleSubmitRef.current = () => {
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
                                file: currentFile,
                            };
                        }
                        const { trackId } = scope;
                        return {
                            key: `:track:${trackId}`,
                            lineNumber: call.endLine,
                            file: currentFile,
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
        };
    }, [currentFile, setPatch]);

    const handleStopRef = useRef(() => {});
    useEffect(() => {
        handleStopRef.current = () => {
            mute();
        };
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
                        onUpdatePatch={handleSubmitRef.current}
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
                            currentFile={currentFile}
                            onChange={handlePatchChange}
                            onSubmit={handleSubmitRef}
                            onStop={handleStopRef}
                            onSave={handleSaveFileRef}
                            editorRef={editorRef}
                            schemas={schemas}
                            scopeViews={scopeViews}
                            onRegisterScopeCanvas={registerScopeCanvas}
                            onUnregisterScopeCanvas={unregisterScopeCanvas}
                        />
                    </div>

                    <FileExplorer
                        files={files}
                        openFiles={openFiles}
                        currentFile={currentFile}
                        fileStates={fileBuffers}
                        formatLabel={formatFileLabel}
                        onFileSelect={selectFile}
                        onRefresh={listFiles}
                        onCreateFile={handleCreateFile}
                        onSaveFile={handleSaveFileRef.current}
                        onRenameFile={handleRenameFile}
                    />
                </main>
            </div>
        </SchemasContext.Provider>
    );
}

export default App;
