import './FileExplorer.css';

interface FileExplorerProps {
  files: string[];
  currentFile: string | null;
  onFileSelect: (filename: string) => void;
  onRefresh: () => void;
}

export function FileExplorer({ 
  files, 
  currentFile, 
  onFileSelect,
  onRefresh 
}: FileExplorerProps) {
  return (
    <div className="file-explorer">
      <div className="file-explorer-header">
        <h3>Patches</h3>
        <button onClick={onRefresh} className="refresh-button" title="Refresh file list">
          ↻
        </button>
      </div>
      <div className="file-list">
        {files.length === 0 ? (
          <div className="empty-message">No .js files found</div>
        ) : (
          <ul>
            {files.map((file) => (
              <li
                key={file}
                className={file === currentFile ? 'active' : ''}
                onClick={() => onFileSelect(file)}
              >
                {file}
              </li>
            ))}
          </ul>
        )}
      </div>
    </div>
  );
}

