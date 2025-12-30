// See the Electron documentation for details on how to use preload scripts:
// https://www.electronjs.org/docs/latest/tutorial/process-model#preload-scripts
import { contextBridge, ipcRenderer } from 'electron/renderer';
import type {
    ModuleSchema,
    ScopeItem,
    PatchGraph,
    ApplyPatchError,
    AudioThreadHealthSnapshot
} from '@modular/core';
import { IPC_CHANNELS, IPCHandlers, IPCRequest, IPCResponse, Promisify } from './ipcTypes';




/**
 * Type-safe wrapper for IPC invoke calls
 */
function invokeIPC<T extends keyof typeof IPC_CHANNELS>(
    channel: T,
    ...args: IPCRequest<typeof IPC_CHANNELS[T]>
): Promise<IPCResponse<typeof IPC_CHANNELS[T]>> {
    return ipcRenderer.invoke(IPC_CHANNELS[channel], ...args);
}
/**
 * The public API exposed to the renderer process.
 * All methods are type-safe and match the @modular/core interface.
 */


export interface ElectronAPI {
    // Schema operations
    getSchemas: Promisify<IPCHandlers[typeof IPC_CHANNELS.GET_SCHEMAS]>;
    // Synthesizer operations
    synthesizer: {
        getSampleRate: Promisify<IPCHandlers[typeof IPC_CHANNELS.SYNTH_GET_SAMPLE_RATE]>;
        getChannels: Promisify<IPCHandlers[typeof IPC_CHANNELS.SYNTH_GET_CHANNELS]>;
        getScopes: Promisify<IPCHandlers[typeof IPC_CHANNELS.SYNTH_GET_SCOPES]>;
        addScope: Promisify<IPCHandlers[typeof IPC_CHANNELS.SYNTH_ADD_SCOPE]>;
        removeScope: Promisify<IPCHandlers[typeof IPC_CHANNELS.SYNTH_REMOVE_SCOPE]>;
        updatePatch: Promisify<IPCHandlers[typeof IPC_CHANNELS.SYNTH_UPDATE_PATCH]>;
        startRecording: Promisify<IPCHandlers[typeof IPC_CHANNELS.SYNTH_START_RECORDING]>;
        stopRecording: Promisify<IPCHandlers[typeof IPC_CHANNELS.SYNTH_STOP_RECORDING]>;
        isRecording: Promisify<IPCHandlers[typeof IPC_CHANNELS.SYNTH_IS_RECORDING]>;
        getHealth: Promisify<IPCHandlers[typeof IPC_CHANNELS.SYNTH_GET_HEALTH]>;
    };
}

const electronAPI: ElectronAPI = {
    // Schema operations
    getSchemas: (...args) =>
        invokeIPC('GET_SCHEMAS', ...args),

    // Synthesizer operations
    synthesizer: {
        getSampleRate: (...args) =>
            invokeIPC('SYNTH_GET_SAMPLE_RATE', ...args),

        getChannels: (...args) =>
            invokeIPC('SYNTH_GET_CHANNELS', ...args),

        getScopes: (...args) =>
            invokeIPC('SYNTH_GET_SCOPES', ...args),

        addScope: (...args) =>
            invokeIPC('SYNTH_ADD_SCOPE', ...args),

        removeScope: (...args) =>
            invokeIPC('SYNTH_REMOVE_SCOPE', ...args),

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
    }
};

// Expose the API to the renderer process
contextBridge.exposeInMainWorld('electronAPI', electronAPI);

