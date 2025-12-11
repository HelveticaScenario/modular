import './FileExplorer.css';

type FileState = {
    dirty?: boolean;
    isNew?: boolean;
};

interface FileExplorerProps {
    files: string[];
    openFiles: string[];
    currentFile: string;
    runningFile: string | null;
    fileStates: Record<string, FileState>;
    formatLabel?: (filename: string) => string;
    onFileSelect: (filename: string) => void;
    onRefresh: () => void;
    onCreateFile: () => void;
    onSaveFile: () => void;
    onRenameFile: () => void;
}

export const SCRATCH_FILE = '__scratch__.mjs';

export function FileExplorer({
    files,
    openFiles,
    currentFile,
    runningFile,
    fileStates,
    formatLabel,
    onFileSelect,
    onRefresh,
    onCreateFile,
    onSaveFile,
    onRenameFile,
}: FileExplorerProps) {
    const renderLabel = (file: string) =>
        formatLabel ? formatLabel(file) : file;

    return (
        <div className="file-explorer">
            <div className="file-explorer-header">
                <div className="file-explorer-title">
                    <h3>Patches</h3>
                    <span className="file-count">{files.length}</span>
                </div>
                <div className="file-explorer-actions">
                    <button
                        onClick={onCreateFile}
                        title="New file"
                        className="action-button"
                    >
                        New
                    </button>
                    <button
                        onClick={onSaveFile}
                        title="Save current file"
                        className="action-button"
                    >
                        Save
                    </button>
                    <button
                        onClick={onRenameFile}
                        title="Rename current file"
                        className="action-button"
                    >
                        Rename
                    </button>
                    <button
                        onClick={onRefresh}
                        className="refresh-button"
                        title="Refresh file list"
                    >
                        Refresh
                    </button>
                </div>
            </div>

            <div className="file-sections">
                <div className="section">
                    <div className="section-header">Open</div>
                    <div className="file-list">
                        {openFiles.length === 0 ? (
                            <div className="empty-message">No open files</div>
                        ) : (
                            <ul>
                                {openFiles.map((file) => {
                                    if (file !== SCRATCH_FILE) return null;
                                    const state = fileStates[file] ?? {};
                                    const isActive = file === currentFile;
                                    const isRunning = file === runningFile;
                                    return (
                                        <li
                                            key={`open-${file}`}
                                            className={[
                                                isActive ? 'active' : '',
                                                state.dirty ? 'dirty' : '',
                                                isRunning ? 'running' : '',
                                            ]
                                                .filter(Boolean)
                                                .join(' ')}
                                            onClick={() => onFileSelect(file)}
                                        >
                                            <span className="file-name">
                                                {renderLabel(file)}
                                            </span>
                                            {isRunning && (
                                                <span className="running-badge">
                                                    running
                                                </span>
                                            )}
                                            {state.isNew && (
                                                <span className="badge">
                                                    new
                                                </span>
                                            )}
                                            {state.dirty && (
                                                <span className="dirty-dot">
                                                    *
                                                </span>
                                            )}
                                        </li>
                                    );
                                })}
                            </ul>
                        )}
                    </div>
                </div>

                <div className="section">
                    <div className="section-header">All files</div>
                    <div className="file-list">
                        {files.length === 0 ? (
                            <div className="empty-message">
                                No .js files found
                            </div>
                        ) : (
                            <ul>
                                {files.map((file) => {
                                    const state = fileStates[file] ?? {};
                                    const isActive = file === currentFile;
                                    const isRunning = file === runningFile;
                                    return (
                                        <li
                                            key={file}
                                            className={[
                                                isActive ? 'active' : '',
                                                state.dirty ? 'dirty' : '',
                                                isRunning ? 'running' : '',
                                            ]
                                                .filter(Boolean)
                                                .join(' ')}
                                            onClick={() => onFileSelect(file)}
                                        >
                                            <span className="file-name">
                                                {renderLabel(file)}
                                            </span>
                                            {isRunning && (
                                                <span className="running-badge">
                                                    running
                                                </span>
                                            )}
                                            {state.dirty && (
                                                <span className="dirty-dot">
                                                    *
                                                </span>
                                            )}
                                        </li>
                                    );
                                })}
                            </ul>
                        )}
                    </div>
                </div>
            </div>
        </div>
    );
}
