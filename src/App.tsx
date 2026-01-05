import { useCallback, useEffect, useRef, useState } from 'react';
import { MonacoPatchEditor as PatchEditor } from './components/MonacoPatchEditor';
import { AudioControls } from './components/AudioControls';
import { ErrorDisplay } from './components/ErrorDisplay';
import { executePatchScript } from './dsl';
import { useSchemas } from './SchemaContext';
import './App.css';
import type { editor } from 'monaco-editor';
import { findScopeCallEndLines } from './utils/findScopeCallEndLines';
import { FileExplorer, SCRATCH_FILE } from './components/FileExplorer';
import electronAPI from './electronAPI';
import { ModuleSchema, ScopeItem, ValidationError } from '@modular/core';
import type { FileTreeEntry } from './ipcTypes';
import { v4 } from 'uuid';

const DEFAULT_PATCH = `// Simple 440 Hz sine wave
const osc = sine(note('A4'));
out.source(osc);
`;

const UNSAVED_STORAGE_KEY = 'modular_unsaved_buffers';

// New buffer model: distinguish between file-backed and untitled buffers
export type EditorBuffer =
    | {
          kind: 'file';
          id: string;
          filePath: string;
          content: string;
          dirty: boolean;
      }
    | { kind: 'untitled'; id: string; content: string; dirty: boolean };

type UnsavedBufferSnapshot =
    | {
          kind: 'file';
          id: string;
          filePath: string;
          content: string;
      }
    | {
          kind: 'untitled';
          id: string;
          content: string;
      };

type ScopeView = {
    key: string;
    lineNumber: number;
    file: string;
};

