import { useState } from 'react';
import './FileExplorer.css';
import type { FileTreeEntry } from '../ipcTypes';
import type { EditorBuffer } from '../App';

interface FileExplorerProps {
    workspaceRoot: string | null;
    fileTree: FileTreeEntry[];
    buffers: EditorBuffer[];
    activeBufferId?: string;
    runningBufferId: string | null;
    formatLabel: (buffer: EditorBuffer) => string;
    onSelectBuffer: (bufferId: string) => void;
    onOpenFile: (relPath: string) => void;
    onCreateFile: () => void;
    onSaveFile: () => void;
    onRenameFile: () => void;
    onDeleteFile: () => void;
    onCloseBuffer: (bufferId: string) => void;
    onSelectWorkspace: () => void;
    onRefreshTree: () => void;
}

export const SCRATCH_FILE = '__scratch__.mjs';

const getBufferId = (buffer: EditorBuffer): string => {
    return buffer.kind === 'file' ? buffer.filePath : buffer.id;
};

function TreeNode({
    entry,
    onOpenFile,
}: {
    entry: FileTreeEntry;
    onOpenFile: (relPath: string) => void;
}) {
    const [expanded, setExpanded] = useState(true);

    if (entry.type === 'file') {
        return (
            <li className="tree-file" onClick={() => onOpenFile(entry.path)}>
                <span className="file-icon">📄</span>
                <span className="file-name">{entry.name}</span>
            </li>
        );
    }

    return (
        <li className="tree-folder">
            <div
                className="folder-header"
                onClick={() => setExpanded(!expanded)}
            >
                <span className="folder-icon">{expanded ? '📂' : '📁'}</span>
                <span className="folder-name">{entry.name}</span>
            </div>
            {expanded && entry.children && entry.children.length > 0 && (
                <ul className="tree-children">
                    {entry.children.map((child) => (
                        <TreeNode
                            key={child.path}
                            entry={child}
                            onOpenFile={onOpenFile}
                        />
                    ))}
                </ul>
            )}
        </li>
    );
}

export function FileExplorer({
    workspaceRoot,
    fileTree,
    buffers,
    activeBufferId,
    runningBufferId,
    formatLabel,
    onSelectBuffer,
    onOpenFile,
    onCreateFile,
    onSaveFile,
    onRenameFile,
    onDeleteFile,
    onCloseBuffer,
    onSelectWorkspace,
    onRefreshTree,
}: FileExplorerProps) {
    const activeBuffer = buffers.find((b) => getBufferId(b) === activeBufferId);

    return (
        <div className="file-explorer">
            <div className="file-explorer-header">
                <div className="file-explorer-title">
                    <h3>Explorer</h3>
                </div>
                <div className="file-explorer-actions">
                    <button
                        onClick={onSelectWorkspace}
                        title="Select workspace folder"
                        className="action-button"
                    >
                        {workspaceRoot ? 'Change Folder' : 'Open Folder'}
                    </button>
                    <button
                        onClick={onRefreshTree}
                        title="Refresh file tree"
                        className="action-button"
                        disabled={!workspaceRoot}
                    >
                        ↻
                    </button>
                </div>
            </div>

            <div className="file-sections">
                {/* Open Editors Section */}
                <div className="section">
                    <div className="section-header">
                        <span>Open Editors</span>
                        <button
                            onClick={onCreateFile}
                            title="New untitled file"
                            className="section-action"
                        >
                            +
                        </button>
                    </div>
                    <div className="file-list">
                        {buffers.length === 0 ? (
                            <div className="empty-message">No open files</div>
                        ) : (
                            <ul>
                                {buffers.map((buffer) => {
                                    const bufferId = getBufferId(buffer);
                                    const isActive =
                                        bufferId === activeBufferId;
                                    const isRunning =
                                        bufferId === runningBufferId;
                                    return (
                                        <li
                                            key={bufferId}
                                            className={[
                                                'buffer-item',
                                                isActive ? 'active' : '',
                                                buffer.dirty ? 'dirty' : '',
                                                isRunning ? 'running' : '',
                                            ]
                                                .filter(Boolean)
                                                .join(' ')}
                                            onClick={() =>
                                                onSelectBuffer(bufferId)
                                            }
                                        >
                                            <span className="file-name">
                                                {formatLabel(buffer)}
                                            </span>
                                            {isRunning && (
                                                <span className="running-badge">
                                                    ▶
                                                </span>
                                            )}
                                            {buffer.dirty && (
                                                <span className="dirty-dot">
                                                    ●
                                                </span>
                                            )}
                                            <button
                                                className="close-button"
                                                onClick={(e) => {
                                                    e.stopPropagation();
                                                    onCloseBuffer(bufferId);
                                                }}
                                                title="Close"
                                            >
                                                ×
                                            </button>
                                        </li>
                                    );
                                })}
                            </ul>
                        )}
                    </div>
                </div>

                {/* Current File Actions */}
                {activeBuffer && (
                    <div className="section">
                        <div className="section-header">Current File</div>
                        <div className="action-buttons">
                            <button
                                onClick={onSaveFile}
                                title="Save current file"
                                className="action-button"
                            >
                                Save
                            </button>
                            {activeBuffer.kind === 'file' && (
                                <>
                                    <button
                                        onClick={onRenameFile}
                                        title="Rename current file"
                                        className="action-button"
                                    >
                                        Rename
                                    </button>
                                    <button
                                        onClick={onDeleteFile}
                                        title="Delete current file"
                                        className="action-button danger"
                                    >
                                        Delete
                                    </button>
                                </>
                            )}
                        </div>
                    </div>
                )}

                {/* Workspace Files Tree */}
                {workspaceRoot && (
                    <div className="section">
                        <div className="section-header">Workspace Files</div>
                        <div className="file-tree">
                            {fileTree.length === 0 ? (
                                <div className="empty-message">
                                    No .js files found
                                </div>
                            ) : (
                                <ul className="tree-root">
                                    {fileTree.map((entry) => (
                                        <TreeNode
                                            key={entry.path}
                                            entry={entry}
                                            onOpenFile={onOpenFile}
                                        />
                                    ))}
                                </ul>
                            )}
                        </div>
                    </div>
                )}
            </div>
        </div>
    );
}
