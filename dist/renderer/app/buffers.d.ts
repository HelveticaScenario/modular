import type { EditorBuffer } from '../types/editor';
export declare const DEFAULT_PATCH = "// Simple 440 Hz sine wave\nsine('a3').out();\n";
export declare const readUnsavedBuffers: () => EditorBuffer[];
export declare const saveUnsavedBuffers: (buffers: EditorBuffer[]) => void;
export declare const getBufferId: (buffer: EditorBuffer) => string;
export declare const formatBufferLabel: (buffer: EditorBuffer) => string;
export declare const normalizeFileName: (name: string) => string;
