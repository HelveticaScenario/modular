import { useCallback, useEffect, useRef, useState } from 'react';
import { MonacoPatchEditor as PatchEditor } from './components/MonacoPatchEditor';
import { AudioControls } from './components/AudioControls';
import { TransportDisplay } from './components/TransportDisplay';
import { ErrorDisplay } from './components/ErrorDisplay';
import { Settings } from './components/Settings';
import './App.css';
// import type { editor } from 'monaco-editor';
import { editor } from 'monaco-editor';
import { getErrorMessage } from './utils/errorUtils';
import { FileExplorer } from './components/FileExplorer';
import { Sidebar } from './components/Sidebar';
import { ControlPanel } from './components/ControlPanel';
import electronAPI from './electronAPI';
import { ValidationError } from '@modular/core';
import type { QueuedTrigger } from '@modular/core';
import type {
    FileTreeEntry,
    SourceLocationInfo,
    TransportSnapshot,
} from '../shared/ipcTypes';
import type { SliderDefinition } from '../shared/dsl/sliderTypes';
import { findSliderValueSpan } from './dsl/sliderSourceEdit';
import type { EditorBuffer, ScopeView } from './types/editor';
import { getBufferId } from './app/buffers';
import { setActiveInterpolationResolutions } from '../shared/dsl/spanTypes';
import { drawOscilloscope, scopeKeyFromSubscription } from './app/oscilloscope';
import { useEditorBuffers } from './app/hooks/useEditorBuffers';

/**
 * Transform validation errors to use source line numbers instead of module IDs
 * for auto-generated modules (where the ID is meaningless to the user).
 */
