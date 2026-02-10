export type EditorBuffer = {
    kind: 'file';
    id: string;
    filePath: string;
    content: string;
    dirty: boolean;
    isPreview?: boolean;
} | {
    kind: 'untitled';
    id: string;
    content: string;
    dirty: boolean;
    isPreview?: boolean;
};
export type UnsavedBufferSnapshot = {
    kind: 'file';
    id: string;
    filePath: string;
    content: string;
} | {
    kind: 'untitled';
    id: string;
    content: string;
};
export type ScopeView = {
    key: string;
    lineNumber: number;
    file: string;
    range: [number, number];
};
