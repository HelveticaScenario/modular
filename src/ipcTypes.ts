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
    parsePattern,
    Synthesizer
} from '@modular/core';

export type { PatchGraph, ApplyPatchError };

export interface AppConfig {
    theme?: string;
    cursorStyle?: 'line' | 'block' | 'underline' | 'line-thin' | 'block-outline' | 'underline-thin';
    lastOpenedFolder?: string;
}

export interface UpdatePatchResult {
    errors: ApplyPatchError[];
    appliedPatch: PatchGraph;
    moduleIdRemap: Record<string, string>;
}

/**
 * File system types
 */
export interface FileTreeEntry {
    name: string;
    path: string; // relative to workspace root
    type: 'file' | 'directory';
    children?: FileTreeEntry[];
}

export interface FSOperationResult {
    success: boolean;
    error?: string;
}

export interface WorkspaceFolder {
    path: string;
}

export interface ContextMenuOptions {
    type: 'file' | 'directory' | 'unknown' | 'untitled';
    path?: string;
    bufferId?: string;
    isWorkspaceFile?: boolean;
    isOpenBuffer?: boolean;
    x?: number;
    y?: number;
}

export type ContextMenuCommand = 'save' | 'rename' | 'delete';

export interface ContextMenuAction {
    command: ContextMenuCommand;
    path?: string;
    bufferId?: string;
}

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
    SYNTH_UPDATE_PATCH: 'modular:synth:update-patch',
    SYNTH_START_RECORDING: 'modular:synth:start-recording',
    SYNTH_STOP_RECORDING: 'modular:synth:stop-recording',
    SYNTH_IS_RECORDING: 'modular:synth:is-recording',
    SYNTH_GET_HEALTH: 'modular:synth:get-health',
    SYNTH_GET_MODULE_STATES: 'modular:synth:get-module-states',
    PARSE_PATTERN: 'modular:parse-pattern',
    SYNTH_STOP: 'modular:synth:stop',
    SYNTH_IS_STOPPED: 'modular:synth:is-stopped',

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

    // Config operations
    CONFIG_GET_PATH: 'modular:config:get-path',
    CONFIG_READ: 'modular:config:read',
    CONFIG_ON_CHANGE: 'modular:config:on-change',
} as const;

export const MENU_CHANNELS = {
    SAVE: 'modular:menu:save',
    STOP: 'modular:menu:stop',
    UPDATE_PATCH: 'modular:menu:update-patch',
    OPEN_WORKSPACE: 'modular:menu:open-workspace',
    CLOSE_BUFFER: 'modular:menu:close-buffer',
    TOGGLE_RECORDING: 'modular:menu:toggle-recording',
    OPEN_SETTINGS: 'modular:menu:open-settings',
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

    [IPC_CHANNELS.SYNTH_UPDATE_PATCH]: (patch: PatchGraph, sourceId?: string) => UpdatePatchResult;

    [IPC_CHANNELS.SYNTH_START_RECORDING]: typeof Synthesizer.prototype.startRecording;

    [IPC_CHANNELS.SYNTH_STOP_RECORDING]: typeof Synthesizer.prototype.stopRecording;

    [IPC_CHANNELS.SYNTH_IS_RECORDING]: typeof Synthesizer.prototype.isRecording;

    [IPC_CHANNELS.SYNTH_GET_HEALTH]: typeof Synthesizer.prototype.getHealth;

    [IPC_CHANNELS.SYNTH_GET_MODULE_STATES]: typeof Synthesizer.prototype.getModuleStates;

    [IPC_CHANNELS.PARSE_PATTERN]: typeof parsePattern;

    [IPC_CHANNELS.SYNTH_STOP]: typeof Synthesizer.prototype.stop;

    [IPC_CHANNELS.SYNTH_IS_STOPPED]: typeof Synthesizer.prototype.isStopped;

    // Filesystem operations (IPC automatically promisifies all handlers)
    [IPC_CHANNELS.FS_SELECT_WORKSPACE]: () => WorkspaceFolder | null;
    [IPC_CHANNELS.FS_GET_WORKSPACE]: () => WorkspaceFolder | null;
    [IPC_CHANNELS.FS_LIST_FILES]: () => FileTreeEntry[];
    [IPC_CHANNELS.FS_READ_FILE]: (filePath: string) => string;
    [IPC_CHANNELS.FS_WRITE_FILE]: (filePath: string, content: string) => FSOperationResult;
    [IPC_CHANNELS.FS_RENAME_FILE]: (oldPath: string, newPath: string) => FSOperationResult;
    [IPC_CHANNELS.FS_DELETE_FILE]: (filePath: string) => Promise<FSOperationResult>;
    [IPC_CHANNELS.FS_MOVE_FILE]: (sourcePath: string, destPath: string) => FSOperationResult;
    [IPC_CHANNELS.FS_CREATE_FOLDER]: (filePath: string) => FSOperationResult;
    [IPC_CHANNELS.FS_SHOW_SAVE_DIALOG]: (defaultPath?: string) => string | null;
    [IPC_CHANNELS.FS_SHOW_INPUT_DIALOG]: (title: string, defaultValue?: string) => string | null;

    // UI operations
    [IPC_CHANNELS.SHOW_CONTEXT_MENU]: (options: ContextMenuOptions) => void;
    [IPC_CHANNELS.ON_CONTEXT_MENU_COMMAND]: (action: ContextMenuAction) => void;
    [IPC_CHANNELS.SHOW_UNSAVED_CHANGES_DIALOG]: (fileName: string) => Promise<number>;

    // Window operations
    [IPC_CHANNELS.OPEN_HELP_WINDOW]: () => void;

    // Config operations
    [IPC_CHANNELS.CONFIG_GET_PATH]: () => string;
    [IPC_CHANNELS.CONFIG_READ]: () => AppConfig;
    [IPC_CHANNELS.CONFIG_ON_CHANGE]: (config: AppConfig) => void;
}

/**
 * Type helper to extract request type for a channel
 */
export type IPCRequest<T extends keyof IPCHandlers> = Parameters<IPCHandlers[T]>;

/**
 * Type helper to extract response type for a channel
 */
export type IPCResponse<T extends keyof IPCHandlers> = ReturnType<Promisify<IPCHandlers[T]>>;

export type Promisify<T> = T extends (...args: any[]) => Promise<any> ? T : T extends (...args: infer P) => infer R ? (...args: P) => Promise<R> : never;
