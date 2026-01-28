import { useCallback, useEffect, useRef, useState } from 'react';
import { MonacoPatchEditor as PatchEditor } from './components/MonacoPatchEditor';
import { AudioControls } from './components/AudioControls';
import { ErrorDisplay } from './components/ErrorDisplay';
import './App.css';
import type { editor } from 'monaco-editor';
import { findScopeCallEndLines } from './utils/findScopeCallEndLines';
import { getErrorMessage } from './utils/errorUtils';
import { FileExplorer } from './components/FileExplorer';
import electronAPI from './electronAPI';
import { ValidationError } from '@modular/core';
import type { FileTreeEntry } from './ipcTypes';
import type { EditorBuffer, ScopeView } from './types/editor';
import { getBufferId } from './app/buffers';
import { drawOscilloscope, scopeKeyFromSubscription } from './app/oscilloscope';
import { useEditorBuffers } from './app/hooks/useEditorBuffers';


function App() {
    // Workspace & filesystem
    const [workspaceRoot, setWorkspaceRoot] = useState<string | null>(null);
    const [fileTree, setFileTree] = useState<FileTreeEntry[]>([]);

    const refreshFileTree = useCallback(async () => {
        try {
            const tree = await electronAPI.filesystem.listFiles();
            setFileTree(tree);
        } catch (error) {
            console.error('Failed to refresh file tree:', error);
        }
    }, []);

    const {
        buffers,
        setBuffers,
        activeBufferId,
        setActiveBufferId,
        patchCode,
        handlePatchChange,
        openFile,
        createUntitledFile,
        saveFile,
        renameFile,
        deleteFile,
        closeBuffer,
        keepBuffer,
        renamingPath,
        setRenamingPath,
        handleRenameCommit,
        formatFileLabel,
    } = useEditorBuffers({ workspaceRoot, refreshFileTree });

    // Audio state
    const [isClockRunning, setIsClockRunning] = useState(true);
    const [isRecording, setIsRecording] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [validationErrors, setValidationErrors] = useState<
        ValidationError[] | null
    >(null);

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
    }, [buffers, refreshFileTree, setBuffers]);

    const handleOpenFile = useCallback(
        async (relPath: string, options?: { preview?: boolean }) => {
            try {
                await openFile(relPath, options);
            } catch (error) {
                setError(`Failed to open file: ${relPath}`);
            }
        },
        [openFile],
    );

    const handleDeleteFile = useCallback(
        async (targetIdOrPath?: string) => {
            try {
                await deleteFile(targetIdOrPath);
            } catch (error) {
                setError(getErrorMessage(error, 'Failed to delete file'));
            }
        },
        [deleteFile],
    );

    const handleRenameCommitSafe = useCallback(
        async (oldPath: string, newName: string) => {
            try {
                await handleRenameCommit(oldPath, newName);
            } catch (error) {
                setError(getErrorMessage(error, 'Failed to rename file'));
            }
        },
        [handleRenameCommit],
    );

    // Handle context menu commands
    useEffect(() => {
        return electronAPI.onContextMenuCommand((action) => {
            switch (action.command) {
                case 'save':
                    saveFile(action.bufferId).catch((error) => {
                        setError(getErrorMessage(error, 'Failed to save file'));
                    });
                    break;
                case 'rename':
                    renameFile(action.path || action.bufferId).catch((error) => {
                        setError(
                            getErrorMessage(error, 'Failed to rename file'),
                        );
                    });
                    break;
                case 'delete':
                    deleteFile(action.path || action.bufferId).catch((error) => {
                        setError(getErrorMessage(error, 'Failed to delete file'));
                    });
                    break;
            }
        });
    }, [saveFile, renameFile, deleteFile]);

    const registerScopeCanvas = useCallback(
        (key: string, canvas: HTMLCanvasElement) => {
            scopeCanvasMapRef.current.set(key, canvas);
        },
        [],
    );

    const unregisterScopeCanvas = useCallback((key: string) => {
        scopeCanvasMapRef.current.delete(key);
    }, []);

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

    const handleSaveFile = useCallback(
        async (id?: string) => {
            try {
                await saveFile(id);
            } catch (error) {
                setError(getErrorMessage(error, 'Failed to save file'));
            }
        },
        [saveFile],
    );

    const handleSaveFileRef = useRef(() => {});
    useEffect(() => {
        handleSaveFileRef.current = handleSaveFile;
    }, [handleSaveFile]);

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
                const patchCodeValue = patchCodeRef.current;
                
                // Execute DSL in main process (has direct N-API access)
                const result = await electronAPI.executeDSL(
                    patchCodeValue,
                    activeBufferId,
                );
                
                if (!result.success) {
                    if (result.errorMessage) {
                        setError(result.errorMessage);
                        setValidationErrors(null);
                    } else if (result.errors && result.errors.length > 0) {
                        setValidationErrors(result.errors.flatMap((e) => e.errors || []));
                        setError(
                            result.errors.map((e) => e.message).join('\n') ||
                                'Failed to apply patch.',
                        );
                    }
                    return;
                }
                
                setIsClockRunning(true);
                setRunningBufferId(activeBufferId);
                setLastSubmittedCode(patchCodeValue);
                setError(null);
                setValidationErrors(null);

                const scopeCalls = findScopeCallEndLines(patchCodeValue);
                console.log('Found scope calls:', scopeCalls);
                console.log('Patch scopes:', result.appliedPatch?.scopes);
                const views: ScopeView[] = (result.appliedPatch?.scopes || [])
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
                setError(getErrorMessage(err, 'Unknown error'));
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
            window.removeEventListener(
                'unhandledrejection',
                handleUnhandledRejection,
            );
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
                const content =
                    await electronAPI.filesystem.readFile(configPath);
                const existingBuffer = buffers.find(
                    (b) => b.kind === 'file' && b.filePath === configPath,
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
                    setBuffers((prev) => [...prev, newBuffer]);
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
                            onOpenFile={handleOpenFile}
                            onCreateFile={createUntitledFile}
                            onSaveFile={handleSaveFileRef.current}
                            onRenameFile={renameFile}
                            onDeleteFile={handleDeleteFile}
                            onCloseBuffer={closeBuffer}
                            onSelectWorkspace={selectWorkspaceFolder}
                            onRefreshTree={refreshFileTree}
                            onRenameCommit={handleRenameCommitSafe}
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
