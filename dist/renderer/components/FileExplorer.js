"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.SCRATCH_FILE = void 0;
exports.FileExplorer = FileExplorer;
const jsx_runtime_1 = require("react/jsx-runtime");
const react_1 = require("react");
require("./FileExplorer.css");
const buffers_1 = require("../app/buffers");
const electronAPI_1 = __importDefault(require("../electronAPI"));
exports.SCRATCH_FILE = '__scratch__.mjs';
function TreeNode({ entry, onOpenFile, onContextMenu, renamingPath, onRenameCommit, onRenameCancel, }) {
    const [expanded, setExpanded] = (0, react_1.useState)(true);
    const [editName, setEditName] = (0, react_1.useState)(entry.name);
    const inputRef = (0, react_1.useRef)(null);
    const isRenaming = renamingPath === entry.path;
    (0, react_1.useEffect)(() => {
        if (isRenaming && inputRef.current) {
            setEditName(entry.name);
            inputRef.current.focus();
            const name = entry.name;
            const lastDotIndex = name.lastIndexOf('.');
            if (lastDotIndex !== -1) {
                inputRef.current.setSelectionRange(0, lastDotIndex);
            }
            else {
                inputRef.current.select();
            }
        }
    }, [isRenaming, entry.name]);
    const handleKeyDown = (e) => {
        if (e.key === 'Enter') {
            e.stopPropagation();
            onRenameCommit(entry.path, editName);
        }
        else if (e.key === 'Escape') {
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
        return ((0, jsx_runtime_1.jsxs)("li", { className: "tree-file", onClick: handleSingleClick, onDoubleClick: handleDoubleClick, onContextMenu: (e) => onContextMenu(e, entry), children: [(0, jsx_runtime_1.jsx)("span", { className: "file-icon", children: "\uD83D\uDCC4" }), isRenaming ? ((0, jsx_runtime_1.jsx)("input", { ref: inputRef, type: "text", className: "rename-input", value: editName, onChange: (e) => setEditName(e.target.value), onKeyDown: handleKeyDown, 
                    // onBlur={handleBlur} // Blur handling can be tricky with specific commit logic, skipping for now to avoid accidental commits while debugging
                    onBlur: onRenameCancel })) : ((0, jsx_runtime_1.jsx)("span", { className: "file-name", children: entry.name }))] }));
    }
    return ((0, jsx_runtime_1.jsxs)("li", { className: "tree-folder", children: [(0, jsx_runtime_1.jsxs)("div", { className: "folder-header", onClick: () => !isRenaming && setExpanded(!expanded), onContextMenu: (e) => onContextMenu(e, entry), children: [(0, jsx_runtime_1.jsx)("span", { className: "folder-icon", children: expanded ? '📂' : '📁' }), (0, jsx_runtime_1.jsx)("span", { className: "folder-name", children: entry.name })] }), expanded && entry.children && entry.children.length > 0 && ((0, jsx_runtime_1.jsx)("ul", { className: "tree-children", children: entry.children.map((child) => ((0, jsx_runtime_1.jsx)(TreeNode, { entry: child, onOpenFile: onOpenFile, onContextMenu: onContextMenu, renamingPath: renamingPath, onRenameCommit: onRenameCommit, onRenameCancel: onRenameCancel }, child.path))) }))] }));
}
function BufferItem({ buffer, isActive, isRunning, renamingPath, formatLabel, onSelectBuffer, onContextMenu, onCloseBuffer, onRenameCommit, onRenameCancel, onKeepBuffer }) {
    const bufferId = (0, buffers_1.getBufferId)(buffer);
    const [editName, setEditName] = (0, react_1.useState)(formatLabel(buffer));
    const inputRef = (0, react_1.useRef)(null);
    const isRenaming = buffer.kind === 'file' && renamingPath === buffer.filePath;
    (0, react_1.useEffect)(() => {
        if (isRenaming && inputRef.current) {
            setEditName(formatLabel(buffer));
            inputRef.current.focus();
            const name = formatLabel(buffer);
            const lastDotIndex = name.lastIndexOf('.');
            if (lastDotIndex !== -1) {
                inputRef.current.setSelectionRange(0, lastDotIndex);
            }
            else {
                inputRef.current.select();
            }
        }
    }, [isRenaming, buffer, formatLabel]);
    const handleKeyDown = (e) => {
        if (e.key === 'Enter') {
            e.stopPropagation();
            if (buffer.kind === 'file') {
                onRenameCommit(buffer.filePath, editName);
            }
        }
        else if (e.key === 'Escape') {
            e.stopPropagation();
            onRenameCancel();
        }
    };
    return ((0, jsx_runtime_1.jsxs)("li", { className: [
            'buffer-item',
            isActive ? 'active' : '',
            buffer.dirty ? 'dirty' : '',
            isRunning ? 'running' : '',
            buffer.isPreview ? 'preview' : '',
        ]
            .filter(Boolean)
            .join(' '), onClick: () => !isRenaming && onSelectBuffer(bufferId), onDoubleClick: () => !isRenaming && onKeepBuffer(bufferId), onContextMenu: (e) => onContextMenu(e, bufferId), children: [isRenaming ? ((0, jsx_runtime_1.jsx)("input", { ref: inputRef, type: "text", className: "rename-input", value: editName, onChange: (e) => setEditName(e.target.value), onKeyDown: handleKeyDown, onBlur: onRenameCancel, onClick: (e) => e.stopPropagation() })) : ((0, jsx_runtime_1.jsx)("span", { className: "file-name", children: formatLabel(buffer) })), !isRenaming && isRunning && ((0, jsx_runtime_1.jsx)("span", { className: "running-badge", children: "\u25B6" })), !isRenaming && buffer.dirty && ((0, jsx_runtime_1.jsx)("span", { className: "dirty-dot", children: "\u25CF" })), !isRenaming && ((0, jsx_runtime_1.jsx)("button", { className: "close-button", onClick: (e) => {
                    e.stopPropagation();
                    onCloseBuffer(bufferId);
                }, title: "Close", children: "\u00D7" }))] }));
}
function FileExplorer({ workspaceRoot, fileTree, buffers, activeBufferId, runningBufferId, renamingPath, formatLabel, onSelectBuffer, onOpenFile, onCreateFile, onSaveFile, onRenameFile, onDeleteFile, onCloseBuffer, onSelectWorkspace, onRefreshTree, onRenameCommit, onRenameCancel, onKeepBuffer }) {
    const activeBuffer = buffers.find((b) => (0, buffers_1.getBufferId)(b) === activeBufferId);
    const handleBufferContextMenu = (e, bufferId) => {
        e.preventDefault();
        const buffer = buffers.find((b) => (0, buffers_1.getBufferId)(b) === bufferId);
        let contextType = 'unknown';
        if (buffer?.kind === 'file')
            contextType = 'file';
        else if (buffer?.kind === 'untitled')
            contextType = 'untitled';
        electronAPI_1.default.showContextMenu({
            type: contextType, // Only allow file operations for real files
            path: buffer?.kind === 'file' ? buffer.filePath : undefined,
            bufferId: bufferId,
            isOpenBuffer: true,
            isWorkspaceFile: false,
            x: e.clientX,
            y: e.clientY,
        });
    };
    const handleTreeContextMenu = (e, entry) => {
        e.preventDefault();
        // Check if it's open
        const buffer = buffers.find(b => b.kind === 'file' && b.filePath === entry.path);
        electronAPI_1.default.showContextMenu({
            type: entry.type === 'directory' ? 'directory' : 'file',
            path: entry.path,
            bufferId: buffer ? (0, buffers_1.getBufferId)(buffer) : undefined,
            isOpenBuffer: !!buffer,
            isWorkspaceFile: true,
            x: e.clientX,
            y: e.clientY,
        });
    };
    return ((0, jsx_runtime_1.jsx)("div", { className: "file-explorer", children: (0, jsx_runtime_1.jsxs)("div", { className: "file-sections", children: [(0, jsx_runtime_1.jsxs)("div", { className: "section", children: [(0, jsx_runtime_1.jsxs)("div", { className: "section-header", children: [(0, jsx_runtime_1.jsx)("span", { children: "Open Editors" }), (0, jsx_runtime_1.jsx)("button", { onClick: onCreateFile, title: "New untitled file", className: "section-action", children: "+" })] }), (0, jsx_runtime_1.jsx)("div", { className: "file-list", children: buffers.length === 0 ? ((0, jsx_runtime_1.jsx)("div", { className: "empty-message", children: "No open files" })) : ((0, jsx_runtime_1.jsx)("ul", { children: buffers.map((buffer) => {
                                    const bufferId = (0, buffers_1.getBufferId)(buffer);
                                    const isActive = bufferId === activeBufferId;
                                    const isRunning = bufferId === runningBufferId;
                                    return ((0, jsx_runtime_1.jsx)(BufferItem, { buffer: buffer, isActive: isActive, isRunning: isRunning, renamingPath: renamingPath, formatLabel: formatLabel, onSelectBuffer: onSelectBuffer, onContextMenu: handleBufferContextMenu, onCloseBuffer: onCloseBuffer, onRenameCommit: onRenameCommit, onRenameCancel: onRenameCancel, onKeepBuffer: onKeepBuffer }, bufferId));
                                }) })) })] }), workspaceRoot && ((0, jsx_runtime_1.jsxs)("div", { className: "section", children: [(0, jsx_runtime_1.jsxs)("div", { className: "section-header", children: [(0, jsx_runtime_1.jsx)("span", { children: "Workspace Files" }), (0, jsx_runtime_1.jsx)("button", { onClick: onRefreshTree, title: "Refresh file tree", className: "section-action", children: "\u21BB" })] }), (0, jsx_runtime_1.jsx)("div", { className: "file-tree", children: fileTree.length === 0 ? ((0, jsx_runtime_1.jsx)("div", { className: "empty-message", children: "No .js files found" })) : ((0, jsx_runtime_1.jsx)("ul", { className: "tree-root", children: fileTree.map((entry) => ((0, jsx_runtime_1.jsx)(TreeNode, { entry: entry, onOpenFile: onOpenFile, onContextMenu: handleTreeContextMenu, renamingPath: renamingPath, onRenameCommit: onRenameCommit, onRenameCancel: onRenameCancel }, entry.path))) })) })] }))] }) }));
}
//# sourceMappingURL=FileExplorer.js.map