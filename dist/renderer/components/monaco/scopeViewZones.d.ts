import type { ScopeView } from '../../types/editor';
import type { editor } from 'monaco-editor';
import type { Monaco } from '../../hooks/useCustomMonaco';
type ScopeViewZoneParams = {
    editor: editor.IStandaloneCodeEditor;
    monaco: Monaco;
    views: ScopeView[];
    onRegisterScopeCanvas?: (key: string, canvas: HTMLCanvasElement) => void;
    onUnregisterScopeCanvas?: (key: string) => void;
};
export declare function createScopeViewZones({ editor, monaco, views, onRegisterScopeCanvas, onUnregisterScopeCanvas, }: ScopeViewZoneParams): () => void;
export {};
