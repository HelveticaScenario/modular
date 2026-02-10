import React from 'react';
import { editor } from 'monaco-editor';
import type { ScopeView } from '../types/editor';
export interface PatchEditorProps {
    value: string;
    currentFile?: string;
    onChange: (value: string) => void;
    editorRef: React.RefObject<editor.IStandaloneCodeEditor | null>;
    scopeViews?: ScopeView[];
    onRegisterScopeCanvas?: (key: string, canvas: HTMLCanvasElement) => void;
    onUnregisterScopeCanvas?: (key: string) => void;
    runningBufferId?: string | null;
}
export declare function MonacoPatchEditor({ value, currentFile, onChange, editorRef, scopeViews, onRegisterScopeCanvas, onUnregisterScopeCanvas, runningBufferId, }: PatchEditorProps): import("react/jsx-runtime").JSX.Element;
