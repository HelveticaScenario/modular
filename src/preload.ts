// See the Electron documentation for details on how to use preload scripts:
// https://www.electronjs.org/docs/latest/tutorial/process-model#preload-scripts
import { contextBridge, ipcRenderer } from 'electron/renderer';
import { IPC_CHANNELS, IPCHandlers, IPCRequest, IPCResponse, Promisify, MENU_CHANNELS, ContextMenuOptions, ContextMenuAction } from './ipcTypes';




/**
 * Type-safe wrapper for IPC invoke calls
 */
function invokeIPC<T extends keyof typeof IPC_CHANNELS>(
    channel: T,
    ...args: IPCRequest<typeof IPC_CHANNELS[T]>
): IPCResponse<typeof IPC_CHANNELS[T]> {
    // @ts-ignore - TypeScript is having trouble inferring the return type here
    return ipcRenderer.invoke(IPC_CHANNELS[channel], ...args);
}

/**
 * The public API exposed to the renderer process.
 * All methods are type-safe and match the @modular/core interface.
 */


export interface ElectronAPI {
    // Schema operations
    getSchemas: Promisify<IPCHandlers[typeof IPC_CHANNELS.GET_SCHEMAS]>;
    parsePattern: Promisify<IPCHandlers[typeof IPC_CHANNELS.PARSE_PATTERN]>;
    // Synthesizer operations
    synthesizer: {
        getSampleRate: Promisify<IPCHandlers[typeof IPC_CHANNELS.SYNTH_GET_SAMPLE_RATE]>;
        getChannels: Promisify<IPCHandlers[typeof IPC_CHANNELS.SYNTH_GET_CHANNELS]>;
        getScopes: Promisify<IPCHandlers[typeof IPC_CHANNELS.SYNTH_GET_SCOPES]>;
        getModuleStates: Promisify<IPCHandlers[typeof IPC_CHANNELS.SYNTH_GET_MODULE_STATES]>;
        updatePatch: Promisify<IPCHandlers[typeof IPC_CHANNELS.SYNTH_UPDATE_PATCH]>;
        startRecording: Promisify<IPCHandlers[typeof IPC_CHANNELS.SYNTH_START_RECORDING]>;
        stopRecording: Promisify<IPCHandlers[typeof IPC_CHANNELS.SYNTH_STOP_RECORDING]>;
        isRecording: Promisify<IPCHandlers[typeof IPC_CHANNELS.SYNTH_IS_RECORDING]>;
        getHealth: Promisify<IPCHandlers[typeof IPC_CHANNELS.SYNTH_GET_HEALTH]>;
        stop: Promisify<IPCHandlers[typeof IPC_CHANNELS.SYNTH_STOP]>;
        isStopped: Promisify<IPCHandlers[typeof IPC_CHANNELS.SYNTH_IS_STOPPED]>;
    };
    // Filesystem operations
    filesystem: {
        selectWorkspace: Promisify<IPCHandlers[typeof IPC_CHANNELS.FS_SELECT_WORKSPACE]>;
        getWorkspace: Promisify<IPCHandlers[typeof IPC_CHANNELS.FS_GET_WORKSPACE]>;
        listFiles: Promisify<IPCHandlers[typeof IPC_CHANNELS.FS_LIST_FILES]>;
        readFile: Promisify<IPCHandlers[typeof IPC_CHANNELS.FS_READ_FILE]>;
        writeFile: Promisify<IPCHandlers[typeof IPC_CHANNELS.FS_WRITE_FILE]>;
        renameFile: Promisify<IPCHandlers[typeof IPC_CHANNELS.FS_RENAME_FILE]>;
        deleteFile: Promisify<IPCHandlers[typeof IPC_CHANNELS.FS_DELETE_FILE]>;
        moveFile: Promisify<IPCHandlers[typeof IPC_CHANNELS.FS_MOVE_FILE]>;
        createFolder: Promisify<IPCHandlers[typeof IPC_CHANNELS.FS_CREATE_FOLDER]>;
        showSaveDialog: Promisify<IPCHandlers[typeof IPC_CHANNELS.FS_SHOW_SAVE_DIALOG]>;
        showInputDialog: Promisify<IPCHandlers[typeof IPC_CHANNELS.FS_SHOW_INPUT_DIALOG]>;
    };
    // Menu events
    onMenuSave: (callback: () => void) => () => void;
    onMenuStop: (callback: () => void) => () => void;
    onMenuUpdatePatch: (callback: () => void) => () => void;
    onMenuOpenWorkspace: (callback: () => void) => () => void;
    // UI operations
    showContextMenu: (options: ContextMenuOptions) => Promise<void>;
    onContextMenuCommand: (callback: (action: ContextMenuAction) => void) => () => void;

