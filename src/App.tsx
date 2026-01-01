import { useCallback, useEffect, useRef, useState } from 'react';
import { MonacoPatchEditor as PatchEditor } from './components/MonacoPatchEditor';
import { AudioControls } from './components/AudioControls';
import { ErrorDisplay } from './components/ErrorDisplay';
import { executePatchScript } from './dsl';
import { SchemasContext } from './SchemaContext';
import './App.css';
import type { editor } from 'monaco-editor';
import { findScopeCallEndLines } from './utils/findScopeCallEndLines';
import { FileExplorer, SCRATCH_FILE } from './components/FileExplorer';
import electronAPI from './electronAPI';
import { ModuleSchema, ScopeItem, ValidationError } from '@modular/core';
import type { FileTreeEntry } from './ipcTypes';

const DEFAULT_PATCH = `// Simple 440 Hz sine wave
const osc = sine('osc1').freq(hz(440));
out.source(osc);
`;

const UNSAVED_STORAGE_KEY = 'modular_unsaved_buffers';

// New buffer model: distinguish between file-backed and untitled buffers
type EditorBuffer =
    | { kind: 'file'; relPath: string; content: string; dirty: boolean }
    | { kind: 'untitled'; id: string; content: string; dirty: boolean };

type UnsavedBufferSnapshot = {
    kind: 'file' | 'untitled';
    id: string;
    relPath?: string;
    content: string;
};

type ScopeView = {
    key: string;
    lineNumber: number;
    file: string;
};

