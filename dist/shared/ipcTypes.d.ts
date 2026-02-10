/**
 * Type-safe IPC channel definitions for @modular/core
 *
 * This file defines all IPC channels with their request/response types.
 * Shared between main and renderer processes.
 */
import type { PatchGraph, ApplyPatchError, AudioDeviceInfo, MidiInputInfo, HostInfo, BufferSizeRange, DeviceCacheSnapshot, HostDeviceInfo, CurrentAudioState, AudioConfigOptions, getSchemas, getMiniLeafSpans, Synthesizer } from '@modular/core';
import type { SliderDefinition } from './dsl/sliderTypes';
export type { PatchGraph, ApplyPatchError, AudioDeviceInfo, MidiInputInfo, HostInfo, BufferSizeRange, DeviceCacheSnapshot, HostDeviceInfo, CurrentAudioState, AudioConfigOptions, };
export interface AudioConfig {
    hostId?: string;
    inputDeviceId?: string | null;
    outputDeviceId?: string;
    sampleRate?: number;
    bufferSize?: number;
}
/** Bundled fonts (shipped with the app) */
export type BundledFont = 'Fira Code' | 'JetBrains Mono' | 'Cascadia Code' | 'Source Code Pro' | 'IBM Plex Mono' | 'Hack' | 'Inconsolata' | 'Monaspace Neon' | 'Monaspace Argon' | 'Monaspace Xenon' | 'Monaspace Krypton' | 'Monaspace Radon' | 'Geist Mono' | 'Iosevka' | 'Victor Mono' | 'Roboto Mono' | 'Maple Mono' | 'Commit Mono' | '0xProto' | 'Intel One Mono' | 'Mononoki' | 'Anonymous Pro' | 'Recursive';
/** System fonts (available only if installed on the OS) */
export type SystemFont = 'SF Mono' | 'Monaco' | 'Menlo' | 'Consolas';
export type MonospaceFont = BundledFont | SystemFont;
export interface PrettierConfig {
    singleQuote?: boolean;
    trailingComma?: 'all' | 'es5' | 'none';
    semi?: boolean;
    tabWidth?: number;
    printWidth?: number;
    [key: string]: unknown;
}
export interface AppConfig {
    theme?: string;
    cursorStyle?: 'line' | 'block' | 'underline' | 'line-thin' | 'block-outline' | 'underline-thin';
    font?: MonospaceFont;
    fontLigatures?: boolean;
    fontSize?: number;
    prettier?: PrettierConfig;
    lastOpenedFolder?: string;
    audioConfig?: AudioConfig;
}
/**
 * Log level for main process logs forwarded to renderer
 */
export type MainLogLevel = 'log' | 'info' | 'warn' | 'error' | 'debug';
/**
 * Log entry from main process
 */
export interface MainLogEntry {
    level: MainLogLevel;
    timestamp: number;
    args: unknown[];
}
export interface UpdatePatchResult {
    errors: ApplyPatchError[];
    appliedPatch: PatchGraph;
    moduleIdRemap: Record<string, string>;
}
/**
 * Source location for mapping validation errors back to DSL code.
 */
export interface SourceLocationInfo {
    /** 1-based line number in the DSL source */
    line: number;
    /** 1-based column number in the DSL source */
    column: number;
    /** Whether the module ID was explicitly set by the user */
    idIsExplicit: boolean;
}
/**
 * Result from DSL execution in main process
 */
/**
 * Serialized form of a ResolvedInterpolation for IPC transfer.
 */
export interface SerializedResolvedInterpolation {
    evaluatedStart: number;
    evaluatedLength: number;
    constLiteralSpan: {
        start: number;
        end: number;
    };
    nestedResolutions?: SerializedResolvedInterpolation[];
}
export interface DSLExecuteResult {
    success: boolean;
    errors?: ApplyPatchError[];
    appliedPatch?: PatchGraph;
    moduleIdRemap?: Record<string, string>;
    errorMessage?: string;
    /** Map from module ID to source location for error reporting */
    sourceLocationMap?: Record<string, SourceLocationInfo>;
    /** Interpolation resolutions for template literal const redirects (serialized Map) */
    interpolationResolutions?: Record<string, SerializedResolvedInterpolation[]>;
    /** Slider definitions created by slider() DSL function calls */
    sliders?: SliderDefinition[];
}
/**
 * File system types
 */
