import { useCallback, useEffect, useRef, useState } from 'react';
import { MonacoPatchEditor as PatchEditor } from './components/MonacoPatchEditor';
import { AudioControls } from './components/AudioControls';
import { ErrorDisplay } from './components/ErrorDisplay';
import { executePatchScript } from './dsl';
import { useSchemas } from './SchemaContext';
import './App.css';
import type { editor } from 'monaco-editor';
import { findScopeCallEndLines } from './utils/findScopeCallEndLines';
import { FileExplorer } from './components/FileExplorer';
import electronAPI from './electronAPI';
import { ModuleSchema, ScopeItem, ValidationError } from '@modular/core';
import type { FileTreeEntry } from './ipcTypes';
import { v4 } from 'uuid';

const DEFAULT_PATCH = `// Simple 440 Hz sine wave
const osc = sine('a4');
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
          isPreview?: boolean;
      }
    | { kind: 'untitled'; id: string; content: string; dirty: boolean; isPreview?: boolean };

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

    // Get theme colors from CSS variables
    const styles = getComputedStyle(document.documentElement);
    const bgColor = styles.getPropertyValue('--bg-primary').trim() || '#0a0a0a';
    const borderColor = styles.getPropertyValue('--border-subtle').trim() || '#222222';
    const mutedColor = styles.getPropertyValue('--text-muted').trim() || '#555555';
    const accentColor = styles.getPropertyValue('--accent-primary').trim() || '#4ec9b0';

    ctx.fillStyle = bgColor;
    ctx.fillRect(0, 0, w, h);

    const midY = h / 2;
    const maxAbsAmplitude = 10;
    const pixelsPerUnit = h / 2 / maxAbsAmplitude;

    // Subtle grid line
    ctx.strokeStyle = borderColor;
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, midY);
    ctx.lineTo(w, midY);
    ctx.stroke();

    if (!data || data.length === 0) {
        ctx.fillStyle = mutedColor;
        ctx.font = '13px "Fira Code", monospace';
        ctx.textAlign = 'center';
        ctx.fillText('~', w / 2, midY);
        return;
    }

    const windowSize = 1024;
    const startIndex = 0;
    const sampleCount = Math.min(windowSize, data.length);

    if (sampleCount < 2) {
        return;
    }

    // Accent color for waveform
    ctx.strokeStyle = accentColor;
    ctx.lineWidth = 1.5;
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

    // Initialize used untitled numbers from initial buffers
    useEffect(() => {
        const used = new Set<number>();
        buffers.forEach(b => {
             if (b.kind === 'untitled') {
                 // Format is "untitled-N"
                 const match = b.id.match(/^untitled-(\d+)$/);
                 if (match) {
                     used.add(parseInt(match[1], 10));
                 }
             }
        });
        setUsedUntitledNumbers(used);
    // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []); 

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

    // Track used untitled numbers
    const [usedUntitledNumbers, setUsedUntitledNumbers] = useState<Set<number>>(new Set());

    const { schemas: schemasMap } = useSchemas();
    const [scopeViews, setScopeViews] = useState<ScopeView[]>([]);
    const [runningBufferId, setRunningBufferId] = useState<string | null>(null);

    const editorRef = useRef<editor.IStandaloneCodeEditor>(null);
    const scopeCanvasMapRef = useRef<Map<string, HTMLCanvasElement>>(new Map());

    // Suppress harmless Monaco Editor cancellation errors when switching files
    useEffect(() => {
        const handleError = (event: ErrorEvent) => {
            // Check if this is the Monaco WordHighlighter cancellation error
            if (
                event.error?.message === 'Canceled' &&
                event.error?.stack?.includes('WordHighlighter')
            ) {
                // Suppress this error - it's harmless and happens during normal file switching
                event.preventDefault();
                return;
            }
        };

        const handleRejection = (event: PromiseRejectionEvent) => {
            // Also catch if it appears as an unhandled promise rejection
            const reason = event.reason;
            if (
                reason?.message === 'Canceled' &&
                (reason?.stack?.includes('WordHighlighter') ||
                    reason?.stack?.includes('Delayer'))
            ) {
                event.preventDefault();
                return;
            }
        };

        window.addEventListener('error', handleError);
        window.addEventListener('unhandledrejection', handleRejection);
        return () => {
            window.removeEventListener('error', handleError);
            window.removeEventListener('unhandledrejection', handleRejection);
        };
    }, []);

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
        schemaRef.current = Object.values(schemasMap);
    }, [schemasMap]);

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
            return buffer.id; // id is "untitled-N" now
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
                        ? { ...b, content: value, dirty: true, isPreview: false }
                        : b,
                ),
            );
        },
        [activeBufferId],
    );

    const openFile = useCallback(
        async (relPath: string, options?: { preview?: boolean }) => {
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
                // If opening explicitly (double click or non-preview), clear preview status
                if (options?.preview === false && existing.isPreview) {
                    setBuffers(prev => prev.map(b => 
                        getBufferId(b) === getBufferId(existing) ? { ...b, isPreview: false } : b
                    ));
                }
                setActiveBufferId(getBufferId(existing));
                return;
            }

            // Load from filesystem
            try {
                const content = await electronAPI.filesystem.readFile(absPath);
                
                 setBuffers((prev) => {
                    let nextBuffers = [...prev];
                    const existingPreviewIndex = nextBuffers.findIndex(b => b.isPreview);
                    
                    if (options?.preview && existingPreviewIndex !== -1) {
                        const previewBuffer = nextBuffers[existingPreviewIndex];
                        if (!previewBuffer.dirty) {
                             nextBuffers.splice(existingPreviewIndex, 1);
                        }
                    }

                    const newBuffer: EditorBuffer = {
                        kind: 'file',
                        filePath: absPath,
                        content,
                        id: v4(),
                        dirty: false,
                        isPreview: options?.preview ?? false
                    };
                    return [...nextBuffers, newBuffer];
                });
                setActiveBufferId(absPath); // For files, ID is filePath
            } catch (error) {
                setError(`Failed to open file: ${relPath}`);
            }
        },
        [buffers, workspaceRoot],
    );

    const createUntitledFile = useCallback(() => {
        let nextIdNum = 1;
        while (usedUntitledNumbers.has(nextIdNum)) {
            nextIdNum++;
        }
        
        const nextId = `untitled-${nextIdNum}`;
        const newBuffer: EditorBuffer = {
            kind: 'untitled',
            id: nextId,
            content: DEFAULT_PATCH,
            dirty: false,
        };
        
        setUsedUntitledNumbers(prev => {
            const next = new Set(prev);
            next.add(nextIdNum);
            return next;
        });

        setBuffers((prev) => [...prev, newBuffer]);
        setActiveBufferId(nextId);
    }, [usedUntitledNumbers]);

    const saveFile = useCallback(async (targetId?: string) => {
        const idToSave = targetId || activeBufferId;
        const buffer = buffers.find((b) => getBufferId(b) === idToSave);
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
                // If it was untitled, release the number
                const match = buffer.id.match(/^untitled-(\d+)$/);
                 if (match) {
                     const num = parseInt(match[1], 10);
                     setUsedUntitledNumbers(prev => {
                         const next = new Set(prev);
                         next.delete(num);
                         return next;
                     });
                 }

                // Replace untitled buffer with file buffer
                setBuffers((prev) =>
                    prev.map((b) =>
                        getBufferId(b) === idToSave
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
                // Only change active buffer if we just saved the active one
                if (idToSave === activeBufferId) {
                    setActiveBufferId(normalized);
                }
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
                        getBufferId(b) === idToSave
                            ? { ...b, dirty: false }
                            : b,
                    ),
                );
            } else {
                setError(result.error || 'Failed to save file');
            }
        }
    }, [activeBufferId, buffers, normalizeFileName, refreshFileTree]);

    const [renamingPath, setRenamingPath] = useState<string | null>(null);

    const renameFile = useCallback(async (targetIdOrPath?: string) => {
        // targetIdOrPath can be a buffer ID or a file path
        let filePath: string | undefined;

        // Try to resolve absolute path if it's relative
        let resolvedPath = targetIdOrPath;
        if (targetIdOrPath && workspaceRoot && !targetIdOrPath.startsWith('/') && !targetIdOrPath.match(/^[a-zA-Z]:/)) {
            resolvedPath = `${workspaceRoot}/${targetIdOrPath}`;
        }

        // Try to find if it corresponds to an open buffer
        const buffer = buffers.find((b) => getBufferId(b) === targetIdOrPath) 
            || buffers.find((b) => b.kind === 'file' && b.filePath === resolvedPath);
        
        if (buffer && buffer.kind === 'file') {
            filePath = buffer.filePath;
        } else if (resolvedPath && typeof resolvedPath === 'string') {
            // Assume it's a file path if provided
             filePath = resolvedPath;
        } else if (activeBufferId) {
             const active = buffers.find(b => getBufferId(b) === activeBufferId);
             if (active && active.kind === 'file') {
                 filePath = active.filePath;
             }
        }

        if (!filePath) return;
        setRenamingPath(filePath);
    }, [activeBufferId, buffers, workspaceRoot]);

    const handleRenameCommit = useCallback(async (oldPath: string, newName: string) => {
        setRenamingPath(null);
        if (!newName) return;
        
        const currentFileName = oldPath.split(/[/\\]/).pop();
        if (newName === currentFileName) return;

        const normalized = normalizeFileName(newName);

        // Construct new path in same directory
        const separator = oldPath.includes('\\') ? '\\' : '/';
        const lastSepIndex = oldPath.lastIndexOf(separator);
        let newPath = normalized;
        if (lastSepIndex !== -1) {
            const dir = oldPath.substring(0, lastSepIndex);
            newPath = `${dir}${separator}${normalized}`;
        }

        if (!newPath || newPath === oldPath) return;

        const result = await electronAPI.filesystem.renameFile(
            oldPath,
            newPath,
        );

        if (result.success) {
            // Update buffer if it was open
            // Note: We should traverse all buffers to see if any match the old path
            setBuffers((prev) =>
                prev.map((b) =>
                    (b.kind === 'file' && b.filePath === oldPath)
                        ? { ...b, filePath: newPath }
                        : b,
                ),
            );
            
            // If the active buffer was the one renamed (via ID or path match)
            // Note: getBufferId might still refer to old path if we haven't updated state fully, 
            // but we are updating setBuffers above.
            // If active buffer is a file and matches oldPath, we should update ID tracking if we rely on it.
            // But we use activeBufferId === getBufferId(activeBuffer).
            // If getBufferId uses filePath, it changes.
            // If we renamed the active buffer...
            const wasActive = buffers.some(b => getBufferId(b) === activeBufferId && b.kind === 'file' && b.filePath === oldPath);
            if (wasActive) {
                setActiveBufferId(newPath); // new ID for file buffer is the file path
            }
            
            await refreshFileTree();
        } else {
            setError(result.error || 'Failed to rename file');
        }

    }, [activeBufferId, buffers, normalizeFileName, refreshFileTree]);

    const deleteFile = useCallback(async (targetIdOrPath?: string) => {
        let filePath: string | undefined;
        let bufferId: string | undefined;

        // Try to resolve absolute path if it's relative
        let resolvedPath = targetIdOrPath;
        if (targetIdOrPath && workspaceRoot && !targetIdOrPath.startsWith('/') && !targetIdOrPath.match(/^[a-zA-Z]:/)) {
            resolvedPath = `${workspaceRoot}/${targetIdOrPath}`;
        }

        // Try to find if it corresponds to an open buffer by absolute path or ID
        const buffer = buffers.find((b) => getBufferId(b) === targetIdOrPath) 
             || buffers.find((b) => b.kind === 'file' && b.filePath === resolvedPath);

        if (buffer && buffer.kind === 'file') {
            filePath = buffer.filePath;
            bufferId = getBufferId(buffer);
        } else if (resolvedPath && typeof resolvedPath === 'string') {
             filePath = resolvedPath;
        } else if (activeBufferId) {
             const active = buffers.find(b => getBufferId(b) === activeBufferId);
             if (active && active.kind === 'file') {
                 filePath = active.filePath;
                 bufferId = getBufferId(active);
             }
        }

        if (!filePath) return;

        if (!window.confirm(`Delete ${filePath}?`)) return;

        const result = await electronAPI.filesystem.deleteFile(filePath);

        if (result.success) {
            // Remove buffer if open
            // If the file was open, we need to close it.
            // Check all buffers
             setBuffers((prev) => {
                const filtered = prev.filter(
                    (b) => !(b.kind === 'file' && b.filePath === filePath),
                );
                return filtered;
            });

            // If the deleted file was active, switch.
            const activeIsDeleted = activeBufferId && (
                 (bufferId && activeBufferId === bufferId) ||
                 (buffers.find(b => getBufferId(b) === activeBufferId)?.kind === 'file' 
                  && (buffers.find(b => getBufferId(b) === activeBufferId) as any).filePath === filePath)
            );

            if (activeIsDeleted) {
                 // The active buffer is gone. We need to pick a new one.
                 // Since we don't have access to the *new* buffers list here immediately (setState is async),
                 // we estimate.
                 const remaining = buffers.filter(b => !(b.kind === 'file' && b.filePath === filePath));
                 if (remaining.length > 0) {
                     setActiveBufferId(getBufferId(remaining[0]));
                 } else {
                     setActiveBufferId(undefined);
                 }
            }
            
            await refreshFileTree();
        } else {
            setError(result.error || 'Failed to delete file');
        }
    }, [activeBufferId, buffers, refreshFileTree]);

    // Handle context menu commands
    useEffect(() => {
        return electronAPI.onContextMenuCommand((action) => {
            switch (action.command) {
                case 'save':
                    saveFile(action.bufferId);
                    break;
                case 'rename':
                    renameFile(action.path || action.bufferId);
                    break;
                case 'delete':
                    deleteFile(action.path || action.bufferId);
                    break;
            }
        });
    }, [saveFile, renameFile, deleteFile]);

    const closeBuffer = useCallback(
        async (bufferId: string) => {
            const buffer = buffers.find((b) => getBufferId(b) === bufferId);
            if (!buffer) return;

            if (buffer.dirty) {
                // Show native Electron dialog
                const response = await electronAPI.showUnsavedChangesDialog(
                    formatFileLabel(buffer)
                );

                // Handle the response: 0=Save, 1=Don't Save, 2=Cancel
                if (response === 2) {
                    // Cancel - do nothing
                    return;
                } else if (response === 0) {
                    // Save - save the file then close
                    try {
                        await saveFile(bufferId);
                        performCloseBuffer(bufferId);
                    } catch (error) {
                        console.error('Error saving file:', error);
                        // Still close buffer even if save fails, to avoid getting stuck
                        performCloseBuffer(bufferId);
                    }
                } else {
                    // Don't Save - close without saving
                    performCloseBuffer(bufferId);
                }
            } else {
                // If not dirty, close immediately
                performCloseBuffer(bufferId);
            }
        },
        [buffers, formatFileLabel, saveFile],
    );

    const performCloseBuffer = useCallback(
        (bufferId: string) => {
            const buffer = buffers.find((b) => getBufferId(b) === bufferId);
            if (!buffer) return;

            // Add a small delay to allow Monaco's cleanup to complete
            setTimeout(() => {
                setBuffers((prev) => {
                    const filtered = prev.filter(
                        (b) => getBufferId(b) !== bufferId,
                    );
                    return filtered;
                });
                
                // If it was untitled, release the number
                if (buffer.kind === 'untitled') {
                     const match = buffer.id.match(/^untitled-(\d+)$/);
                     if (match) {
                         const num = parseInt(match[1], 10);
                         setUsedUntitledNumbers(prev => {
                             const next = new Set(prev);
                             next.delete(num);
                             return next;
                         });
                     }
                }

                if (activeBufferId === bufferId) {
                    const remaining = buffers.filter(
                        (b) => getBufferId(b) !== bufferId,
                    );
                    if (remaining.length > 0) {
                        setActiveBufferId(getBufferId(remaining[0]));
                    } else {
                        setActiveBufferId(undefined);
                    }
                }
            }, 50); // 50ms delay to allow Monaco cleanup
        },
        [activeBufferId, buffers],
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
        // Add global error handler to suppress Monaco cancellation errors
        const handleError = (event: ErrorEvent) => {
            // Suppress Monaco-specific cancellation errors that occur during buffer closing
            if (event.error && event.error.message === 'Canceled') {
                event.preventDefault();
                event.stopPropagation();
                return false;
            }
        };

        const handleUnhandledRejection = (event: PromiseRejectionEvent) => {
            // Suppress Monaco-specific cancellation errors in promises
            if (event.reason && event.reason.message === 'Canceled') {
                event.preventDefault();
                return false;
            }
        };

        window.addEventListener('error', handleError);
        window.addEventListener('unhandledrejection', handleUnhandledRejection);

        return () => {
            window.removeEventListener('error', handleError);
            window.removeEventListener('unhandledrejection', handleUnhandledRejection);
        };
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
        const cleanupCloseBuffer = electronAPI.onMenuCloseBuffer(() => {
            if (activeBufferId) {
                closeBuffer(activeBufferId);
            }
        });
        const cleanupToggleRecording = electronAPI.onMenuToggleRecording(() => {
            if (isRecording) {
                electronAPI.synthesizer.stopRecording();
                setIsRecording(false);
            } else {
                electronAPI.synthesizer.startRecording();
                setIsRecording(true);
            }
        });

        // Handle opening settings from menu (Cmd+,)
        const cleanupOpenSettings = electronAPI.onMenuOpenSettings(async () => {
            try {
                const configPath = await electronAPI.config.getPath();
                const content = await electronAPI.filesystem.readFile(configPath);
                const existingBuffer = buffers.find(b => 
                    b.kind === 'file' && b.filePath === configPath
                );
                if (existingBuffer) {
                    setActiveBufferId(getBufferId(existingBuffer));
                } else {
                    const newBuffer: EditorBuffer = {
                        kind: 'file',
                        id: configPath,
                        filePath: configPath,
                        content,
                        dirty: false,
                        isPreview: false,
                    };
                    setBuffers(prev => [...prev, newBuffer]);
                    setActiveBufferId(configPath);
                }
            } catch (err) {
                console.error('Failed to open settings:', err);
            }
        });

        return () => {
            cleanupSave();
            cleanupStop();
            cleanupUpdate();
            cleanupOpenWorkspace();
            cleanupCloseBuffer();
            cleanupToggleRecording();
            cleanupOpenSettings();
        };
    }, [activeBufferId, closeBuffer, isRecording, buffers]);

    const keepBuffer = useCallback((bufferId: string) => {
        setBuffers((prev) =>
            prev.map((b) =>
                getBufferId(b) === bufferId ? { ...b, isPreview: false } : b,
            ),
        );
    }, []);

    return (
        <div className="app">
            <header className="app-header">
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
                                runningBufferId={runningBufferId}
                                currentFile={activeBufferId}
                                onChange={handlePatchChange}
                                editorRef={editorRef}
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
                            renamingPath={renamingPath}
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
                            onRenameCommit={handleRenameCommit}
                            onRenameCancel={() => setRenamingPath(null)}
                            onKeepBuffer={keepBuffer}
                        />
                    </>
                )}
            </main>
        </div>
    );
}

export default App;
