"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
const jsx_runtime_1 = require("react/jsx-runtime");
const react_1 = require("react");
const MonacoPatchEditor_1 = require("./components/MonacoPatchEditor");
const AudioControls_1 = require("./components/AudioControls");
const ErrorDisplay_1 = require("./components/ErrorDisplay");
const Settings_1 = require("./components/Settings");
require("./App.css");
const findScopeCallEndLines_1 = require("./utils/findScopeCallEndLines");
const errorUtils_1 = require("./utils/errorUtils");
const FileExplorer_1 = require("./components/FileExplorer");
const Sidebar_1 = require("./components/Sidebar");
const ControlPanel_1 = require("./components/ControlPanel");
const electronAPI_1 = __importDefault(require("./electronAPI"));
const sliderSourceEdit_1 = require("./dsl/sliderSourceEdit");
const spanTypes_1 = require("../shared/dsl/spanTypes");
const oscilloscope_1 = require("./app/oscilloscope");
const useEditorBuffers_1 = require("./app/hooks/useEditorBuffers");
/**
 * Transform validation errors to use source line numbers instead of module IDs
 * for auto-generated modules (where the ID is meaningless to the user).
 */
function transformErrorsWithSourceLocations(errors, sourceLocationMap) {
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
    const [workspaceRoot, setWorkspaceRoot] = (0, react_1.useState)(null);
    const [fileTree, setFileTree] = (0, react_1.useState)([]);
    const refreshFileTree = (0, react_1.useCallback)(async () => {
        try {
            const tree = await electronAPI_1.default.filesystem.listFiles();
            setFileTree(tree);
        }
        catch (error) {
            console.error('Failed to refresh file tree:', error);
        }
    }, []);
    const { buffers, setBuffers, activeBufferId, setActiveBufferId, patchCode, handlePatchChange, openFile, createUntitledFile, saveFile, renameFile, deleteFile, closeBuffer, keepBuffer, renamingPath, setRenamingPath, handleRenameCommit, formatFileLabel, } = (0, useEditorBuffers_1.useEditorBuffers)({ workspaceRoot, refreshFileTree });
    // Audio state
    const [isClockRunning, setIsClockRunning] = (0, react_1.useState)(true);
    const [isRecording, setIsRecording] = (0, react_1.useState)(false);
    const [isSettingsOpen, setIsSettingsOpen] = (0, react_1.useState)(false);
    const [error, setError] = (0, react_1.useState)(null);
    const [validationErrors, setValidationErrors] = (0, react_1.useState)(null);
    const [scopeViews, setScopeViews] = (0, react_1.useState)([]);
    const [runningBufferId, setRunningBufferId] = (0, react_1.useState)(null);
    const [sliderDefs, setSliderDefs] = (0, react_1.useState)([]);
    const editorRef = (0, react_1.useRef)(null);
    const scopeCanvasMapRef = (0, react_1.useRef)(new Map());
    const handleSliderChange = (0, react_1.useCallback)((label, newValue) => {
        // Find the slider definition
        const slider = sliderDefs.find((s) => s.label === label);
        if (!slider)
            return;
        // Update audio engine via lightweight param update
        electronAPI_1.default.synthesizer.setModuleParam(slider.moduleId, 'signal', { source: newValue });
        // Update the source code in the editor
        const editorInstance = editorRef.current;
        if (editorInstance) {
            const model = editorInstance.getModel();
            if (model) {
                const source = model.getValue();
                const span = (0, sliderSourceEdit_1.findSliderValueSpan)(source, label);
                if (span) {
                    const startPos = model.getPositionAt(span.start);
                    const endPos = model.getPositionAt(span.end);
                    const range = new window.monaco.Range(startPos.lineNumber, startPos.column, endPos.lineNumber, endPos.column);
                    const formattedValue = Number(newValue.toPrecision(6)).toString();
                    // Use pushEditOperations for proper undo stack integration
                    model.pushEditOperations([], [{ range, text: formattedValue }], () => null);
                }
            }
        }
        // Update slider state
        setSliderDefs((prev) => prev.map((s) => s.label === label ? { ...s, value: newValue } : s));
    }, [sliderDefs]);
    // Load workspace and file tree on mount
    (0, react_1.useEffect)(() => {
        electronAPI_1.default.filesystem
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
    const selectWorkspaceFolder = (0, react_1.useCallback)(async () => {
        // Check for dirty file-backed buffers before switching
        const dirtyFileBuffers = buffers.filter((b) => b.kind === 'file' && b.dirty);
        if (dirtyFileBuffers.length > 0) {
            const fileList = dirtyFileBuffers
                .map((b) => (b.kind === 'file' ? b.filePath : ''))
                .filter(Boolean)
                .join(', ');
            const shouldSave = window.confirm(`You have unsaved changes in: ${fileList}. Save changes before switching workspace?`);
            if (shouldSave) {
                // Save all dirty file buffers
                for (const buffer of dirtyFileBuffers) {
                    if (buffer.kind === 'file') {
                        await electronAPI_1.default.filesystem.writeFile(buffer.filePath, buffer.content);
                    }
                }
                // Mark them clean
                setBuffers((prev) => prev.map((b) => b.kind === 'file' && b.dirty
                    ? { ...b, dirty: false }
                    : b));
            }
            else {
                // Discard changes: remove dirty file buffers from the list
                setBuffers((prev) => prev.filter((b) => !(b.kind === 'file' && b.dirty)));
            }
        }
        const workspace = await electronAPI_1.default.filesystem.selectWorkspace();
        if (workspace) {
            setWorkspaceRoot(workspace.path);
            await refreshFileTree();
        }
    }, [buffers, refreshFileTree, setBuffers]);
    const handleOpenFile = (0, react_1.useCallback)(async (relPath, options) => {
        try {
            await openFile(relPath, options);
        }
        catch (error) {
            setError(`Failed to open file: ${relPath}`);
        }
    }, [openFile]);
    const handleDeleteFile = (0, react_1.useCallback)(async (targetIdOrPath) => {
        try {
            await deleteFile(targetIdOrPath);
        }
        catch (error) {
            setError((0, errorUtils_1.getErrorMessage)(error, 'Failed to delete file'));
        }
    }, [deleteFile]);
    const handleRenameCommitSafe = (0, react_1.useCallback)(async (oldPath, newName) => {
        try {
            await handleRenameCommit(oldPath, newName);
        }
        catch (error) {
            setError((0, errorUtils_1.getErrorMessage)(error, 'Failed to rename file'));
        }
    }, [handleRenameCommit]);
    // Handle context menu commands
    (0, react_1.useEffect)(() => {
        return electronAPI_1.default.onContextMenuCommand((action) => {
            switch (action.command) {
                case 'save':
                    saveFile(action.bufferId).catch((error) => {
                        setError((0, errorUtils_1.getErrorMessage)(error, 'Failed to save file'));
                    });
                    break;
                case 'rename':
                    renameFile(action.path || action.bufferId).catch((error) => {
                        setError((0, errorUtils_1.getErrorMessage)(error, 'Failed to rename file'));
                    });
                    break;
                case 'delete':
                    deleteFile(action.path || action.bufferId).catch((error) => {
                        setError((0, errorUtils_1.getErrorMessage)(error, 'Failed to delete file'));
                    });
                    break;
            }
        });
    }, [saveFile, renameFile, deleteFile]);
    const registerScopeCanvas = (0, react_1.useCallback)((key, canvas) => {
        scopeCanvasMapRef.current.set(key, canvas);
    }, []);
    const unregisterScopeCanvas = (0, react_1.useCallback)((key) => {
        scopeCanvasMapRef.current.delete(key);
    }, []);
    const patchCodeRef = (0, react_1.useRef)(patchCode);
    (0, react_1.useEffect)(() => {
        patchCodeRef.current = patchCode;
    }, [patchCode]);
    const isClockRunningRef = (0, react_1.useRef)(isClockRunning);
    (0, react_1.useEffect)(() => {
        isClockRunningRef.current = isClockRunning;
    }, [isClockRunning]);
    (0, react_1.useEffect)(() => {
        if (isClockRunningRef.current) {
            const tick = () => {
                electronAPI_1.default.synthesizer
                    .getScopes()
                    .then((scopes) => {
                    for (const [scopeItem, channels, stats] of scopes) {
                        const scopeKey = (0, oscilloscope_1.scopeKeyFromSubscription)(scopeItem);
                        const scopedCanvas = scopeCanvasMapRef.current.get(scopeKey);
                        if (scopedCanvas) {
                            const rangeMin = parseFloat(scopedCanvas.dataset.scopeRangeMin || '-5');
                            const rangeMax = parseFloat(scopedCanvas.dataset.scopeRangeMax || '5');
                            (0, oscilloscope_1.drawOscilloscope)(channels, scopedCanvas, { range: [rangeMin, rangeMax], stats });
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
    const handleSaveFile = (0, react_1.useCallback)(async (id) => {
        try {
            await saveFile(id);
        }
        catch (error) {
            setError((0, errorUtils_1.getErrorMessage)(error, 'Failed to save file'));
        }
    }, [saveFile]);
    const handleSaveFileRef = (0, react_1.useRef)(() => { });
    (0, react_1.useEffect)(() => {
        handleSaveFileRef.current = handleSaveFile;
    }, [handleSaveFile]);
    const handleOpenWorkspaceRef = (0, react_1.useRef)(() => { });
    (0, react_1.useEffect)(() => {
        handleOpenWorkspaceRef.current = selectWorkspaceFolder;
    }, [selectWorkspaceFolder]);
    const handleSubmitRef = (0, react_1.useRef)(() => { });
    (0, react_1.useEffect)(() => {
        handleSubmitRef.current = async () => {
            if (!activeBufferId)
                return;
            try {
                const patchCodeValue = patchCodeRef.current;
                // Execute DSL in main process (has direct N-API access)
                const result = await electronAPI_1.default.executeDSL(patchCodeValue, activeBufferId);
                if (!result.success) {
                    // Still set interpolation resolutions even on validation errors
                    // (the analysis succeeded, only the patch application failed)
                    if (result.interpolationResolutions) {
                        const map = new Map(Object.entries(result.interpolationResolutions));
                        (0, spanTypes_1.setActiveInterpolationResolutions)(map);
                    }
                    if (result.errorMessage) {
                        setError(result.errorMessage);
                        setValidationErrors(null);
                    }
                    else if (result.errors && result.errors.length > 0) {
                        // Extract and transform validation errors to show source lines
                        const rawErrors = result.errors.flatMap((e) => e.errors || []);
                        const transformedErrors = transformErrorsWithSourceLocations(rawErrors, result.sourceLocationMap);
                        setValidationErrors(transformedErrors);
                        setError(result.errors.map((e) => e.message).join('\n') ||
                            'Failed to apply patch.');
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
                    (0, spanTypes_1.setActiveInterpolationResolutions)(map);
                }
                const scopeCalls = (0, findScopeCallEndLines_1.findScopeCallEndLines)(patchCodeValue);
                console.log('Found scope calls:', scopeCalls);
                console.log('Patch scopes:', result.appliedPatch?.scopes);
                const views = (result.appliedPatch?.scopes || [])
                    .map((scope, idx) => {
                    const call = scopeCalls[idx];
                    if (!call)
                        return null;
                    const { moduleId, portName } = scope.item;
                    return {
                        key: `:module:${moduleId}:${portName}`,
                        lineNumber: call.endLine,
                        file: activeBufferId,
                        range: scope.range ?? [-5, 5],
                    };
                })
                    .filter((v) => v !== null);
                console.log('Scope views:', views);
                setScopeViews(views);
                // Update slider definitions from DSL execution
                setSliderDefs(result.sliders ?? []);
            }
            catch (err) {
                setError((0, errorUtils_1.getErrorMessage)(err, 'Unknown error'));
                setValidationErrors(null);
            }
        };
    }, [activeBufferId]);
    const handleStopRef = (0, react_1.useRef)(() => { });
    (0, react_1.useEffect)(() => {
        handleStopRef.current = async () => {
            await electronAPI_1.default.synthesizer.stop();
            setIsClockRunning(false);
        };
    }, []);
    const dismissError = (0, react_1.useCallback)(() => {
        setError(null);
        setValidationErrors(null);
    }, []);
    (0, react_1.useEffect)(() => {
        const cleanupSave = electronAPI_1.default.onMenuSave(() => {
            handleSaveFileRef.current();
        });
        const cleanupStop = electronAPI_1.default.onMenuStop(() => {
            handleStopRef.current();
        });
        const cleanupUpdate = electronAPI_1.default.onMenuUpdatePatch(() => {
            handleSubmitRef.current();
        });
        const cleanupOpenWorkspace = electronAPI_1.default.onMenuOpenWorkspace(() => {
            handleOpenWorkspaceRef.current();
        });
        const cleanupCloseBuffer = electronAPI_1.default.onMenuCloseBuffer(() => {
            if (activeBufferId) {
                closeBuffer(activeBufferId);
            }
        });
        const cleanupToggleRecording = electronAPI_1.default.onMenuToggleRecording(() => {
            if (isRecording) {
                electronAPI_1.default.synthesizer.stopRecording();
                setIsRecording(false);
            }
            else {
                electronAPI_1.default.synthesizer.startRecording();
                setIsRecording(true);
            }
        });
        // Handle opening settings from menu (Cmd+,)
        const cleanupOpenSettings = electronAPI_1.default.onMenuOpenSettings(() => {
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
    return ((0, jsx_runtime_1.jsxs)("div", { className: "app", children: [(0, jsx_runtime_1.jsx)("header", { className: "app-header", children: (0, jsx_runtime_1.jsx)(AudioControls_1.AudioControls, { isRunning: isClockRunning, isRecording: isRecording, onStop: handleStopRef.current, onStartRecording: async () => {
                        await electronAPI_1.default.synthesizer.startRecording();
                        setIsRecording(true);
                    }, onStopRecording: async () => {
                        await electronAPI_1.default.synthesizer.stopRecording();
                        setIsRecording(false);
                    }, onUpdatePatch: handleSubmitRef.current }) }), (0, jsx_runtime_1.jsx)(ErrorDisplay_1.ErrorDisplay, { error: error, errors: validationErrors, onDismiss: dismissError }), (0, jsx_runtime_1.jsx)(Settings_1.Settings, { isOpen: isSettingsOpen, onClose: () => setIsSettingsOpen(false) }), (0, jsx_runtime_1.jsx)("main", { className: "app-main", children: !workspaceRoot ? ((0, jsx_runtime_1.jsx)("div", { className: "empty-state", children: (0, jsx_runtime_1.jsx)("button", { className: "open-folder-button", onClick: selectWorkspaceFolder, children: "Open Folder" }) })) : ((0, jsx_runtime_1.jsxs)(jsx_runtime_1.Fragment, { children: [(0, jsx_runtime_1.jsx)("div", { className: "editor-panel", children: (0, jsx_runtime_1.jsx)(MonacoPatchEditor_1.MonacoPatchEditor, { value: patchCode, runningBufferId: runningBufferId, currentFile: activeBufferId, onChange: handlePatchChange, editorRef: editorRef, scopeViews: scopeViews, onRegisterScopeCanvas: registerScopeCanvas, onUnregisterScopeCanvas: unregisterScopeCanvas }) }), (0, jsx_runtime_1.jsx)(Sidebar_1.Sidebar, { explorerContent: (0, jsx_runtime_1.jsx)(FileExplorer_1.FileExplorer, { workspaceRoot: workspaceRoot, fileTree: fileTree, buffers: buffers, activeBufferId: activeBufferId, runningBufferId: runningBufferId, renamingPath: renamingPath, formatLabel: (buffer) => {
                                    const path = formatFileLabel(buffer);
                                    const parts = path.split(/[/\\]/);
                                    return parts[parts.length - 1];
                                }, onSelectBuffer: setActiveBufferId, onOpenFile: handleOpenFile, onCreateFile: createUntitledFile, onSaveFile: handleSaveFileRef.current, onRenameFile: renameFile, onDeleteFile: handleDeleteFile, onCloseBuffer: closeBuffer, onSelectWorkspace: selectWorkspaceFolder, onRefreshTree: refreshFileTree, onRenameCommit: handleRenameCommitSafe, onRenameCancel: () => setRenamingPath(null), onKeepBuffer: keepBuffer }), controlContent: (0, jsx_runtime_1.jsx)(ControlPanel_1.ControlPanel, { sliders: sliderDefs, onSliderChange: handleSliderChange }) })] })) })] }));
}
exports.default = App;
//# sourceMappingURL=App.js.map