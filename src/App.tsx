import { useCallback, useEffect, useRef, useState } from 'react';
import { MonacoPatchEditor as PatchEditor } from './components/MonacoPatchEditor';
import { AudioControls } from './components/AudioControls';
import { ErrorDisplay } from './components/ErrorDisplay';
import { Settings } from './components/Settings';
import './App.css';
import type { editor } from 'monaco-editor';
import { findScopeCallEndLines } from './utils/findScopeCallEndLines';
import { getErrorMessage } from './utils/errorUtils';
import { FileExplorer } from './components/FileExplorer';
import { Sidebar } from './components/Sidebar';
import { ControlPanel } from './components/ControlPanel';
import electronAPI from './electronAPI';
import { ValidationError } from '@modular/core';
import type { FileTreeEntry, SourceLocationInfo } from './ipcTypes';
import type { SliderDefinition } from './dsl/sliderTypes';
import { findSliderValueSpan } from './dsl/sliderSourceEdit';
import type { EditorBuffer, ScopeView } from './types/editor';
import { getBufferId } from './app/buffers';
import { setActiveInterpolationResolutions } from './dsl/spanTypes';
import { drawOscilloscope, scopeKeyFromSubscription } from './app/oscilloscope';
import { useEditorBuffers } from './app/hooks/useEditorBuffers';

/**
 * Transform validation errors to use source line numbers instead of module IDs
 * for auto-generated modules (where the ID is meaningless to the user).
 */
function transformErrorsWithSourceLocations(
    errors: ValidationError[],
    sourceLocationMap?: Record<string, SourceLocationInfo>
): ValidationError[] {
    if (!sourceLocationMap) {
        return errors;
    }

    return errors.map((err) => {
        // The location field contains module ID like "sine-1" or user's explicit ID
        if (!err.location) {
            return err;
        }

        // Parse the location - it's either:
        // - "'myModule'" for explicit IDs (from format_module_location in Rust)
        // - "moduleName(...)" for auto-generated IDs
        const explicitIdMatch = err.location.match(/^'([^']+)'$/);
        if (explicitIdMatch) {
            // User explicitly named this module - keep showing the ID
            return err;
        }

        // For auto-generated module locations like "sine(...)", 
        // try to find source line from the map
        // The moduleType(...) format is produced by Rust, but we need the actual moduleId
        // to look up in the map. Let's check all entries in the map.
        for (const [moduleId, loc] of Object.entries(sourceLocationMap)) {
            if (!loc.idIsExplicit && err.location.includes(moduleId.split('-')[0])) {
                // Found a match - replace location with line number
                return {
                    ...err,
                    location: `line ${loc.line}`,
                };
            }
        }

        return err;
    });
}


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
    const [isSettingsOpen, setIsSettingsOpen] = useState(false);
    const [error, setError] = useState<string | null>(null);
    const [validationErrors, setValidationErrors] = useState<
        ValidationError[] | null
    >(null);

    const [scopeViews, setScopeViews] = useState<ScopeView[]>([]);
    const [runningBufferId, setRunningBufferId] = useState<string | null>(null);
    const [sliderDefs, setSliderDefs] = useState<SliderDefinition[]>([]);

    const editorRef = useRef<editor.IStandaloneCodeEditor>(null);
    const scopeCanvasMapRef = useRef<Map<string, HTMLCanvasElement>>(new Map());

    const handleSliderChange = useCallback(
        (label: string, newValue: number) => {
            // Find the slider definition
            const slider = sliderDefs.find((s) => s.label === label);
            if (!slider) return;

            // Update audio engine via lightweight param update
            electronAPI.synthesizer.setModuleParam(
                slider.moduleId,
                'signal',
                { source: newValue },
            );

            // Update the source code in the editor
            const editorInstance = editorRef.current;
            if (editorInstance) {
                const model = editorInstance.getModel();
                if (model) {
                    const source = model.getValue();
                    const span = findSliderValueSpan(source, label);
                    if (span) {
                        const startPos = model.getPositionAt(span.start);
                        const endPos = model.getPositionAt(span.end);
                        const range = new (window as any).monaco.Range(
                            startPos.lineNumber,
                            startPos.column,
                            endPos.lineNumber,
                            endPos.column,
                        );
                        const formattedValue = Number(newValue.toPrecision(6)).toString();
                        // Use pushEditOperations for proper undo stack integration
                        model.pushEditOperations(
                            [],
                            [{ range, text: formattedValue }],
                            () => null,
                        );
                    }
                }
            }

            // Update slider state
            setSliderDefs((prev) =>
                prev.map((s) =>
                    s.label === label ? { ...s, value: newValue } : s,
                ),
            );
        },
        [sliderDefs],
    );

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
                        for (const [scopeItem, channels, stats] of scopes) {
                            const scopeKey =
                                scopeKeyFromSubscription(scopeItem);
                            const scopedCanvas =
                                scopeCanvasMapRef.current.get(scopeKey);
                            if (scopedCanvas) {
                                const scale = parseFloat(scopedCanvas.dataset.scopeScale || '5');
                                drawOscilloscope(channels, scopedCanvas, { scale, stats });
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
                    // Still set interpolation resolutions even on validation errors
                    // (the analysis succeeded, only the patch application failed)
                    if (result.interpolationResolutions) {
                        const map = new Map(Object.entries(result.interpolationResolutions));
                        setActiveInterpolationResolutions(map);
                    }
                    if (result.errorMessage) {
                        setError(result.errorMessage);
                        setValidationErrors(null);
                    } else if (result.errors && result.errors.length > 0) {
                        // Extract and transform validation errors to show source lines
                        const rawErrors = result.errors.flatMap((e) => e.errors || []);
                        const transformedErrors = transformErrorsWithSourceLocations(
                            rawErrors,
                            result.sourceLocationMap
                        );
                        setValidationErrors(transformedErrors);
                        setError(
                            result.errors.map((e) => e.message).join('\n') ||
                                'Failed to apply patch.',
                        );
                    }
                    return;
                }
                
                setIsClockRunning(true);
                setRunningBufferId(activeBufferId);
                setError(null);
                setValidationErrors(null);

                // Set interpolation resolutions in renderer for template literal highlighting
                if (result.interpolationResolutions) {
                    const map = new Map(Object.entries(result.interpolationResolutions));
                    setActiveInterpolationResolutions(map);
                }

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
                            scale: scope.scale ?? 5,
                        };
                    })
                    .filter((v): v is ScopeView => v !== null);
                console.log('Scope views:', views);

                setScopeViews(views);

                // Update slider definitions from DSL execution
                setSliderDefs(result.sliders ?? []);
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
        const cleanupOpenSettings = electronAPI.onMenuOpenSettings(() => {
            setIsSettingsOpen(true);
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

            <Settings
                isOpen={isSettingsOpen}
                onClose={() => setIsSettingsOpen(false)}
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
                                runningBufferId={runningBufferId}
                                currentFile={activeBufferId}
                                onChange={handlePatchChange}
                                editorRef={editorRef}
                                scopeViews={scopeViews}
                                onRegisterScopeCanvas={registerScopeCanvas}
                                onUnregisterScopeCanvas={unregisterScopeCanvas}
                            />
                        </div>

                        <Sidebar
                            explorerContent={
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
                            }
                            controlContent={
                                <ControlPanel
                                    sliders={sliderDefs}
                                    onSliderChange={handleSliderChange}
                                />
                            }
                        />
                    </>
                )}
            </main>
        </div>
    );
}

export default App;