function transformErrorsWithSourceLocations(
    errors: ValidationError[],
    sourceLocationMap?: Record<string, SourceLocationInfo>,
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
            if (
                !loc.idIsExplicit &&
                err.location.includes(moduleId.split('-')[0])
            ) {
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
    const [transportState, setTransportState] =
        useState<TransportSnapshot | null>(null);

    const editorRef = useRef<editor.IStandaloneCodeEditor>(null);
    const scopeCanvasMapRef = useRef<Map<string, HTMLCanvasElement>>(new Map());
    const lastPatchResultRef = useRef<any>(null);

    /** Long-lived invisible tracked decorations spanning each scope() call.
     *  Monaco automatically adjusts these ranges as the document is edited,
     *  so we can always read the current position of a scope call from them. */
    const scopeDecorationsRef =
        useRef<editor.IEditorDecorationsCollection | null>(null);

    /** Pending UI state waiting for the audio thread to apply a queued update */
    const pendingUIStateRef = useRef<{
        updateId: number;
        scopeViews: ScopeView[];
        sliderDefs: SliderDefinition[];
        interpolationResolutions?: Map<string, any[]>;
        /** Tracked decorations created at submit time, swapped into
         *  scopeDecorationsRef when the pending state is committed. */
        scopeDecorations: editor.IEditorDecorationsCollection | null;
    } | null>(null);

    const handleSliderChange = useCallback(
        (label: string, newValue: number) => {
            // Find the slider definition
            const slider = sliderDefs.find((s) => s.label === label);
            if (!slider) return;

            // Update audio engine via lightweight param update
            electronAPI.synthesizer.setModuleParam(slider.moduleId, 'signal', {
                source: newValue,
            });

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
                        const formattedValue = Number(
                            newValue.toPrecision(6),
                        ).toString();
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

            const response =
                await electronAPI.showUnsavedChangesDialog(fileList);

            if (response === 2) {
                // Cancel / Escape: abort the open workspace operation
                return;
            } else if (response === 0) {
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
                // Don't Save: discard dirty file buffers
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
                    renameFile(action.path || action.bufferId).catch(
                        (error) => {
                            setError(
                                getErrorMessage(error, 'Failed to rename file'),
                            );
                        },
                    );
                    break;
                case 'delete':
                    deleteFile(action.path || action.bufferId).catch(
                        (error) => {
                            setError(
                                getErrorMessage(error, 'Failed to delete file'),
                            );
                        },
                    );
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
                Promise.all([
                    electronAPI.synthesizer.getScopes(),
                    electronAPI.synthesizer.getTransportState(),
                ])
                    .then(([scopes, transport]) => {
                        for (const [scopeItem, channels, stats] of scopes) {
                            const scopeKey =
                                scopeKeyFromSubscription(scopeItem);
                            const scopedCanvas =
                                scopeCanvasMapRef.current.get(scopeKey);
                            if (scopedCanvas) {
                                const rangeMin = parseFloat(
                                    scopedCanvas.dataset.scopeRangeMin || '-5',
                                );
                                const rangeMax = parseFloat(
                                    scopedCanvas.dataset.scopeRangeMax || '5',
                                );
                                drawOscilloscope(channels, scopedCanvas, {
                                    range: [rangeMin, rangeMax],
                                    stats,
                                });
                            }
                        }
                        setTransportState(transport);

                        // Check if a pending UI state should be committed
                        const pending = pendingUIStateRef.current;
                        if (
                            pending &&
                            transport.lastAppliedUpdateId >= pending.updateId
                        ) {
                            pendingUIStateRef.current = null;
                            // Swap decoration collections: dispose old, activate pending
                            scopeDecorationsRef.current?.clear();
                            scopeDecorationsRef.current =
                                pending.scopeDecorations;
                            setScopeViews(pending.scopeViews);
                            setSliderDefs(pending.sliderDefs);
                            if (pending.interpolationResolutions) {
                                setActiveInterpolationResolutions(
                                    pending.interpolationResolutions,
                                );
                            }
                        }

                        if (isClockRunningRef.current) {
                            requestAnimationFrame(tick);
                        }
                    })
                    .catch((err) => {
                        console.error('Failed to get scopes:', err);
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

    const handleSubmitRef = useRef((trigger?: QueuedTrigger) => {});
    useEffect(() => {
        handleSubmitRef.current = async (trigger?: QueuedTrigger) => {
            if (!activeBufferId) return;
            try {
                const patchCodeValue = patchCodeRef.current;

                // Execute DSL in main process (has direct N-API access)
                const result = await electronAPI.executeDSL(
                    patchCodeValue,
                    activeBufferId,
                    trigger,
                );
                lastPatchResultRef.current = result;

                if (!result.success) {
                    // Still set interpolation resolutions even on validation errors
                    // (the analysis succeeded, only the patch application failed)
                    if (result.interpolationResolutions) {
                        const map = new Map(
                            Object.entries(result.interpolationResolutions),
                        );
                        setActiveInterpolationResolutions(map);
                    }
                    if (result.errorMessage) {
                        setError(result.errorMessage);
                        setValidationErrors(null);
                    } else if (result.errors && result.errors.length > 0) {
                        // Extract and transform validation errors to show source lines
                        const rawErrors = result.errors.flatMap(
                            (e) => e.errors || [],
                        );
                        const transformedErrors =
                            transformErrorsWithSourceLocations(
                                rawErrors,
                                result.sourceLocationMap,
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
                const interpolationMap = result.interpolationResolutions
                    ? new Map(Object.entries(result.interpolationResolutions))
                    : undefined;

                const scopes = result.appliedPatch?.scopes || [];
                const callSiteSpans = result.callSiteSpans;

                const editorInstance = editorRef.current;
                const model = editorInstance?.getModel();
                const views: ScopeView[] = [];
                const decorationDescs: editor.IModelDeltaDecoration[] = [];

                for (let i = 0; i < scopes.length; i++) {
                    const scope = scopes[i];
                    const { moduleId, portName } = scope.item;

                    const loc = (scope as any).sourceLocation as
                        | { line: number; column: number }
                        | undefined;

                    views.push({
                        key: `:module:${moduleId}:${portName}`,
                        file: activeBufferId,
                        range: scope.range ?? [-5, 5],
                    });

                    if (model && loc) {
                        // Look up the full call expression span for this scope call.
                        // The key matches captureSourceLocation's {line, column} format.
                        const spanKey = `${loc.line}:${loc.column}`;
                        const callSpan = callSiteSpans?.[spanKey];
                        const endLine = callSpan?.endLine ?? loc.line;

                        const endLineContent =
                            model.getLineContent(endLine) ?? '';
                        decorationDescs.push({
                            range: {
                                startLineNumber: loc.line,
                                startColumn: loc.column,
                                endLineNumber: endLine,
                                endColumn: endLineContent.length + 1,
                            },
                            options: {
                                stickiness:
                                    editor.TrackedRangeStickiness
                                        .NeverGrowsWhenTypingAtEdges,
                            },
                        });
                    }
                }

                let newScopeDecorations: editor.IEditorDecorationsCollection | null =
                    null;
                if (editorInstance && decorationDescs.length > 0) {
                    newScopeDecorations =
                        editorInstance.createDecorationsCollection(
                            decorationDescs,
                        );
                }

                const newSliderDefs = result.sliders ?? [];

                // For queued (non-immediate) triggers, defer UI state until the
                // audio thread actually applies the patch update.
                const isDeferred =
                    trigger === 'NextBar' || trigger === 'NextBeat';

                if (isDeferred && result.updateId != null) {
                    // Stash both new decorations and UI state; keep old
                    // decorations alive so the current view zones still work.
                    // Any previously pending (but never committed) decorations
                    // are cleaned up before storing the new pending state.
                    pendingUIStateRef.current?.scopeDecorations?.clear();
                    pendingUIStateRef.current = {
                        updateId: result.updateId,
                        scopeViews: views,
                        sliderDefs: newSliderDefs,
                        interpolationResolutions: interpolationMap,
                        scopeDecorations: newScopeDecorations,
                    };
                } else {
                    // Immediate trigger (or button click): swap decorations
                    // and apply UI state right away.
                    pendingUIStateRef.current?.scopeDecorations?.clear();
                    pendingUIStateRef.current = null;
                    scopeDecorationsRef.current?.clear();
                    scopeDecorationsRef.current = newScopeDecorations;
                    setScopeViews(views);
                    setSliderDefs(newSliderDefs);
                    if (interpolationMap) {
                        setActiveInterpolationResolutions(interpolationMap);
                    }
                }
            } catch (err) {
                setError(getErrorMessage(err, 'Unknown error'));
                setValidationErrors(null);
            }
        };
    }, [activeBufferId]);

    // Expose test API for E2E tests
    useEffect(() => {
        window.__TEST_API__ = {
            getEditorValue: () => editorRef.current?.getValue() ?? '',
            setEditorValue: (code: string) => editorRef.current?.setValue(code),
            executePatch: async () => {
                handleSubmitRef.current();
            },
            getLastPatchResult: () => lastPatchResultRef.current,
            getScopeData: () => electronAPI.synthesizer.getScopes(),
            getAudioHealth: () => electronAPI.synthesizer.getHealth(),
            isClockRunning: () => isClockRunningRef.current,
        };
        return () => {
            delete window.__TEST_API__;
        };
    }, []);

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
        const cleanupNewFile = electronAPI.onMenuNewFile(() => {
            createUntitledFile();
        });
        const cleanupSave = electronAPI.onMenuSave(() => {
            handleSaveFileRef.current();
        });
        const cleanupStop = electronAPI.onMenuStop(() => {
            handleStopRef.current();
        });
        const cleanupUpdate = electronAPI.onMenuUpdatePatch(() => {
            handleSubmitRef.current('NextBar');
        });
        const cleanupUpdateNextBeat = electronAPI.onMenuUpdatePatchNextBeat(
            () => {
                handleSubmitRef.current('NextBeat');
            },
        );
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
            cleanupNewFile();
            cleanupSave();
            cleanupStop();
            cleanupUpdate();
            cleanupUpdateNextBeat();
            cleanupOpenWorkspace();
            cleanupCloseBuffer();
            cleanupToggleRecording();
            cleanupOpenSettings();
        };
    }, [activeBufferId, closeBuffer, isRecording, buffers]);

    return (
        <div className="app">
            <header className="app-header">
                <TransportDisplay transport={transportState} />
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
                    onUpdatePatch={() => handleSubmitRef.current()}
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
                                scopeDecorations={scopeDecorationsRef.current}
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