export interface FileTreeEntry {
    name: string;
    path: string;
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
export declare const IPC_CHANNELS: {
    readonly GET_SCHEMAS: "modular:get-schemas";
    readonly DSL_EXECUTE: "modular:dsl:execute";
    readonly GET_DSL_LIB_SOURCE: "modular:dsl:get-lib-source";
    readonly SYNTH_GET_SAMPLE_RATE: "modular:synth:get-sample-rate";
    readonly SYNTH_GET_CHANNELS: "modular:synth:get-channels";
    readonly SYNTH_GET_SCOPES: "modular:synth:get-scopes";
    readonly SYNTH_UPDATE_PATCH: "modular:synth:update-patch";
    readonly SYNTH_START_RECORDING: "modular:synth:start-recording";
    readonly SYNTH_STOP_RECORDING: "modular:synth:stop-recording";
    readonly SYNTH_IS_RECORDING: "modular:synth:is-recording";
    readonly SYNTH_GET_HEALTH: "modular:synth:get-health";
    readonly SYNTH_GET_MODULE_STATES: "modular:synth:get-module-states";
    readonly GET_MINI_LEAF_SPANS: "modular:get-mini-leaf-spans";
    readonly SYNTH_STOP: "modular:synth:stop";
    readonly SYNTH_IS_STOPPED: "modular:synth:is-stopped";
    readonly SYNTH_SET_MODULE_PARAM: "modular:synth:set-module-param";
    readonly AUDIO_REFRESH_DEVICE_CACHE: "modular:audio:refresh-device-cache";
    readonly AUDIO_GET_DEVICE_CACHE: "modular:audio:get-device-cache";
    readonly AUDIO_GET_CURRENT_STATE: "modular:audio:get-current-state";
    readonly AUDIO_RECREATE_STREAMS: "modular:audio:recreate-streams";
    readonly AUDIO_REFRESH_DEVICE_LIST: "modular:audio:refresh-device-list";
    readonly AUDIO_LIST_HOSTS: "modular:audio:list-hosts";
    readonly AUDIO_LIST_OUTPUT_DEVICES: "modular:audio:list-output-devices";
    readonly AUDIO_LIST_INPUT_DEVICES: "modular:audio:list-input-devices";
    readonly AUDIO_GET_OUTPUT_DEVICE: "modular:audio:get-output-device";
    readonly AUDIO_GET_INPUT_DEVICE: "modular:audio:get-input-device";
    readonly AUDIO_SET_OUTPUT_DEVICE: "modular:audio:set-output-device";
    readonly AUDIO_SET_INPUT_DEVICE: "modular:audio:set-input-device";
    readonly AUDIO_GET_INPUT_CHANNELS: "modular:audio:get-input-channels";
    readonly AUDIO_FALLBACK_WARNING: "modular:audio:fallback-warning";
    readonly MIDI_LIST_INPUTS: "modular:midi:list-inputs";
    readonly MIDI_GET_INPUT: "modular:midi:get-input";
    readonly MIDI_SET_INPUT: "modular:midi:set-input";
    readonly MIDI_TRY_RECONNECT: "modular:midi:try-reconnect";
    readonly FS_SELECT_WORKSPACE: "modular:fs:select-workspace";
    readonly FS_GET_WORKSPACE: "modular:fs:get-workspace";
    readonly FS_LIST_FILES: "modular:fs:list-files";
    readonly FS_READ_FILE: "modular:fs:read-file";
    readonly FS_WRITE_FILE: "modular:fs:write-file";
    readonly FS_RENAME_FILE: "modular:fs:rename-file";
    readonly FS_DELETE_FILE: "modular:fs:delete-file";
    readonly FS_MOVE_FILE: "modular:fs:move-file";
    readonly FS_CREATE_FOLDER: "modular:fs:create-folder";
    readonly FS_SHOW_SAVE_DIALOG: "modular:fs:show-save-dialog";
    readonly FS_SHOW_INPUT_DIALOG: "modular:fs:show-input-dialog";
    readonly SHOW_CONTEXT_MENU: "ui:show-context-menu";
    readonly ON_CONTEXT_MENU_COMMAND: "ui:on-context-menu-command";
    readonly SHOW_UNSAVED_CHANGES_DIALOG: "ui:show-unsaved-changes-dialog";
    readonly OPEN_HELP_WINDOW: "modular:window:open-help";
    readonly OPEN_HELP_FOR_SYMBOL: "modular:window:open-help-for-symbol";
    readonly CONFIG_GET_PATH: "modular:config:get-path";
    readonly CONFIG_READ: "modular:config:read";
    readonly CONFIG_WRITE: "modular:config:write";
    readonly CONFIG_ON_CHANGE: "modular:config:on-change";
    readonly MAIN_LOG: "modular:main:log";
};
export declare const MENU_CHANNELS: {
    readonly SAVE: "modular:menu:save";
    readonly STOP: "modular:menu:stop";
    readonly UPDATE_PATCH: "modular:menu:update-patch";
    readonly OPEN_WORKSPACE: "modular:menu:open-workspace";
    readonly CLOSE_BUFFER: "modular:menu:close-buffer";
    readonly TOGGLE_RECORDING: "modular:menu:toggle-recording";
    readonly OPEN_SETTINGS: "modular:menu:open-settings";
};
/**
 * Type-safe request/response pairs for each IPC channel
 */
export interface IPCHandlers {
    [IPC_CHANNELS.GET_SCHEMAS]: typeof getSchemas;
    [IPC_CHANNELS.DSL_EXECUTE]: (source: string, sourceId?: string) => DSLExecuteResult;
    [IPC_CHANNELS.GET_DSL_LIB_SOURCE]: () => string;
    [IPC_CHANNELS.SYNTH_GET_SAMPLE_RATE]: typeof Synthesizer.prototype.sampleRate;
    [IPC_CHANNELS.SYNTH_GET_CHANNELS]: typeof Synthesizer.prototype.channels;
    [IPC_CHANNELS.SYNTH_GET_SCOPES]: typeof Synthesizer.prototype.getScopes;
    [IPC_CHANNELS.SYNTH_UPDATE_PATCH]: (patch: PatchGraph, sourceId?: string) => UpdatePatchResult;
    [IPC_CHANNELS.SYNTH_START_RECORDING]: typeof Synthesizer.prototype.startRecording;
    [IPC_CHANNELS.SYNTH_STOP_RECORDING]: typeof Synthesizer.prototype.stopRecording;
    [IPC_CHANNELS.SYNTH_IS_RECORDING]: typeof Synthesizer.prototype.isRecording;
    [IPC_CHANNELS.SYNTH_GET_HEALTH]: typeof Synthesizer.prototype.getHealth;
    [IPC_CHANNELS.SYNTH_GET_MODULE_STATES]: typeof Synthesizer.prototype.getModuleStates;
    [IPC_CHANNELS.GET_MINI_LEAF_SPANS]: typeof getMiniLeafSpans;
    [IPC_CHANNELS.SYNTH_STOP]: typeof Synthesizer.prototype.stop;
    [IPC_CHANNELS.SYNTH_IS_STOPPED]: typeof Synthesizer.prototype.isStopped;
    [IPC_CHANNELS.SYNTH_SET_MODULE_PARAM]: (moduleId: string, moduleType: string, params: object) => void;
    [IPC_CHANNELS.AUDIO_REFRESH_DEVICE_CACHE]: typeof Synthesizer.prototype.refreshDeviceCache;
    [IPC_CHANNELS.AUDIO_GET_DEVICE_CACHE]: typeof Synthesizer.prototype.getDeviceCache;
    [IPC_CHANNELS.AUDIO_GET_CURRENT_STATE]: typeof Synthesizer.prototype.getCurrentAudioState;
    [IPC_CHANNELS.AUDIO_RECREATE_STREAMS]: typeof Synthesizer.prototype.recreateStreams;
    [IPC_CHANNELS.AUDIO_REFRESH_DEVICE_LIST]: typeof Synthesizer.prototype.refreshDeviceList;
    [IPC_CHANNELS.AUDIO_LIST_HOSTS]: typeof Synthesizer.prototype.listAudioHosts;
    [IPC_CHANNELS.AUDIO_LIST_OUTPUT_DEVICES]: typeof Synthesizer.prototype.listAudioOutputDevices;
    [IPC_CHANNELS.AUDIO_LIST_INPUT_DEVICES]: typeof Synthesizer.prototype.listAudioInputDevices;
    [IPC_CHANNELS.AUDIO_GET_OUTPUT_DEVICE]: typeof Synthesizer.prototype.getOutputDeviceId;
    [IPC_CHANNELS.AUDIO_GET_INPUT_DEVICE]: typeof Synthesizer.prototype.getInputDeviceId;
    [IPC_CHANNELS.AUDIO_SET_OUTPUT_DEVICE]: typeof Synthesizer.prototype.setAudioOutputDevice;
    [IPC_CHANNELS.AUDIO_SET_INPUT_DEVICE]: typeof Synthesizer.prototype.setAudioInputDevice;
    [IPC_CHANNELS.AUDIO_GET_INPUT_CHANNELS]: typeof Synthesizer.prototype.inputChannels;
    [IPC_CHANNELS.AUDIO_FALLBACK_WARNING]: (warning: string) => void;
    [IPC_CHANNELS.MIDI_LIST_INPUTS]: typeof Synthesizer.prototype.listMidiInputs;
    [IPC_CHANNELS.MIDI_GET_INPUT]: typeof Synthesizer.prototype.getMidiInputName;
    [IPC_CHANNELS.MIDI_SET_INPUT]: typeof Synthesizer.prototype.setMidiInput;
    [IPC_CHANNELS.MIDI_TRY_RECONNECT]: typeof Synthesizer.prototype.tryReconnectMidi;
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
    [IPC_CHANNELS.SHOW_CONTEXT_MENU]: (options: ContextMenuOptions) => void;
    [IPC_CHANNELS.ON_CONTEXT_MENU_COMMAND]: (action: ContextMenuAction) => void;
    [IPC_CHANNELS.SHOW_UNSAVED_CHANGES_DIALOG]: (fileName: string) => Promise<number>;
    [IPC_CHANNELS.OPEN_HELP_WINDOW]: () => void;
    [IPC_CHANNELS.OPEN_HELP_FOR_SYMBOL]: (symbolType: 'type' | 'module' | 'namespace', symbolName: string) => void;
    [IPC_CHANNELS.CONFIG_GET_PATH]: () => string;
    [IPC_CHANNELS.CONFIG_READ]: () => AppConfig;
    [IPC_CHANNELS.CONFIG_WRITE]: (config: Partial<AppConfig>) => void;
    [IPC_CHANNELS.CONFIG_ON_CHANGE]: (config: AppConfig) => void;
    [IPC_CHANNELS.MAIN_LOG]: (entry: MainLogEntry) => void;
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
