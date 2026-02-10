import './FileExplorer.css';
import type { FileTreeEntry } from '../../shared/ipcTypes';
import type { EditorBuffer } from '../types/editor';
interface FileExplorerProps {
    workspaceRoot: string | null;
    fileTree: FileTreeEntry[];
    buffers: EditorBuffer[];
    activeBufferId?: string;
    runningBufferId: string | null;
    renamingPath: string | null;
    formatLabel: (buffer: EditorBuffer) => string;
    onSelectBuffer: (bufferId: string) => void;
    onOpenFile: (relPath: string, options?: {
        preview?: boolean;
    }) => void;
    onCreateFile: () => void;
    onSaveFile: (id?: string) => void;
    onRenameFile: (id?: string) => void;
    onDeleteFile: (id?: string) => void;
    onCloseBuffer: (bufferId: string) => void;
    onSelectWorkspace: () => void;
    onRefreshTree: () => void;
    onRenameCommit: (path: string, newName: string) => void;
    onRenameCancel: () => void;
    onKeepBuffer: (bufferId: string) => void;
}
export declare const SCRATCH_FILE = "__scratch__.mjs";
export declare function FileExplorer({ workspaceRoot, fileTree, buffers, activeBufferId, runningBufferId, renamingPath, formatLabel, onSelectBuffer, onOpenFile, onCreateFile, onSaveFile, onRenameFile, onDeleteFile, onCloseBuffer, onSelectWorkspace, onRefreshTree, onRenameCommit, onRenameCancel, onKeepBuffer }: FileExplorerProps): import("react/jsx-runtime").JSX.Element;
export {};
