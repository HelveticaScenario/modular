import type { Monaco } from '../../hooks/useCustomMonaco';
export declare function applyDslLibToMonaco(monaco: Monaco, libSource: string): {
    extraLib?: undefined;
    extraLibModel?: undefined;
} | {
    extraLib: import("monaco-editor").IDisposable;
    extraLibModel: import("monaco-editor").editor.ITextModel;
};
export declare function formatPath(currentFile: string): string;
