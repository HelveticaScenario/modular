"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
// See the Electron documentation for details on how to use preload scripts:
// https://www.electronjs.org/docs/latest/tutorial/process-model#preload-scripts
const renderer_1 = require("electron/renderer");
const ipcTypes_1 = require("../shared/ipcTypes");
/**
 * Type-safe wrapper for IPC invoke calls
 */
function invokeIPC(channel, ...args) {
    // @ts-ignore - TypeScript is having trouble inferring the return type here
    return renderer_1.ipcRenderer.invoke(ipcTypes_1.IPC_CHANNELS[channel], ...args);
}
const electronAPI = {
    // Schema operations
    getSchemas: (...args) => invokeIPC('GET_SCHEMAS', ...args),
    getMiniLeafSpans: (...args) => invokeIPC('GET_MINI_LEAF_SPANS', ...args),
    // DSL operations
    executeDSL: (source, sourceId) => invokeIPC('DSL_EXECUTE', source, sourceId),
    getDslLibSource: () => invokeIPC('GET_DSL_LIB_SOURCE'),
    // Window operations
    openHelpWindow: () => invokeIPC('OPEN_HELP_WINDOW'),
    openHelpForSymbol: (symbolType, symbolName) => invokeIPC('OPEN_HELP_FOR_SYMBOL', symbolType, symbolName),
    onNavigateToSymbol: (callback) => {
        const listener = (_event, data) => callback(data);
        renderer_1.ipcRenderer.on('navigate-to-symbol', listener);
        return () => renderer_1.ipcRenderer.removeListener('navigate-to-symbol', listener);
    },
    // Synthesizer operations
    synthesizer: {
        getSampleRate: (...args) => invokeIPC('SYNTH_GET_SAMPLE_RATE', ...args),
        getChannels: (...args) => invokeIPC('SYNTH_GET_CHANNELS', ...args),
        getScopes: (...args) => invokeIPC('SYNTH_GET_SCOPES', ...args),
        getModuleStates: (...args) => invokeIPC('SYNTH_GET_MODULE_STATES', ...args),
        updatePatch: (...args) => invokeIPC('SYNTH_UPDATE_PATCH', ...args),
        startRecording: (...args) => invokeIPC('SYNTH_START_RECORDING', ...args),
        stopRecording: (...args) => invokeIPC('SYNTH_STOP_RECORDING', ...args),
        isRecording: (...args) => invokeIPC('SYNTH_IS_RECORDING', ...args),
        getHealth: (...args) => invokeIPC('SYNTH_GET_HEALTH', ...args),
        stop: (...args) => invokeIPC('SYNTH_STOP', ...args),
        isStopped: (...args) => invokeIPC('SYNTH_IS_STOPPED', ...args),
        setModuleParam: (...args) => invokeIPC('SYNTH_SET_MODULE_PARAM', ...args),
    },
    // Audio device operations
    audio: {
        // New API
        refreshDeviceCache: (...args) => invokeIPC('AUDIO_REFRESH_DEVICE_CACHE', ...args),
        getDeviceCache: (...args) => invokeIPC('AUDIO_GET_DEVICE_CACHE', ...args),
        getCurrentState: (...args) => invokeIPC('AUDIO_GET_CURRENT_STATE', ...args),
        recreateStreams: (...args) => invokeIPC('AUDIO_RECREATE_STREAMS', ...args),
        onFallbackWarning: menuEventHandler(ipcTypes_1.IPC_CHANNELS.AUDIO_FALLBACK_WARNING),
        // Legacy (kept for backward compatibility)
        refreshDeviceList: (...args) => invokeIPC('AUDIO_REFRESH_DEVICE_LIST', ...args),
        listHosts: (...args) => invokeIPC('AUDIO_LIST_HOSTS', ...args),
        listOutputDevices: (...args) => invokeIPC('AUDIO_LIST_OUTPUT_DEVICES', ...args),
        listInputDevices: (...args) => invokeIPC('AUDIO_LIST_INPUT_DEVICES', ...args),
        getOutputDevice: (...args) => invokeIPC('AUDIO_GET_OUTPUT_DEVICE', ...args),
        getInputDevice: (...args) => invokeIPC('AUDIO_GET_INPUT_DEVICE', ...args),
        setOutputDevice: (...args) => invokeIPC('AUDIO_SET_OUTPUT_DEVICE', ...args),
        setInputDevice: (...args) => invokeIPC('AUDIO_SET_INPUT_DEVICE', ...args),
        getInputChannels: (...args) => invokeIPC('AUDIO_GET_INPUT_CHANNELS', ...args),
    },
    // MIDI device operations
    midi: {
        listInputs: (...args) => invokeIPC('MIDI_LIST_INPUTS', ...args),
        getInput: (...args) => invokeIPC('MIDI_GET_INPUT', ...args),
        setInput: (...args) => invokeIPC('MIDI_SET_INPUT', ...args),
        tryReconnect: (...args) => invokeIPC('MIDI_TRY_RECONNECT', ...args),
    },
    // Filesystem operations
    filesystem: {
        selectWorkspace: (...args) => invokeIPC('FS_SELECT_WORKSPACE', ...args),
        getWorkspace: (...args) => invokeIPC('FS_GET_WORKSPACE', ...args),
        listFiles: (...args) => invokeIPC('FS_LIST_FILES', ...args),
        readFile: (...args) => invokeIPC('FS_READ_FILE', ...args),
        writeFile: (...args) => invokeIPC('FS_WRITE_FILE', ...args),
        renameFile: (...args) => invokeIPC('FS_RENAME_FILE', ...args),
        deleteFile: (...args) => invokeIPC('FS_DELETE_FILE', ...args),
        moveFile: (...args) => invokeIPC('FS_MOVE_FILE', ...args),
        createFolder: (...args) => invokeIPC('FS_CREATE_FOLDER', ...args),
        showSaveDialog: (...args) => invokeIPC('FS_SHOW_SAVE_DIALOG', ...args),
        showInputDialog: (...args) => invokeIPC('FS_SHOW_INPUT_DIALOG', ...args),
    },
    // Menu events
    onMenuSave: menuEventHandler(ipcTypes_1.MENU_CHANNELS.SAVE),
    onMenuStop: menuEventHandler(ipcTypes_1.MENU_CHANNELS.STOP),
    onMenuUpdatePatch: menuEventHandler(ipcTypes_1.MENU_CHANNELS.UPDATE_PATCH),
    onMenuOpenWorkspace: menuEventHandler(ipcTypes_1.MENU_CHANNELS.OPEN_WORKSPACE),
    onMenuCloseBuffer: menuEventHandler(ipcTypes_1.MENU_CHANNELS.CLOSE_BUFFER),
    onMenuToggleRecording: menuEventHandler(ipcTypes_1.MENU_CHANNELS.TOGGLE_RECORDING),
    onMenuOpenSettings: menuEventHandler(ipcTypes_1.MENU_CHANNELS.OPEN_SETTINGS),
    // Programmatically trigger a menu action (for Monaco keybindings on Windows)
    triggerMenuAction: (action) => {
        const channel = ipcTypes_1.MENU_CHANNELS[action];
        if (channel) {
            // Emit the event locally so registered listeners receive it
            renderer_1.ipcRenderer.emit(channel, { sender: renderer_1.ipcRenderer });
        }
    },
    // UI operations
    showContextMenu: (options) => invokeIPC('SHOW_CONTEXT_MENU', options),
    onContextMenuCommand: menuEventHandler(ipcTypes_1.IPC_CHANNELS.ON_CONTEXT_MENU_COMMAND),
    showUnsavedChangesDialog: (fileName) => invokeIPC('SHOW_UNSAVED_CHANGES_DIALOG', fileName),
    // Config operations
    config: {
        getPath: () => invokeIPC('CONFIG_GET_PATH'),
        read: () => invokeIPC('CONFIG_READ'),
        write: (config) => invokeIPC('CONFIG_WRITE', config),
        onChange: menuEventHandler(ipcTypes_1.IPC_CHANNELS.CONFIG_ON_CHANGE),
    },
    // Main process log forwarding
    onMainLog: menuEventHandler(ipcTypes_1.IPC_CHANNELS.MAIN_LOG),
};
// Expose the API to the renderer process
renderer_1.contextBridge.exposeInMainWorld('electronAPI', electronAPI);
function menuEventHandler(channel) {
    return (callback) => {
        const subscription = (_event, ...args) => callback(...args);
        renderer_1.ipcRenderer.on(channel, subscription);
        return () => renderer_1.ipcRenderer.removeListener(channel, subscription);
    };
}
//# sourceMappingURL=preload.js.map