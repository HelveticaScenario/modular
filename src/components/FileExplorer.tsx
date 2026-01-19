import { useState, useEffect, useRef } from 'react';
import './FileExplorer.css';
import type { FileTreeEntry } from '../ipcTypes';
import type { EditorBuffer } from '../types/editor';
import { getBufferId } from '../app/buffers';
import electronAPI from '../electronAPI';

interface FileExplorerProps {
    workspaceRoot: string | null;
    fileTree: FileTreeEntry[];
    buffers: EditorBuffer[];
    activeBufferId?: string;
    runningBufferId: string | null;
    renamingPath: string | null;
    formatLabel: (buffer: EditorBuffer) => string;
    onSelectBuffer: (bufferId: string) => void;
    onOpenFile: (relPath: string, options?: { preview?: boolean }) => void;
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

export const SCRATCH_FILE = '__scratch__.mjs';

function TreeNode({
    entry,
    onOpenFile,
    onContextMenu,
    renamingPath,
    onRenameCommit,
    onRenameCancel,
}: {
    entry: FileTreeEntry;
    onOpenFile: (relPath: string, options?: { preview?: boolean }) => void;
    onContextMenu: (e: React.MouseEvent, entry: FileTreeEntry) => void;
    renamingPath: string | null;
    onRenameCommit: (path: string, newName: string) => void;
    onRenameCancel: () => void;
}) {
    const [expanded, setExpanded] = useState(true);
    const [editName, setEditName] = useState(entry.name);
    const inputRef = useRef<HTMLInputElement>(null);

    const isRenaming = renamingPath === entry.path;

    useEffect(() => {
        if (isRenaming && inputRef.current) {
            setEditName(entry.name);
            inputRef.current.focus();
            const name = entry.name;
            const lastDotIndex = name.lastIndexOf('.');
            if (lastDotIndex !== -1) {
                inputRef.current.setSelectionRange(0, lastDotIndex);
            } else {
                inputRef.current.select();
            }
        }
    }, [isRenaming, entry.name]);

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === 'Enter') {
            e.stopPropagation();
            onRenameCommit(entry.path, editName);
        } else if (e.key === 'Escape') {
            e.stopPropagation();
            onRenameCancel();
        }
    };

    const handleSingleClick = () => {
        if (!isRenaming) {
            onOpenFile(entry.path, { preview: true });
        }
    };

    const handleDoubleClick = () => {
        if (!isRenaming) {
            onOpenFile(entry.path, { preview: false });
        }
    };

    if (entry.type === 'file') {
        return (
            <li
                className="tree-file"
                onClick={handleSingleClick}
                onDoubleClick={handleDoubleClick}
                onContextMenu={(e) => onContextMenu(e, entry)}
            >
                <span className="file-icon">📄</span>
                {isRenaming ? (
                    <input
                        ref={inputRef}
                        type="text"
                        className="rename-input"
                        value={editName}
                        onChange={(e) => setEditName(e.target.value)}
                        onKeyDown={handleKeyDown}
                        // onBlur={handleBlur} // Blur handling can be tricky with specific commit logic, skipping for now to avoid accidental commits while debugging
                        onBlur={onRenameCancel} // For now cancel on blur to be safe, or just keep focus
                    />
                ) : (
                    <span className="file-name">{entry.name}</span>
                )}
            </li>
        );
    }

    return (
        <li className="tree-folder">
            <div
                className="folder-header"
                onClick={() => !isRenaming && setExpanded(!expanded)}
                onContextMenu={(e) => onContextMenu(e, entry)}
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
                            onContextMenu={onContextMenu}
                            renamingPath={renamingPath}
                            onRenameCommit={onRenameCommit}
                            onRenameCancel={onRenameCancel}
                        />
                    ))}
                </ul>
            )}
        </li>
    );
}

