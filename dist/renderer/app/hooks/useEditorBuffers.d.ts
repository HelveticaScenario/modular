import type { EditorBuffer } from '../../types/editor';
type UseEditorBuffersParams = {
    workspaceRoot: string | null;
    refreshFileTree: () => Promise<void>;
};
export declare function useEditorBuffers({ workspaceRoot, refreshFileTree, }: UseEditorBuffersParams): {
    buffers: EditorBuffer[];
    setBuffers: import("react").Dispatch<import("react").SetStateAction<EditorBuffer[]>>;
    activeBufferId: string | undefined;
    setActiveBufferId: import("react").Dispatch<import("react").SetStateAction<string | undefined>>;
    patchCode: string;
    handlePatchChange: (value: string) => void;
    openFile: (relPath: string, options?: {
        preview?: boolean;
    }) => Promise<void>;
    createUntitledFile: () => void;
    saveFile: (targetId?: string) => Promise<void>;
    renameFile: (targetIdOrPath?: string) => Promise<void>;
    deleteFile: (targetIdOrPath?: string) => Promise<void>;
    closeBuffer: (bufferId: string) => Promise<void>;
    keepBuffer: (bufferId: string) => void;
    renamingPath: string | null;
    setRenamingPath: import("react").Dispatch<import("react").SetStateAction<string | null>>;
    handleRenameCommit: (oldPath: string, newName: string) => Promise<void>;
    formatFileLabel: (buffer: EditorBuffer) => string;
};
export {};