    // Window operations
    openHelpWindow: () => Promise<void>;
}

const electronAPI: ElectronAPI = {
    // Schema operations
    getSchemas: (...args) =>
        invokeIPC('GET_SCHEMAS', ...args),
    parsePattern: (...args) =>
        invokeIPC('PARSE_PATTERN', ...args),

    // Window operations
    openHelpWindow: () => invokeIPC('OPEN_HELP_WINDOW'),

    // Synthesizer operations
    synthesizer: {
        getSampleRate: (...args) =>
            invokeIPC('SYNTH_GET_SAMPLE_RATE', ...args),

        getChannels: (...args) =>
            invokeIPC('SYNTH_GET_CHANNELS', ...args),

        getScopes: (...args) =>
            invokeIPC('SYNTH_GET_SCOPES', ...args),

        getModuleStates: (...args) =>
            invokeIPC('SYNTH_GET_MODULE_STATES', ...args),

        updatePatch: (...args) =>
            invokeIPC('SYNTH_UPDATE_PATCH', ...args),

        startRecording: (...args) =>
            invokeIPC('SYNTH_START_RECORDING', ...args),

        stopRecording: (...args) =>
            invokeIPC('SYNTH_STOP_RECORDING', ...args),

        isRecording: (...args) =>
            invokeIPC('SYNTH_IS_RECORDING', ...args),

        getHealth: (...args) =>
            invokeIPC('SYNTH_GET_HEALTH', ...args),

        stop: (...args) =>
            invokeIPC('SYNTH_STOP', ...args),

        isStopped: (...args) =>
            invokeIPC('SYNTH_IS_STOPPED', ...args),
    },

    // Filesystem operations
    filesystem: {
        selectWorkspace: (...args) =>
            invokeIPC('FS_SELECT_WORKSPACE', ...args),

        getWorkspace: (...args) =>
            invokeIPC('FS_GET_WORKSPACE', ...args),

        listFiles: (...args) =>
            invokeIPC('FS_LIST_FILES', ...args),

        readFile: (...args) =>
            invokeIPC('FS_READ_FILE', ...args),

        writeFile: (...args) =>
            invokeIPC('FS_WRITE_FILE', ...args),

        renameFile: (...args) =>
            invokeIPC('FS_RENAME_FILE', ...args),

        deleteFile: (...args) =>
            invokeIPC('FS_DELETE_FILE', ...args),

        moveFile: (...args) =>
            invokeIPC('FS_MOVE_FILE', ...args),

        createFolder: (...args) =>
            invokeIPC('FS_CREATE_FOLDER', ...args),

        showSaveDialog: (...args) =>
            invokeIPC('FS_SHOW_SAVE_DIALOG', ...args),

        showInputDialog: (...args) =>
            invokeIPC('FS_SHOW_INPUT_DIALOG', ...args),
    },

    // Menu events
    onMenuSave: (callback) => {
        const subscription = (_event: any) => callback();
        ipcRenderer.on(MENU_CHANNELS.SAVE, subscription);
        return () => ipcRenderer.removeListener(MENU_CHANNELS.SAVE, subscription);
    },
    onMenuStop: (callback) => {
        const subscription = (_event: any) => callback();
        ipcRenderer.on(MENU_CHANNELS.STOP, subscription);
        return () => ipcRenderer.removeListener(MENU_CHANNELS.STOP, subscription);
    },
    onMenuUpdatePatch: (callback) => {
        const subscription = (_event: any) => callback();
        ipcRenderer.on(MENU_CHANNELS.UPDATE_PATCH, subscription);
        return () => ipcRenderer.removeListener(MENU_CHANNELS.UPDATE_PATCH, subscription);
    },
    onMenuOpenWorkspace: (callback) => {
        const subscription = (_event: any) => callback();
        ipcRenderer.on(MENU_CHANNELS.OPEN_WORKSPACE, subscription);
        return () => ipcRenderer.removeListener(MENU_CHANNELS.OPEN_WORKSPACE, subscription);
    },

    // UI operations
    showContextMenu: (options) => invokeIPC('SHOW_CONTEXT_MENU', options),
    onContextMenuCommand: (callback) => {
        const subscription = (_event: any, action: ContextMenuAction) => callback(action);
        ipcRenderer.on(IPC_CHANNELS.ON_CONTEXT_MENU_COMMAND, subscription);
        return () => ipcRenderer.removeListener(IPC_CHANNELS.ON_CONTEXT_MENU_COMMAND, subscription);
    }
};

// Expose the API to the renderer process
contextBridge.exposeInMainWorld('electronAPI', electronAPI);

