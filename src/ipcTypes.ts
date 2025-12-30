/**
 * Type-safe IPC channel definitions for @modular/core
 * 
 * This file defines all IPC channels with their request/response types.
 * Shared between main and renderer processes.
 */

import type {
    ModuleSchema,
    ScopeItem,
    PatchGraph,
    ApplyPatchError,
    AudioThreadHealthSnapshot,
    getSchemas,
    Synthesizer
} from '@modular/core';

/**
 * IPC Channel names - centralized to avoid typos
 */
export const IPC_CHANNELS = {
    // Schema operations
    GET_SCHEMAS: 'modular:get-schemas',

    // Synthesizer operations
    SYNTH_GET_SAMPLE_RATE: 'modular:synth:get-sample-rate',
    SYNTH_GET_CHANNELS: 'modular:synth:get-channels',
    SYNTH_GET_SCOPES: 'modular:synth:get-scopes',
    SYNTH_ADD_SCOPE: 'modular:synth:add-scope',
    SYNTH_REMOVE_SCOPE: 'modular:synth:remove-scope',
    SYNTH_UPDATE_PATCH: 'modular:synth:update-patch',
    SYNTH_START_RECORDING: 'modular:synth:start-recording',
    SYNTH_STOP_RECORDING: 'modular:synth:stop-recording',
    SYNTH_IS_RECORDING: 'modular:synth:is-recording',
    SYNTH_GET_HEALTH: 'modular:synth:get-health',
    SYNTH_STOP: 'modular:synth:stop',
    SYNTH_IS_STOPPED: 'modular:synth:is-stopped',
} as const;

/**
 * Type-safe request/response pairs for each IPC channel
 */
export interface IPCHandlers {
    // Schema operations
    [IPC_CHANNELS.GET_SCHEMAS]: typeof getSchemas;

    // Synthesizer operations
    [IPC_CHANNELS.SYNTH_GET_SAMPLE_RATE]: typeof Synthesizer.prototype.sampleRate;

    [IPC_CHANNELS.SYNTH_GET_CHANNELS]: typeof Synthesizer.prototype.channels;

    [IPC_CHANNELS.SYNTH_GET_SCOPES]: typeof Synthesizer.prototype.getScopes;

    [IPC_CHANNELS.SYNTH_ADD_SCOPE]: typeof Synthesizer.prototype.addScope;

    [IPC_CHANNELS.SYNTH_REMOVE_SCOPE]: typeof Synthesizer.prototype.removeScope;

    [IPC_CHANNELS.SYNTH_UPDATE_PATCH]: typeof Synthesizer.prototype.updatePatch;

    [IPC_CHANNELS.SYNTH_START_RECORDING]: typeof Synthesizer.prototype.startRecording;

    [IPC_CHANNELS.SYNTH_STOP_RECORDING]: typeof Synthesizer.prototype.stopRecording;

    [IPC_CHANNELS.SYNTH_IS_RECORDING]: typeof Synthesizer.prototype.isRecording;

    [IPC_CHANNELS.SYNTH_GET_HEALTH]: typeof Synthesizer.prototype.getHealth;

    [IPC_CHANNELS.SYNTH_STOP]: typeof Synthesizer.prototype.stop;

    [IPC_CHANNELS.SYNTH_IS_STOPPED]: typeof Synthesizer.prototype.isStopped;
}

/**
 * Type helper to extract request type for a channel
 */
export type IPCRequest<T extends keyof IPCHandlers> = Parameters<IPCHandlers[T]>;

/**
 * Type helper to extract response type for a channel
 */
export type IPCResponse<T extends keyof IPCHandlers> = ReturnType<IPCHandlers[T]>;

export type Promisify<T extends (...args: any) => any> = (...args: Parameters<T>) => Promise<ReturnType<T>>;
