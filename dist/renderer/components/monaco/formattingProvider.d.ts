import type { Monaco } from '../../hooks/useCustomMonaco';
import type { PrettierConfig } from '../../../shared/ipcTypes';
export declare function registerDslFormattingProvider(monaco: Monaco, userConfig?: PrettierConfig): import("monaco-editor").IDisposable;
