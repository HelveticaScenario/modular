import { IPC_CHANNELS, IPCHandlers, Promisify, MENU_CHANNELS, ContextMenuOptions, ContextMenuAction, AppConfig, DSLExecuteResult, MainLogEntry } from '../shared/ipcTypes';
/**
 * The public API exposed to the renderer process.
 * All methods are type-safe and match the @modular/core interface.
 */
export interface ElectronAPI {
    getSchemas: Promisify<IPCHandlers[typeof IPC_CHANNELS.GET_SCHEMAS]>;
    getMiniLeafSpans: Promisify<IPCHandlers[typeof IPC_CHANNELS.GET_MINI_LEAF_SPANS]>;
    executeDSL: (source: string, sourceId?: string) => Promise<DSLExecuteResult>;
    getDslLibSource: () => Promise<string>;
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
        setModuleParam: Promisify<IPCHandlers[typeof IPC_CHANNELS.SYNTH_SET_MODULE_PARAM]>;
    };
    audio: {
        refreshDeviceCache: Promisify<IPCHandlers[typeof IPC_CHANNELS.AUDIO_REFRESH_DEVICE_CACHE]>;
        getDeviceCache: Promisify<IPCHandlers[typeof IPC_CHANNELS.AUDIO_GET_DEVICE_CACHE]>;
        getCurrentState: Promisify<IPCHandlers[typeof IPC_CHANNELS.AUDIO_GET_CURRENT_STATE]>;
        recreateStreams: Promisify<IPCHandlers[typeof IPC_CHANNELS.AUDIO_RECREATE_STREAMS]>;
        onFallbackWarning: (callback: (warning: string) => void) => () => void;
        refreshDeviceList: Promisify<IPCHandlers[typeof IPC_CHANNELS.AUDIO_REFRESH_DEVICE_LIST]>;
        listHosts: Promisify<IPCHandlers[typeof IPC_CHANNELS.AUDIO_LIST_HOSTS]>;
        listOutputDevices: Promisify<IPCHandlers[typeof IPC_CHANNELS.AUDIO_LIST_OUTPUT_DEVICES]>;
        listInputDevices: Promisify<IPCHandlers[typeof IPC_CHANNELS.AUDIO_LIST_INPUT_DEVICES]>;
        getOutputDevice: Promisify<IPCHandlers[typeof IPC_CHANNELS.AUDIO_GET_OUTPUT_DEVICE]>;
        getInputDevice: Promisify<IPCHandlers[typeof IPC_CHANNELS.AUDIO_GET_INPUT_DEVICE]>;
        setOutputDevice: Promisify<IPCHandlers[typeof IPC_CHANNELS.AUDIO_SET_OUTPUT_DEVICE]>;
        setInputDevice: Promisify<IPCHandlers[typeof IPC_CHANNELS.AUDIO_SET_INPUT_DEVICE]>;
        getInputChannels: Promisify<IPCHandlers[typeof IPC_CHANNELS.AUDIO_GET_INPUT_CHANNELS]>;
    };
    midi: {
        listInputs: Promisify<IPCHandlers[typeof IPC_CHANNELS.MIDI_LIST_INPUTS]>;
        getInput: Promisify<IPCHandlers[typeof IPC_CHANNELS.MIDI_GET_INPUT]>;
        setInput: Promisify<IPCHandlers[typeof IPC_CHANNELS.MIDI_SET_INPUT]>;
        tryReconnect: Promisify<IPCHandlers[typeof IPC_CHANNELS.MIDI_TRY_RECONNECT]>;
    };
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
    onMenuSave: (callback: () => void) => () => void;
    onMenuStop: (callback: () => void) => () => void;
    onMenuUpdatePatch: (callback: () => void) => () => void;
    onMenuOpenWorkspace: (callback: () => void) => () => void;
    onMenuCloseBuffer: (callback: () => void) => () => void;
    onMenuToggleRecording: (callback: () => void) => () => void;
    onMenuOpenSettings: (callback: () => void) => () => void;
    /**
     * Trigger a menu action programmatically (e.g., from Monaco keybindings).
     * This emits the same IPC event that the Electron menu would send.
     */
    triggerMenuAction: (action: keyof typeof MENU_CHANNELS) => void;
    showContextMenu: (options: ContextMenuOptions) => Promise<void>;
    onContextMenuCommand: (callback: (action: ContextMenuAction) => void) => () => void;
    showUnsavedChangesDialog: (fileName: string) => Promise<number>;
    openHelpWindow: () => Promise<void>;
    openHelpForSymbol: (symbolType: 'type' | 'module' | 'namespace', symbolName: string) => Promise<void>;
    onNavigateToSymbol: (callback: (data: {
        symbolType: 'type' | 'module' | 'namespace';
        symbolName: string;
    }) => void) => () => void;
    config: {
        getPath: () => Promise<string>;
        read: () => Promise<AppConfig>;
        write: (config: Partial<AppConfig>) => Promise<void>;
        onChange: (callback: (config: AppConfig) => void) => () => void;
    };
    onMainLog: (callback: (entry: MainLogEntry) => void) => () => void;
}
