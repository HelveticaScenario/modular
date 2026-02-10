"use strict";
/**
 * Type-safe IPC channel definitions for @modular/core
 *
 * This file defines all IPC channels with their request/response types.
 * Shared between main and renderer processes.
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.MENU_CHANNELS = exports.IPC_CHANNELS = void 0;
/**
 * IPC Channel names - centralized to avoid typos
 */
exports.IPC_CHANNELS = {
    // Schema operations
    GET_SCHEMAS: 'modular:get-schemas',
    // DSL operations
    DSL_EXECUTE: 'modular:dsl:execute',
    GET_DSL_LIB_SOURCE: 'modular:dsl:get-lib-source',
    // Synthesizer operations
    SYNTH_GET_SAMPLE_RATE: 'modular:synth:get-sample-rate',
    SYNTH_GET_CHANNELS: 'modular:synth:get-channels',
    SYNTH_GET_SCOPES: 'modular:synth:get-scopes',
    SYNTH_UPDATE_PATCH: 'modular:synth:update-patch',
    SYNTH_START_RECORDING: 'modular:synth:start-recording',
    SYNTH_STOP_RECORDING: 'modular:synth:stop-recording',
    SYNTH_IS_RECORDING: 'modular:synth:is-recording',
    SYNTH_GET_HEALTH: 'modular:synth:get-health',
    SYNTH_GET_MODULE_STATES: 'modular:synth:get-module-states',
    GET_MINI_LEAF_SPANS: 'modular:get-mini-leaf-spans',
    SYNTH_STOP: 'modular:synth:stop',
    SYNTH_IS_STOPPED: 'modular:synth:is-stopped',
    SYNTH_SET_MODULE_PARAM: 'modular:synth:set-module-param',
    // Audio device operations
    AUDIO_REFRESH_DEVICE_CACHE: 'modular:audio:refresh-device-cache',
    AUDIO_GET_DEVICE_CACHE: 'modular:audio:get-device-cache',
    AUDIO_GET_CURRENT_STATE: 'modular:audio:get-current-state',
    AUDIO_RECREATE_STREAMS: 'modular:audio:recreate-streams',
    // Legacy (kept for backward compatibility)
    AUDIO_REFRESH_DEVICE_LIST: 'modular:audio:refresh-device-list',
    AUDIO_LIST_HOSTS: 'modular:audio:list-hosts',
    AUDIO_LIST_OUTPUT_DEVICES: 'modular:audio:list-output-devices',
    AUDIO_LIST_INPUT_DEVICES: 'modular:audio:list-input-devices',
    AUDIO_GET_OUTPUT_DEVICE: 'modular:audio:get-output-device',
    AUDIO_GET_INPUT_DEVICE: 'modular:audio:get-input-device',
    AUDIO_SET_OUTPUT_DEVICE: 'modular:audio:set-output-device',
    AUDIO_SET_INPUT_DEVICE: 'modular:audio:set-input-device',
    AUDIO_GET_INPUT_CHANNELS: 'modular:audio:get-input-channels',
    // Fallback warning notification
    AUDIO_FALLBACK_WARNING: 'modular:audio:fallback-warning',
    // MIDI device operations
    MIDI_LIST_INPUTS: 'modular:midi:list-inputs',
    MIDI_GET_INPUT: 'modular:midi:get-input',
    MIDI_SET_INPUT: 'modular:midi:set-input',
    MIDI_TRY_RECONNECT: 'modular:midi:try-reconnect',
    // Filesystem operations
    FS_SELECT_WORKSPACE: 'modular:fs:select-workspace',
    FS_GET_WORKSPACE: 'modular:fs:get-workspace',
    FS_LIST_FILES: 'modular:fs:list-files',
    FS_READ_FILE: 'modular:fs:read-file',
    FS_WRITE_FILE: 'modular:fs:write-file',
    FS_RENAME_FILE: 'modular:fs:rename-file',
    FS_DELETE_FILE: 'modular:fs:delete-file',
    FS_MOVE_FILE: 'modular:fs:move-file',
    FS_CREATE_FOLDER: 'modular:fs:create-folder',
    FS_SHOW_SAVE_DIALOG: 'modular:fs:show-save-dialog',
    FS_SHOW_INPUT_DIALOG: 'modular:fs:show-input-dialog',
    // UI operations
    SHOW_CONTEXT_MENU: 'ui:show-context-menu',
    ON_CONTEXT_MENU_COMMAND: 'ui:on-context-menu-command',
    SHOW_UNSAVED_CHANGES_DIALOG: 'ui:show-unsaved-changes-dialog',
    // Window operations
    OPEN_HELP_WINDOW: 'modular:window:open-help',
    OPEN_HELP_FOR_SYMBOL: 'modular:window:open-help-for-symbol',
    // Config operations
    CONFIG_GET_PATH: 'modular:config:get-path',
    CONFIG_READ: 'modular:config:read',
    CONFIG_WRITE: 'modular:config:write',
    CONFIG_ON_CHANGE: 'modular:config:on-change',
    // Main process logging
    MAIN_LOG: 'modular:main:log',
};
exports.MENU_CHANNELS = {
    SAVE: 'modular:menu:save',
    STOP: 'modular:menu:stop',
    UPDATE_PATCH: 'modular:menu:update-patch',
    OPEN_WORKSPACE: 'modular:menu:open-workspace',
    CLOSE_BUFFER: 'modular:menu:close-buffer',
    TOGGLE_RECORDING: 'modular:menu:toggle-recording',
    OPEN_SETTINGS: 'modular:menu:open-settings',
};
//# sourceMappingURL=ipcTypes.js.map