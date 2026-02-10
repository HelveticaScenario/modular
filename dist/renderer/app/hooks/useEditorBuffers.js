"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.useEditorBuffers = useEditorBuffers;
const react_1 = require("react");
const uuid_1 = require("uuid");
const electronAPI_1 = __importDefault(require("../../electronAPI"));
const buffers_1 = require("../buffers");
function useEditorBuffers({ workspaceRoot, refreshFileTree, }) {
    const [buffers, setBuffers] = (0, react_1.useState)(() => {
        const saved = (0, buffers_1.readUnsavedBuffers)();
        return saved;
    });
    const [activeBufferId, setActiveBufferId] = (0, react_1.useState)(() => {
        const saved = (0, buffers_1.readUnsavedBuffers)();
        return saved.length > 0 ? (0, buffers_1.getBufferId)(saved[0]) : undefined;
    });
    const [usedUntitledNumbers, setUsedUntitledNumbers] = (0, react_1.useState)(new Set());
    const [renamingPath, setRenamingPath] = (0, react_1.useState)(null);
    (0, react_1.useEffect)(() => {
        const used = new Set();
        buffers.forEach((b) => {
            if (b.kind === 'untitled') {
                const match = b.id.match(/^untitled-(\d+)$/);
                if (match) {
                    used.add(parseInt(match[1], 10));
                }
            }
        });
        setUsedUntitledNumbers(used);
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, []);
    const activeBuffer = buffers.find((b) => (0, buffers_1.getBufferId)(b) === activeBufferId);
    const patchCode = activeBuffer?.content ?? buffers_1.DEFAULT_PATCH;
    (0, react_1.useEffect)(() => {
        (0, buffers_1.saveUnsavedBuffers)(buffers);
    }, [buffers]);
    const handlePatchChange = (0, react_1.useCallback)((value) => {
        setBuffers((prev) => prev.map((b) => (0, buffers_1.getBufferId)(b) === activeBufferId
            ? {
                ...b,
                content: value,
                dirty: true,
                isPreview: false,
            }
            : b));
    }, [activeBufferId]);
    const openFile = (0, react_1.useCallback)(async (relPath, options) => {
        if (!workspaceRoot) {
            throw new Error('No workspace open');
        }
        const absPath = `${workspaceRoot}/${relPath}`;
        const existing = buffers.find((b) => b.kind === 'file' && b.filePath === absPath);
        if (existing) {
            if (options?.preview === false && existing.isPreview) {
                setBuffers((prev) => prev.map((b) => (0, buffers_1.getBufferId)(b) === (0, buffers_1.getBufferId)(existing)
                    ? { ...b, isPreview: false }
                    : b));
            }
            setActiveBufferId((0, buffers_1.getBufferId)(existing));
            return;
        }
        const content = await electronAPI_1.default.filesystem.readFile(absPath);
        setBuffers((prev) => {
            let nextBuffers = [...prev];
            const existingPreviewIndex = nextBuffers.findIndex((b) => b.isPreview);
            if (options?.preview && existingPreviewIndex !== -1) {
                const previewBuffer = nextBuffers[existingPreviewIndex];
                if (!previewBuffer.dirty) {
                    nextBuffers.splice(existingPreviewIndex, 1);
                }
            }
            const newBuffer = {
                kind: 'file',
                filePath: absPath,
                content,
                id: (0, uuid_1.v4)(),
                dirty: false,
                isPreview: options?.preview ?? false,
            };
            return [...nextBuffers, newBuffer];
        });
        setActiveBufferId(absPath);
    }, [buffers, workspaceRoot]);
    const createUntitledFile = (0, react_1.useCallback)(() => {
        let nextIdNum = 1;
        while (usedUntitledNumbers.has(nextIdNum)) {
            nextIdNum++;
        }
        const nextId = `untitled-${nextIdNum}`;
        const newBuffer = {
            kind: 'untitled',
            id: nextId,
            content: buffers_1.DEFAULT_PATCH,
            dirty: false,
        };
        setUsedUntitledNumbers((prev) => {
            const next = new Set(prev);
            next.add(nextIdNum);
            return next;
        });
        setBuffers((prev) => [...prev, newBuffer]);
        setActiveBufferId(nextId);
    }, [usedUntitledNumbers]);
    const saveFile = (0, react_1.useCallback)(async (targetId) => {
        const idToSave = targetId || activeBufferId;
        const buffer = buffers.find((b) => (0, buffers_1.getBufferId)(b) === idToSave);
        if (!buffer)
            return;
        if (buffer.kind === 'untitled') {
            const input = await electronAPI_1.default.filesystem.showSaveDialog('untitled.mjs');
            if (!input)
                return;
            const normalized = (0, buffers_1.normalizeFileName)(input);
            if (!normalized)
                return;
            const result = await electronAPI_1.default.filesystem.writeFile(normalized, buffer.content);
            if (result.success) {
                const match = buffer.id.match(/^untitled-(\d+)$/);
                if (match) {
                    const num = parseInt(match[1], 10);
                    setUsedUntitledNumbers((prev) => {
                        const next = new Set(prev);
                        next.delete(num);
                        return next;
                    });
                }
                setBuffers((prev) => prev.map((b) => (0, buffers_1.getBufferId)(b) === idToSave
                    ? {
                        kind: 'file',
                        filePath: normalized,
                        id: b.id,
                        content: buffer.content,
                        dirty: false,
                    }
                    : b));
                if (idToSave === activeBufferId) {
                    setActiveBufferId(normalized);
                }
                await refreshFileTree();
            }
            else {
                throw new Error(result.error || 'Failed to save file');
            }
        }
        else {
            const result = await electronAPI_1.default.filesystem.writeFile(buffer.filePath, buffer.content);
            if (result.success) {
                setBuffers((prev) => prev.map((b) => (0, buffers_1.getBufferId)(b) === idToSave
                    ? { ...b, dirty: false }
                    : b));
            }
            else {
                throw new Error(result.error || 'Failed to save file');
            }
        }
    }, [activeBufferId, buffers, refreshFileTree]);
    const renameFile = (0, react_1.useCallback)(async (targetIdOrPath) => {
        let filePath;
        let resolvedPath = targetIdOrPath;
        if (targetIdOrPath &&
            workspaceRoot &&
            !targetIdOrPath.startsWith('/') &&
            !targetIdOrPath.match(/^[a-zA-Z]:/)) {
            resolvedPath = `${workspaceRoot}/${targetIdOrPath}`;
        }
        const buffer = buffers.find((b) => (0, buffers_1.getBufferId)(b) === targetIdOrPath) ||
            buffers.find((b) => b.kind === 'file' && b.filePath === resolvedPath);
        if (buffer && buffer.kind === 'file') {
            filePath = buffer.filePath;
        }
        else if (resolvedPath && typeof resolvedPath === 'string') {
            filePath = resolvedPath;
        }
        else if (activeBufferId) {
            const active = buffers.find((b) => (0, buffers_1.getBufferId)(b) === activeBufferId);
            if (active && active.kind === 'file') {
                filePath = active.filePath;
            }
        }
        if (!filePath)
            return;
        setRenamingPath(filePath);
    }, [activeBufferId, buffers, workspaceRoot]);
    const handleRenameCommit = (0, react_1.useCallback)(async (oldPath, newName) => {
        setRenamingPath(null);
        if (!newName)
            return;
        const currentFileName = oldPath.split(/[/\\]/).pop();
        if (newName === currentFileName)
            return;
        const normalized = (0, buffers_1.normalizeFileName)(newName);
        const separator = oldPath.includes('\\') ? '\\' : '/';
        const lastSepIndex = oldPath.lastIndexOf(separator);
        let newPath = normalized;
        if (lastSepIndex !== -1) {
            const dir = oldPath.substring(0, lastSepIndex);
            newPath = `${dir}${separator}${normalized}`;
        }
        if (!newPath || newPath === oldPath)
            return;
        const result = await electronAPI_1.default.filesystem.renameFile(oldPath, newPath);
        if (result.success) {
            setBuffers((prev) => prev.map((b) => b.kind === 'file' && b.filePath === oldPath
                ? { ...b, filePath: newPath }
                : b));
            const wasActive = buffers.some((b) => (0, buffers_1.getBufferId)(b) === activeBufferId &&
                b.kind === 'file' &&
                b.filePath === oldPath);
            if (wasActive) {
                setActiveBufferId(newPath);
            }
            await refreshFileTree();
        }
        else {
            throw new Error(result.error || 'Failed to rename file');
        }
    }, [activeBufferId, buffers, refreshFileTree]);
    const deleteFile = (0, react_1.useCallback)(async (targetIdOrPath) => {
        let filePath;
        let bufferId;
        let resolvedPath = targetIdOrPath;
        if (targetIdOrPath &&
            workspaceRoot &&
            !targetIdOrPath.startsWith('/') &&
            !targetIdOrPath.match(/^[a-zA-Z]:/)) {
            resolvedPath = `${workspaceRoot}/${targetIdOrPath}`;
        }
        const buffer = buffers.find((b) => (0, buffers_1.getBufferId)(b) === targetIdOrPath) ||
            buffers.find((b) => b.kind === 'file' && b.filePath === resolvedPath);
        if (buffer && buffer.kind === 'file') {
            filePath = buffer.filePath;
            bufferId = (0, buffers_1.getBufferId)(buffer);
        }
        else if (resolvedPath && typeof resolvedPath === 'string') {
            filePath = resolvedPath;
        }
        else if (activeBufferId) {
            const active = buffers.find((b) => (0, buffers_1.getBufferId)(b) === activeBufferId);
            if (active && active.kind === 'file') {
                filePath = active.filePath;
                bufferId = (0, buffers_1.getBufferId)(active);
            }
        }
        if (!filePath)
            return;
        if (!window.confirm(`Delete ${filePath}?`))
            return;
        const result = await electronAPI_1.default.filesystem.deleteFile(filePath);
        if (result.success) {
            setBuffers((prev) => prev.filter((b) => !(b.kind === 'file' && b.filePath === filePath)));
            const activeIsDeleted = activeBufferId &&
                ((bufferId && activeBufferId === bufferId) ||
                    (buffers.find((b) => (0, buffers_1.getBufferId)(b) === activeBufferId)
                        ?.kind === 'file' &&
                        buffers.find((b) => (0, buffers_1.getBufferId)(b) === activeBufferId).filePath === filePath));
            if (activeIsDeleted) {
                const remaining = buffers.filter((b) => !(b.kind === 'file' && b.filePath === filePath));
                if (remaining.length > 0) {
                    setActiveBufferId((0, buffers_1.getBufferId)(remaining[0]));
                }
                else {
                    setActiveBufferId(undefined);
                }
            }
            await refreshFileTree();
        }
        else {
            throw new Error(result.error || 'Failed to delete file');
        }
    }, [activeBufferId, buffers, refreshFileTree, workspaceRoot]);
    const closeBuffer = (0, react_1.useCallback)(async (bufferId) => {
        const buffer = buffers.find((b) => (0, buffers_1.getBufferId)(b) === bufferId);
        if (!buffer)
            return;
        if (buffer.dirty) {
            const response = await electronAPI_1.default.showUnsavedChangesDialog((0, buffers_1.formatBufferLabel)(buffer));
            if (response === 2) {
                return;
            }
            else if (response === 0) {
                try {
                    await saveFile(bufferId);
                    performCloseBuffer(bufferId);
                }
                catch (error) {
                    console.error('Error saving file:', error);
                    performCloseBuffer(bufferId);
                }
            }
            else {
                performCloseBuffer(bufferId);
            }
        }
        else {
            performCloseBuffer(bufferId);
        }
    }, [buffers, saveFile]);
    const performCloseBuffer = (0, react_1.useCallback)((bufferId) => {
        const buffer = buffers.find((b) => (0, buffers_1.getBufferId)(b) === bufferId);
        if (!buffer)
            return;
        setTimeout(() => {
            setBuffers((prev) => prev.filter((b) => (0, buffers_1.getBufferId)(b) !== bufferId));
            if (buffer.kind === 'untitled') {
                const match = buffer.id.match(/^untitled-(\d+)$/);
                if (match) {
                    const num = parseInt(match[1], 10);
                    setUsedUntitledNumbers((prev) => {
                        const next = new Set(prev);
                        next.delete(num);
                        return next;
                    });
                }
            }
            if (activeBufferId === bufferId) {
                const remaining = buffers.filter((b) => (0, buffers_1.getBufferId)(b) !== bufferId);
                if (remaining.length > 0) {
                    setActiveBufferId((0, buffers_1.getBufferId)(remaining[0]));
                }
                else {
                    setActiveBufferId(undefined);
                }
            }
        }, 50);
    }, [activeBufferId, buffers]);
    const keepBuffer = (0, react_1.useCallback)((bufferId) => {
        setBuffers((prev) => prev.map((b) => (0, buffers_1.getBufferId)(b) === bufferId ? { ...b, isPreview: false } : b));
    }, []);
    const formatFileLabel = (0, react_1.useCallback)((buffer) => {
        return (0, buffers_1.formatBufferLabel)(buffer);
    }, []);
    return {
        buffers,
        setBuffers,
        activeBufferId,
        setActiveBufferId,
        patchCode,
        handlePatchChange,
        openFile,
        createUntitledFile,
        saveFile,
        renameFile,
        deleteFile,
        closeBuffer,
        keepBuffer,
        renamingPath,
        setRenamingPath,
        handleRenameCommit,
        formatFileLabel,
    };
}
//# sourceMappingURL=useEditorBuffers.js.map