const scopeKeyFromSubscription = (subscription: ScopeItem) => {
    if (subscription.type === 'ModuleOutput') {
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
    const maxAbsAmplitude = 10;
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

    const windowSize = 256;
    const startIndex = 0;
    const sampleCount = Math.min(windowSize, data.length);

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

const readUnsavedBuffers = (): EditorBuffer[] => {
    if (typeof window === 'undefined') {
        return [];
    }

    try {
        const raw = window.localStorage.getItem(UNSAVED_STORAGE_KEY);
        if (!raw) return [];

        const parsed = JSON.parse(raw) as UnsavedBufferSnapshot[];
        return parsed.map((snapshot): EditorBuffer => {
            if (snapshot.kind === 'file' && snapshot.relPath) {
                return {
                    kind: 'file',
                    relPath: snapshot.relPath,
                    content: snapshot.content,
                    dirty: true,
                };
            }
            return {
                kind: 'untitled',
                id: snapshot.id,
                content: snapshot.content,
                dirty: true,
            };
        });
    } catch (error) {
        console.error('Failed to read unsaved buffers:', error);
        return [];
    }
};

const saveUnsavedBuffers = (buffers: EditorBuffer[]) => {
    if (typeof window === 'undefined') return;

    try {
        const dirtyBuffers = buffers.filter((b) => b.dirty);
        const snapshots: UnsavedBufferSnapshot[] = dirtyBuffers.map(
            (buffer) => {
                if (buffer.kind === 'file') {
                    return {
                        kind: 'file',
                        id: buffer.relPath,
                        relPath: buffer.relPath,
                        content: buffer.content,
                    };
                }
                return {
                    kind: 'untitled',
                    id: buffer.id,
                    content: buffer.content,
                };
            },
        );

        window.localStorage.setItem(
            UNSAVED_STORAGE_KEY,
            JSON.stringify(snapshots),
        );
    } catch (error) {
        console.error('Failed to save unsaved buffers:', error);
    }
};

const getBufferId = (buffer: EditorBuffer): string => {
    return buffer.kind === 'file' ? buffer.relPath : buffer.id;
};

function App() {
    // Editor buffers: file-backed + untitled
    const [buffers, setBuffers] = useState<EditorBuffer[]>(() => {
        const saved = readUnsavedBuffers();
        // Always start with one untitled buffer if none exist
        if (saved.length === 0) {
            return [
                {
                    kind: 'untitled',
                    id: 'untitled-1',
                    content: DEFAULT_PATCH,
                    dirty: false,
                },
            ];
        }
        return saved;
    });

    const [activeBufferId, setActiveBufferId] = useState<string>(() => {
        const saved = readUnsavedBuffers();
        return saved.length > 0 ? getBufferId(saved[0]) : 'untitled-1';
    });

    const activeBuffer = buffers.find((b) => getBufferId(b) === activeBufferId);
    const patchCode = activeBuffer?.content ?? DEFAULT_PATCH;

    // Workspace & filesystem
    const [workspaceRoot, setWorkspaceRoot] = useState<string | null>(null);
    const [fileTree, setFileTree] = useState<FileTreeEntry[]>([]);

    // Audio state
    const [isClockRunning, setIsClockRunning] = useState(true);
    const [isRecording, setIsRecording] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [validationErrors, setValidationErrors] = useState<
        ValidationError[] | null
    >(null);
    const [schemas, setSchemas] = useState<ModuleSchema[]>([]);
    const [scopeViews, setScopeViews] = useState<ScopeView[]>([]);
    const [runningBufferId, setRunningBufferId] = useState<string | null>(null);

    const editorRef = useRef<editor.IStandaloneCodeEditor>(null);
    const scopeCanvasMapRef = useRef<Map<string, HTMLCanvasElement>>(new Map());

    // Load workspace and file tree on mount
    useEffect(() => {
        electronAPI.filesystem.getWorkspace().then((workspace) => {
            if (workspace) {
                setWorkspaceRoot(workspace.path);
                refreshFileTree();
            }
        });
    }, []);

    const refreshFileTree = useCallback(async () => {
        const tree = await electronAPI.filesystem.listFiles();
        setFileTree(tree);
    }, []);

    const selectWorkspaceFolder = useCallback(async () => {
        // Check for dirty file-backed buffers before switching
        const dirtyFileBuffers = buffers.filter(
            (b) => b.kind === 'file' && b.dirty,
        );

        if (dirtyFileBuffers.length > 0) {
            const fileList = dirtyFileBuffers
                .map((b) => (b.kind === 'file' ? b.relPath : ''))
                .filter(Boolean)
                .join(', ');

            const shouldSave = window.confirm(
                `You have unsaved changes in: ${fileList}. Save changes before switching workspace?`,
            );

            if (shouldSave) {
                // Save all dirty file buffers
                for (const buffer of dirtyFileBuffers) {
                    if (buffer.kind === 'file') {
                        await electronAPI.filesystem.writeFile(
                            buffer.relPath,
                            buffer.content,
                        );
                    }
                }
                // Mark them clean
                setBuffers((prev) =>
                    prev.map((b) =>
                        b.kind === 'file' && b.dirty
                            ? { ...b, dirty: false }
                            : b,
                    ),
                );
            } else {
                // Discard changes: remove dirty file buffers from the list
                setBuffers((prev) =>
                    prev.filter((b) => !(b.kind === 'file' && b.dirty)),
                );
            }
        }

        const workspace = await electronAPI.filesystem.selectWorkspace();
        if (workspace) {
            setWorkspaceRoot(workspace.path);
            await refreshFileTree();
        }
    }, [buffers, refreshFileTree]);

    // Save dirty buffers to localStorage
    useEffect(() => {
        saveUnsavedBuffers(buffers);
    }, [buffers]);

    const registerScopeCanvas = useCallback(
        (key: string, canvas: HTMLCanvasElement) => {
            scopeCanvasMapRef.current.set(key, canvas);
        },
        [],
    );

    const unregisterScopeCanvas = useCallback((key: string) => {
        scopeCanvasMapRef.current.delete(key);
    }, []);

    useEffect(() => {
        electronAPI.getSchemas().then((fetchedSchemas) => {
            setSchemas(fetchedSchemas);
        });
    }, []);

    const schemaRef = useRef<ModuleSchema[]>([]);
    useEffect(() => {
        schemaRef.current = schemas;
    }, [schemas]);

    const patchCodeRef = useRef<string>(patchCode);
    useEffect(() => {
        patchCodeRef.current = patchCode;
    }, [patchCode]);

    const isClockRunningRef = useRef(isClockRunning);
    useEffect(() => {
        isClockRunningRef.current = isClockRunning;
    }, [isClockRunning]);

    useEffect(() => {
        if (isClockRunningRef.current) {
            const tick = () => {
                electronAPI.synthesizer.getScopes().then((scopes) => {
                    for (const [scopeItem, samples] of scopes) {
                        const scopeKey = scopeKeyFromSubscription(scopeItem);
                        const scopedCanvas =
                            scopeCanvasMapRef.current.get(scopeKey);
                        if (scopedCanvas) {
                            drawOscilloscope(samples, scopedCanvas);
                        }
                    }
                    if (isClockRunningRef.current) {
                        requestAnimationFrame(tick);
                    }
                });
            };
            requestAnimationFrame(tick);
        }
    }, [isClockRunning]);

    const formatFileLabel = useCallback((buffer: EditorBuffer) => {
        if (buffer.kind === 'untitled') {
            return `Untitled-${buffer.id}`;
        }
        return buffer.relPath;
    }, []);

    const normalizeFileName = useCallback((name: string) => {
        const trimmed = name.trim();
        if (!trimmed) {
            return trimmed;
        }
        return trimmed.endsWith('.js') || trimmed.endsWith('.mjs')
            ? trimmed
            : `${trimmed}.mjs`;
    }, []);

    const handlePatchChange = useCallback(
        (value: string) => {
            setBuffers((prev) =>
                prev.map((b) =>
                    getBufferId(b) === activeBufferId
                        ? { ...b, content: value, dirty: true }
                        : b,
                ),
            );
        },
        [activeBufferId],
    );

    const openFile = useCallback(
        async (relPath: string) => {
            // Check if already open
            const existing = buffers.find(
                (b) => b.kind === 'file' && b.relPath === relPath,
            );
            if (existing) {
                setActiveBufferId(getBufferId(existing));
                return;
            }

            // Load from filesystem
            try {
                const content = await electronAPI.filesystem.readFile(relPath);
                const newBuffer: EditorBuffer = {
                    kind: 'file',
                    relPath,
                    content,
                    dirty: false,
                };
                setBuffers((prev) => [...prev, newBuffer]);
                setActiveBufferId(getBufferId(newBuffer));
            } catch (error) {
                setError(`Failed to open file: ${relPath}`);
            }
        },
        [buffers],
    );

    const createUntitledFile = useCallback(() => {
        const nextId = `untitled-${Date.now()}`;
        const newBuffer: EditorBuffer = {
            kind: 'untitled',
            id: nextId,
            content: DEFAULT_PATCH,
            dirty: false,
        };
        setBuffers((prev) => [...prev, newBuffer]);
        setActiveBufferId(nextId);
    }, []);

    const saveFile = useCallback(async () => {
        const buffer = buffers.find((b) => getBufferId(b) === activeBufferId);
        if (!buffer) return;

        if (buffer.kind === 'untitled') {
            // Save as...
            const input =
                await electronAPI.filesystem.showSaveDialog('untitled.mjs');
            if (!input) return;

            const normalized = normalizeFileName(input);
            if (!normalized) return;

            const result = await electronAPI.filesystem.writeFile(
                normalized,
                buffer.content,
            );

            if (result.success) {
                // Replace untitled buffer with file buffer
                setBuffers((prev) =>
                    prev.map((b) =>
                        getBufferId(b) === activeBufferId
                            ? {
                                  kind: 'file' as const,
                                  relPath: normalized,
                                  content: buffer.content,
                                  dirty: false,
                              }
                            : b,
                    ),
                );
                setActiveBufferId(normalized);
                await refreshFileTree();
            } else {
                setError(result.error || 'Failed to save file');
            }
        } else {
            // Save existing file
            const result = await electronAPI.filesystem.writeFile(
                buffer.relPath,
                buffer.content,
            );

            if (result.success) {
                setBuffers((prev) =>
                    prev.map((b) =>
                        getBufferId(b) === activeBufferId
                            ? { ...b, dirty: false }
                            : b,
                    ),
                );
            } else {
                setError(result.error || 'Failed to save file');
            }
        }
    }, [activeBufferId, buffers, normalizeFileName, refreshFileTree]);

    const renameFile = useCallback(async () => {
        const buffer = buffers.find((b) => getBufferId(b) === activeBufferId);
        if (!buffer || buffer.kind !== 'file') return;

        const input = await electronAPI.filesystem.showInputDialog(
            'Rename file',
            buffer.relPath,
        );
        if (!input) return;

        const normalized = normalizeFileName(input);
        if (!normalized || normalized === buffer.relPath) return;

        const result = await electronAPI.filesystem.renameFile(
            buffer.relPath,
            normalized,
        );

        if (result.success) {
            setBuffers((prev) =>
                prev.map((b) =>
                    getBufferId(b) === activeBufferId
                        ? { ...b, relPath: normalized }
                        : b,
                ),
            );
            setActiveBufferId(normalized);
            await refreshFileTree();
        } else {
            setError(result.error || 'Failed to rename file');
        }
    }, [activeBufferId, buffers, normalizeFileName, refreshFileTree]);

    const deleteFile = useCallback(async () => {
        const buffer = buffers.find((b) => getBufferId(b) === activeBufferId);
        if (!buffer || buffer.kind !== 'file') return;

        if (!window.confirm(`Delete ${buffer.relPath}?`)) return;

        const result = await electronAPI.filesystem.deleteFile(buffer.relPath);

        if (result.success) {
            // Remove buffer and switch to another
            setBuffers((prev) => {
                const filtered = prev.filter(
                    (b) => getBufferId(b) !== activeBufferId,
                );
                if (filtered.length === 0) {
                    // Create a new untitled buffer if this was the last one
                    return [
                        {
                            kind: 'untitled',
                            id: `untitled-${Date.now()}`,
                            content: DEFAULT_PATCH,
                            dirty: false,
                        },
                    ];
                }
                return filtered;
            });
            setActiveBufferId(getBufferId(buffers[0]));
            await refreshFileTree();
        } else {
            setError(result.error || 'Failed to delete file');
        }
    }, [activeBufferId, buffers, refreshFileTree]);

    const closeBuffer = useCallback(
        (bufferId: string) => {
            const buffer = buffers.find((b) => getBufferId(b) === bufferId);
            if (!buffer) return;

            if (buffer.dirty) {
                const shouldSave = window.confirm(
                    `${formatFileLabel(buffer)} has unsaved changes. Save before closing?`,
                );
                if (shouldSave) {
                    // TODO: Save before closing
                    return;
                }
            }

            setBuffers((prev) => {
                const filtered = prev.filter(
                    (b) => getBufferId(b) !== bufferId,
                );
                if (filtered.length === 0) {
                    // Always keep at least one untitled buffer
                    return [
                        {
                            kind: 'untitled',
                            id: `untitled-${Date.now()}`,
                            content: DEFAULT_PATCH,
                            dirty: false,
                        },
                    ];
                }
                return filtered;
            });

            if (activeBufferId === bufferId) {
                const remaining = buffers.filter(
                    (b) => getBufferId(b) !== bufferId,
                );
                if (remaining.length > 0) {
                    setActiveBufferId(getBufferId(remaining[0]));
                }
            }
        },
        [activeBufferId, buffers, formatFileLabel],
    );

    const handleSaveFileRef = useRef(() => {});
    useEffect(() => {
        handleSaveFileRef.current = saveFile;
    }, [saveFile]);

    const handleSubmitRef = useRef(() => {});
    useEffect(() => {
        handleSubmitRef.current = async () => {
            try {
                const schemasValue = schemaRef.current;
                const patchCodeValue = patchCodeRef.current;
                const patch = executePatchScript(patchCodeValue, schemasValue);
                patch.moduleIdRemaps = [];
                const { errors, appliedPatch } =
                    await electronAPI.synthesizer.updatePatch(
                        patch,
                        activeBufferId,
                    );
                if (errors.length > 0) {
                    setValidationErrors(errors.flatMap((e) => e.errors || []));
                    setError(
                        errors.map((e) => e.message).join('\n') ||
                            'Failed to apply patch.',
                    );
                    return;
                }
                setIsClockRunning(true);
                setRunningBufferId(activeBufferId);
                setError(null);
                setValidationErrors(null);

                const scopeCalls = findScopeCallEndLines(patchCodeValue);
                const views: ScopeView[] = patch.scopes
                    .map((scope, idx) => {
                        const call = scopeCalls[idx];
                        if (!call) return null;
                        if (scope.item.type === 'ModuleOutput') {
                            const { moduleId, portName } = scope.item;
                            return {
                                key: `:module:${moduleId}:${portName}`,
                                lineNumber: call.endLine,
                                file: activeBufferId,
                            };
                        }
                        const { trackId } = scope.item;
                        return {
                            key: `:track:${trackId}`,
                            lineNumber: call.endLine,
                            file: activeBufferId,
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
    }, [activeBufferId]);

    const handleStopRef = useRef(() => {});
    useEffect(() => {
        handleStopRef.current = async () => {
            await electronAPI.synthesizer.stop();
            setIsClockRunning(false);
        };
    }, []);

    const dismissError = useCallback(() => {
        setError(null);
        setValidationErrors(null);
    }, []);

    useEffect(() => {
        const handleKeyDown = async (e: KeyboardEvent) => {
            if ((e.metaKey || e.ctrlKey) && (e.key === 's' || e.key === 'S')) {
                e.preventDefault();
                handleSaveFileRef.current();
                return;
            }

            if ((e.ctrlKey || e.altKey) && (e.key === 'r' || e.key === 'R')) {
                if (e.altKey) {
                    e.preventDefault();
                }
                if (isRecording) {
                    await electronAPI.synthesizer.stopRecording();
                    setIsRecording(false);
                } else {
                    await electronAPI.synthesizer.startRecording();
                    setIsRecording(true);
                }
            }
        };

        window.addEventListener('keydown', handleKeyDown, { capture: true });
        return () =>
            window.removeEventListener('keydown', handleKeyDown, {
                capture: true,
            });
    }, [isRecording]);

    return (
        <SchemasContext.Provider value={schemas}>
            <div className="app">
                <header className="app-header">
                    <h1></h1>
                    <AudioControls
                        isRunning={isClockRunning}
                        isRecording={isRecording}
                        onStop={handleStopRef.current}
                        onStartRecording={async () => {
                            await electronAPI.synthesizer.startRecording();
                            setIsRecording(true);
                        }}
                        onStopRecording={async () => {
                            await electronAPI.synthesizer.stopRecording();
                            setIsRecording(false);
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
                            currentFile={formatFileLabel(activeBuffer!)}
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
                        workspaceRoot={workspaceRoot}
                        fileTree={fileTree}
                        buffers={buffers}
                        activeBufferId={activeBufferId}
                        runningBufferId={runningBufferId}
                        formatLabel={formatFileLabel}
                        onSelectBuffer={setActiveBufferId}
                        onOpenFile={openFile}
                        onCreateFile={createUntitledFile}
                        onSaveFile={handleSaveFileRef.current}
                        onRenameFile={renameFile}
                        onDeleteFile={deleteFile}
                        onCloseBuffer={closeBuffer}
                        onSelectWorkspace={selectWorkspaceFolder}
                        onRefreshTree={refreshFileTree}
                    />
                </main>
            </div>
        </SchemasContext.Provider>
    );
}

export default App;