function BufferItem({
    buffer,
    isActive,
    isRunning,
    renamingPath,
    formatLabel,
    onSelectBuffer,
    onContextMenu,
    onCloseBuffer,
    onRenameCommit,
    onRenameCancel,
    onKeepBuffer
}: {
    buffer: EditorBuffer;
    isActive: boolean;
    isRunning: boolean;
    renamingPath: string | null;
    formatLabel: (buffer: EditorBuffer) => string;
    onSelectBuffer: (id: string) => void;
    onContextMenu: (e: React.MouseEvent, id: string) => void;
    onCloseBuffer: (id: string) => void;
    onRenameCommit: (path: string, newName: string) => void;
    onRenameCancel: () => void;
    onKeepBuffer: (id: string) => void;
}) {
    const bufferId = getBufferId(buffer);
    const [editName, setEditName] = useState(formatLabel(buffer));
    const inputRef = useRef<HTMLInputElement>(null);
    const isRenaming = buffer.kind === 'file' && renamingPath === buffer.filePath;

    useEffect(() => {
        if (isRenaming && inputRef.current) {
            setEditName(formatLabel(buffer));
            inputRef.current.focus();
            const name = formatLabel(buffer);
            const lastDotIndex = name.lastIndexOf('.');
            if (lastDotIndex !== -1) {
                inputRef.current.setSelectionRange(0, lastDotIndex);
            } else {
                inputRef.current.select();
            }
        }
    }, [isRenaming, buffer, formatLabel]);

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === 'Enter') {
            e.stopPropagation();
            if (buffer.kind === 'file') {
                onRenameCommit(buffer.filePath, editName);
            }
        } else if (e.key === 'Escape') {
            e.stopPropagation();
            onRenameCancel();
        }
    };

    return (
        <li
            className={[
                'buffer-item',
                isActive ? 'active' : '',
                buffer.dirty ? 'dirty' : '',
                isRunning ? 'running' : '',
                buffer.isPreview ? 'preview' : '',
            ]
                .filter(Boolean)
                .join(' ')}
            onClick={() => !isRenaming && onSelectBuffer(bufferId)}
            onDoubleClick={() => !isRenaming && onKeepBuffer(bufferId)}
            onContextMenu={(e) => onContextMenu(e, bufferId)}
        >
            {isRenaming ? (
                <input
                    ref={inputRef}
                    type="text"
                    className="rename-input"
                    value={editName}
                    onChange={(e) => setEditName(e.target.value)}
                    onKeyDown={handleKeyDown}
                    onBlur={onRenameCancel}
                    onClick={(e) => e.stopPropagation()}
                />
            ) : (
                <span className="file-name">
                    {formatLabel(buffer)}
                </span>
            )}
            {!isRenaming && isRunning && (
                <span className="running-badge">
                    ▶
                </span>
            )}
            {!isRenaming && buffer.dirty && (
                <span className="dirty-dot">
                    ●
                </span>
            )}
            {!isRenaming && (
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
    renamingPath,
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
    onRenameCommit,
    onRenameCancel,
    onKeepBuffer
}: FileExplorerProps) {
    const activeBuffer = buffers.find((b) => getBufferId(b) === activeBufferId);

    const handleBufferContextMenu = (e: React.MouseEvent, bufferId: string) => {
        e.preventDefault();
        const buffer = buffers.find((b) => getBufferId(b) === bufferId);
        
        let contextType: 'file' | 'untitled' | 'unknown' = 'unknown';
        if (buffer?.kind === 'file') contextType = 'file';
        else if (buffer?.kind === 'untitled') contextType = 'untitled';

        electronAPI.showContextMenu({
            type: contextType, // Only allow file operations for real files
            path: buffer?.kind === 'file' ? buffer.filePath : undefined,
            bufferId: bufferId,
            isOpenBuffer: true,
            isWorkspaceFile: false,
            x: e.clientX,
            y: e.clientY,
        });
    };

    const handleTreeContextMenu = (e: React.MouseEvent, entry: FileTreeEntry) => {
        e.preventDefault();
        
        // Check if it's open
        const buffer = buffers.find(b => b.kind === 'file' && b.filePath === entry.path);
        
        electronAPI.showContextMenu({
            type: entry.type === 'directory' ? 'directory' : 'file',
            path: entry.path,
            bufferId: buffer ? getBufferId(buffer) : undefined,
            isOpenBuffer: !!buffer,
            isWorkspaceFile: true,
            x: e.clientX,
            y: e.clientY,
        });
    };

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
                                        <BufferItem
                                            key={bufferId}
                                            buffer={buffer}
                                            isActive={isActive}
                                            isRunning={isRunning}
                                            renamingPath={renamingPath}
                                            formatLabel={formatLabel}
                                            onSelectBuffer={onSelectBuffer}
                                            onContextMenu={handleBufferContextMenu}
                                            onCloseBuffer={onCloseBuffer}
                                            onRenameCommit={onRenameCommit}
                                            onRenameCancel={onRenameCancel}
                                            onKeepBuffer={onKeepBuffer}
                                        />
                                    );
                                })}
                            </ul>
                        )}
                    </div>
                </div>

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
                                            onContextMenu={handleTreeContextMenu}
                                            renamingPath={renamingPath}
                                            onRenameCommit={onRenameCommit}
                                            onRenameCancel={onRenameCancel}
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