const scopeKeyFromSubscription = (subscription: ScopeItem) => {
    const { moduleId, portName } = subscription;
    return `:module:${moduleId}:${portName}`;
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

    const windowSize = 1024;
    const startIndex = 0;
    const sampleCount = Math.min(windowSize, data.length);

    if (sampleCount < 2) {
        return;
    }

    ctx.strokeStyle = '#ffffff';
    ctx.lineWidth = 2;
    ctx.beginPath();

    const stepX = w / (windowSize - 1);

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
            if (snapshot.kind === 'file') {
                return {
                    kind: 'file',
                    filePath: snapshot.filePath,
                    content: snapshot.content,
                    id: snapshot.id,
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
                        id: buffer.filePath,
                        filePath: buffer.filePath,
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
    return buffer.kind === 'file' ? buffer.filePath : buffer.id;
};

function App() {
    // Editor buffers: file-backed + untitled
    const [buffers, setBuffers] = useState<EditorBuffer[]>(() => {
        const saved = readUnsavedBuffers();

        return saved;
    });

    const [activeBufferId, setActiveBufferId] = useState<string | undefined>(
        () => {
            const saved = readUnsavedBuffers();
            return saved.length > 0 ? getBufferId(saved[0]) : undefined;
        },
    );

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
    const { schemas: schemasMap } = useSchemas();
    const schemas = Object.values(schemasMap);
    const [scopeViews, setScopeViews] = useState<ScopeView[]>([]);
    const [runningBufferId, setRunningBufferId] = useState<string | null>(null);

    const editorRef = useRef<editor.IStandaloneCodeEditor>(null);
    const scopeCanvasMapRef = useRef<Map<string, HTMLCanvasElement>>(new Map());

    // Load workspace and file tree on mount
    useEffect(() => {
        electronAPI.filesystem
            .getWorkspace()
            .then((workspace) => {
                if (workspace) {
                    setWorkspaceRoot(workspace.path);
                    refreshFileTree();
                }
            })
            .catch((err) => {
                console.error('Failed to load workspace:', err);
            });
    }, []);

    const refreshFileTree = useCallback(async () => {
        try {
            const tree = await electronAPI.filesystem.listFiles();
            setFileTree(tree);
        } catch (error) {
            console.error('Failed to refresh file tree:', error);
        }
    }, []);

    const selectWorkspaceFolder = useCallback(async () => {
        // Check for dirty file-backed buffers before switching
        const dirtyFileBuffers = buffers.filter(
            (b) => b.kind === 'file' && b.dirty,
        );

        if (dirtyFileBuffers.length > 0) {
            const fileList = dirtyFileBuffers
                .map((b) => (b.kind === 'file' ? b.filePath : ''))
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
                            buffer.filePath,
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
                electronAPI.synthesizer
                    .getScopes()
                    .then((scopes) => {
                        for (const [scopeItem, samples] of scopes) {
                            const scopeKey =
                                scopeKeyFromSubscription(scopeItem);
                            const scopedCanvas =
                                scopeCanvasMapRef.current.get(scopeKey);
                            if (scopedCanvas) {
                                drawOscilloscope(samples, scopedCanvas);
                            }
                        }
                        if (isClockRunningRef.current) {
                            requestAnimationFrame(tick);
                        }
                    })
                    .catch((err) => {
                        console.error('Failed to get scopes:', err);
                        // Continue loop even if one frame fails, or stop?
                        // For now, let's try to continue but maybe with a delay or just next frame
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
        return buffer.filePath;
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
            if (!workspaceRoot) {
                setError('No workspace open');
                return;
            }

            // Construct absolute path
            const absPath = `${workspaceRoot}/${relPath}`;

            // Check if already open
            const existing = buffers.find(
                (b) => b.kind === 'file' && b.filePath === absPath,
            );
            if (existing) {
                setActiveBufferId(getBufferId(existing));
                return;
            }

            // Load from filesystem
            try {
                const content = await electronAPI.filesystem.readFile(absPath);
                const newBuffer: EditorBuffer = {
                    kind: 'file',
                    filePath: absPath,
                    content,
                    id: v4(),
                    dirty: false,
                };
                setBuffers((prev) => [...prev, newBuffer]);
                setActiveBufferId(getBufferId(newBuffer));
            } catch (error) {
                setError(`Failed to open file: ${relPath}`);
            }
        },
        [buffers, workspaceRoot],
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
                                  filePath: normalized,
                                  id: b.id,
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
                buffer.filePath,
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

        const currentFileName = buffer.filePath.split(/[/\\]/).pop();

        const input = await electronAPI.filesystem.showInputDialog(
            'Rename file',
            currentFileName || buffer.filePath,
        );
        if (!input) return;

        const normalized = normalizeFileName(input);

        // Construct new path in same directory
        const separator = buffer.filePath.includes('\\') ? '\\' : '/';
        const lastSepIndex = buffer.filePath.lastIndexOf(separator);
        let newPath = normalized;
        if (lastSepIndex !== -1) {
            const dir = buffer.filePath.substring(0, lastSepIndex);
            newPath = `${dir}${separator}${normalized}`;
        }

        if (!newPath || newPath === buffer.filePath) return;

        const result = await electronAPI.filesystem.renameFile(
            buffer.filePath,
            newPath,
        );

        if (result.success) {
            setBuffers((prev) =>
                prev.map((b) =>
                    getBufferId(b) === activeBufferId
                        ? { ...b, filePath: newPath }
                        : b,
                ),
            );
            setActiveBufferId(newPath);
            await refreshFileTree();
        } else {
            setError(result.error || 'Failed to rename file');
        }
    }, [activeBufferId, buffers, normalizeFileName, refreshFileTree]);

    const deleteFile = useCallback(async () => {
        const buffer = buffers.find((b) => getBufferId(b) === activeBufferId);
        if (!buffer || buffer.kind !== 'file') return;

        if (!window.confirm(`Delete ${buffer.filePath}?`)) return;

        const result = await electronAPI.filesystem.deleteFile(buffer.filePath);

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
                            id: v4(),
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

    const handleOpenWorkspaceRef = useRef(() => {});
    useEffect(() => {
        handleOpenWorkspaceRef.current = selectWorkspaceFolder;
    }, [selectWorkspaceFolder]);

    const [lastSubmittedCode, setLastSubmittedCode] = useState<string | null>(
        null,
    );

    const handleSubmitRef = useRef(() => {});
    useEffect(() => {
        handleSubmitRef.current = async () => {
            if (!activeBufferId) return;
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
                setLastSubmittedCode(patchCodeValue);
                setError(null);
                setValidationErrors(null);

                const scopeCalls = findScopeCallEndLines(patchCodeValue);
                console.log('Found scope calls:', scopeCalls);
                console.log('Patch scopes:', patch.scopes);
                const views: ScopeView[] = patch.scopes
                    .map((scope, idx) => {
                        const call = scopeCalls[idx];
                        if (!call) return null;
                        const { moduleId, portName } = scope.item;
                        return {
                            key: `:module:${moduleId}:${portName}`,
                            lineNumber: call.endLine,
                            file: activeBufferId,
                        };
                    })
                    .filter((v): v is ScopeView => v !== null);
                console.log('Scope views:', views);

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
        const cleanupSave = electronAPI.onMenuSave(() => {
            handleSaveFileRef.current();
        });
        const cleanupStop = electronAPI.onMenuStop(() => {
            handleStopRef.current();
        });
        const cleanupUpdate = electronAPI.onMenuUpdatePatch(() => {
            handleSubmitRef.current();
        });
        const cleanupOpenWorkspace = electronAPI.onMenuOpenWorkspace(() => {
            handleOpenWorkspaceRef.current();
        });

        return () => {
            cleanupSave();
            cleanupStop();
            cleanupUpdate();
            cleanupOpenWorkspace();
        };
    }, []);

    useEffect(() => {
        const handleKeyDown = async (e: KeyboardEvent) => {
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
                    {!workspaceRoot ? (
                        <div className="empty-state">
                            <button
                                className="open-folder-button"
                                onClick={selectWorkspaceFolder}
                            >
                                Open Folder
                            </button>
                        </div>
                    ) : (
                        <>
                            <div className="editor-panel">
                                <PatchEditor
                                    value={patchCode}
                                    lastSubmittedCode={lastSubmittedCode}
                                    currentFile={activeBufferId}
                                    onChange={handlePatchChange}
                                    onSubmit={handleSubmitRef}
                                    onStop={handleStopRef}
                                    onSave={handleSaveFileRef}
                                    editorRef={editorRef}
                                    schemas={schemas}
                                    scopeViews={scopeViews}
                                    onRegisterScopeCanvas={registerScopeCanvas}
                                    onUnregisterScopeCanvas={
                                        unregisterScopeCanvas
                                    }
                                />
                            </div>

                            <FileExplorer
                                workspaceRoot={workspaceRoot}
                                fileTree={fileTree}
                                buffers={buffers}
                                activeBufferId={activeBufferId}
                                runningBufferId={runningBufferId}
                                formatLabel={(buffer) => {
                                    const path = formatFileLabel(buffer);
                                    const parts = path.split(/[/\\]/);
                                    return parts[parts.length - 1];
                                }}
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
                        </>
                    )}
                </main>
            </div>
    );
}

export default App;
