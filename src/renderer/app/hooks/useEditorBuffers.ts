import { useCallback, useEffect, useState } from 'react';
import { v4 } from 'uuid';
import electronAPI from '../../electronAPI';
import type { EditorBuffer } from '../../types/editor';
import {
    DEFAULT_PATCH,
    formatBufferLabel,
    getBufferId,
    normalizeFileName,
    readUnsavedBuffers,
    saveUnsavedBuffers,
} from '../buffers';

type UseEditorBuffersParams = {
    workspaceRoot: string | null;
    refreshFileTree: () => Promise<void>;
};

export function useEditorBuffers({
    workspaceRoot,
    refreshFileTree,
}: UseEditorBuffersParams) {
    const [buffers, setBuffers] = useState<EditorBuffer[]>(() => {
        const saved = readUnsavedBuffers();
        return saved;
    });

    const [activeBufferId, setActiveBufferId] = useState<string | undefined>(
        () => {
            const saved = readUnsavedBuffers();
            return saved.length > 0 ? getBufferId(saved[0]) : undefined;
        },
    );

    const [usedUntitledNumbers, setUsedUntitledNumbers] = useState<Set<number>>(
        new Set(),
    );

    const [renamingPath, setRenamingPath] = useState<string | null>(null);

    useEffect(() => {
        const used = new Set<number>();
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

    const activeBuffer = buffers.find((b) => getBufferId(b) === activeBufferId);
    const patchCode = activeBuffer?.content ?? DEFAULT_PATCH;

    useEffect(() => {
        saveUnsavedBuffers(buffers);
    }, [buffers]);

    const handlePatchChange = useCallback(
        (value: string) => {
            setBuffers((prev) =>
                prev.map((b) =>
                    getBufferId(b) === activeBufferId
                        ? {
                              ...b,
                              content: value,
                              dirty: true,
                              isPreview: false,
                          }
                        : b,
                ),
            );
        },
        [activeBufferId],
    );

    const openFile = useCallback(
        async (relPath: string, options?: { preview?: boolean }) => {
            if (!workspaceRoot) {
                throw new Error('No workspace open');
            }

            const absPath = `${workspaceRoot}/${relPath}`;

            const existing = buffers.find(
                (b) => b.kind === 'file' && b.filePath === absPath,
            );

            if (existing) {
                if (options?.preview === false && existing.isPreview) {
                    setBuffers((prev) =>
                        prev.map((b) =>
                            getBufferId(b) === getBufferId(existing)
                                ? { ...b, isPreview: false }
                                : b,
                        ),
                    );
                }
                setActiveBufferId(getBufferId(existing));
                return;
            }

            const content = await electronAPI.filesystem.readFile(absPath);

            setBuffers((prev) => {
                let nextBuffers = [...prev];
                const existingPreviewIndex = nextBuffers.findIndex(
                    (b) => b.isPreview,
                );

                if (options?.preview && existingPreviewIndex !== -1) {
                    const previewBuffer = nextBuffers[existingPreviewIndex];
                    if (!previewBuffer.dirty) {
                        nextBuffers.splice(existingPreviewIndex, 1);
                    }
                }

                const newBuffer: EditorBuffer = {
                    kind: 'file',
                    filePath: absPath,
                    content,
                    id: v4(),
                    dirty: false,
                    isPreview: options?.preview ?? false,
                };
                return [...nextBuffers, newBuffer];
            });
            setActiveBufferId(absPath);
        },
        [buffers, workspaceRoot],
    );

    const createUntitledFile = useCallback(() => {
        let nextIdNum = 1;
        while (usedUntitledNumbers.has(nextIdNum)) {
            nextIdNum++;
        }

        const nextId = `untitled-${nextIdNum}`;
        const newBuffer: EditorBuffer = {
            kind: 'untitled',
            id: nextId,
            content: DEFAULT_PATCH,
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

    const saveFile = useCallback(
        async (targetId?: string) => {
            const idToSave = targetId || activeBufferId;
            const buffer = buffers.find((b) => getBufferId(b) === idToSave);
            if (!buffer) return;

            if (buffer.kind === 'untitled') {
                const input = await electronAPI.filesystem.showSaveDialog(
                    'untitled.mjs',
                );
                if (!input) return;

                const normalized = normalizeFileName(input);
                if (!normalized) return;

                const result = await electronAPI.filesystem.writeFile(
                    normalized,
                    buffer.content,
                );

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

                    setBuffers((prev) =>
                        prev.map((b) =>
                            getBufferId(b) === idToSave
                                ? {
                                      kind: 'file' as const,
                                      filePath: normalized,
                                      id: b.id,
                                      content: buffer.content,
                                      dirty: false,
                                  }
                                : b,
                        ),
                    );
                    if (idToSave === activeBufferId) {
                        setActiveBufferId(normalized);
                    }
                    await refreshFileTree();
                } else {
                    throw new Error(result.error || 'Failed to save file');
                }
            } else {
                const result = await electronAPI.filesystem.writeFile(
                    buffer.filePath,
                    buffer.content,
                );

                if (result.success) {
                    setBuffers((prev) =>
                        prev.map((b) =>
                            getBufferId(b) === idToSave
                                ? { ...b, dirty: false }
                                : b,
                        ),
                    );
                } else {
                    throw new Error(result.error || 'Failed to save file');
                }
            }
        },
        [activeBufferId, buffers, refreshFileTree],
    );

    const renameFile = useCallback(
        async (targetIdOrPath?: string) => {
            let filePath: string | undefined;

            let resolvedPath = targetIdOrPath;
            if (
                targetIdOrPath &&
                workspaceRoot &&
                !targetIdOrPath.startsWith('/') &&
                !targetIdOrPath.match(/^[a-zA-Z]:/)
            ) {
                resolvedPath = `${workspaceRoot}/${targetIdOrPath}`;
            }

            const buffer =
                buffers.find((b) => getBufferId(b) === targetIdOrPath) ||
                buffers.find(
                    (b) => b.kind === 'file' && b.filePath === resolvedPath,
                );

            if (buffer && buffer.kind === 'file') {
                filePath = buffer.filePath;
            } else if (resolvedPath && typeof resolvedPath === 'string') {
                filePath = resolvedPath;
            } else if (activeBufferId) {
                const active = buffers.find(
                    (b) => getBufferId(b) === activeBufferId,
                );
                if (active && active.kind === 'file') {
                    filePath = active.filePath;
                }
            }

            if (!filePath) return;
            setRenamingPath(filePath);
        },
        [activeBufferId, buffers, workspaceRoot],
    );

    const handleRenameCommit = useCallback(
        async (oldPath: string, newName: string) => {
            setRenamingPath(null);
            if (!newName) return;

            const currentFileName = oldPath.split(/[/\\]/).pop();
            if (newName === currentFileName) return;

            const normalized = normalizeFileName(newName);

            const separator = oldPath.includes('\\') ? '\\' : '/';
            const lastSepIndex = oldPath.lastIndexOf(separator);
            let newPath = normalized;
            if (lastSepIndex !== -1) {
                const dir = oldPath.substring(0, lastSepIndex);
                newPath = `${dir}${separator}${normalized}`;
            }

            if (!newPath || newPath === oldPath) return;

            const result = await electronAPI.filesystem.renameFile(
                oldPath,
                newPath,
            );

            if (result.success) {
                setBuffers((prev) =>
                    prev.map((b) =>
                        b.kind === 'file' && b.filePath === oldPath
                            ? { ...b, filePath: newPath }
                            : b,
                    ),
                );

                const wasActive = buffers.some(
                    (b) =>
                        getBufferId(b) === activeBufferId &&
                        b.kind === 'file' &&
                        b.filePath === oldPath,
                );
                if (wasActive) {
                    setActiveBufferId(newPath);
                }

                await refreshFileTree();
            } else {
                throw new Error(result.error || 'Failed to rename file');
            }
        },
        [activeBufferId, buffers, refreshFileTree],
    );

    const deleteFile = useCallback(
        async (targetIdOrPath?: string) => {
            let filePath: string | undefined;
            let bufferId: string | undefined;

            let resolvedPath = targetIdOrPath;
            if (
                targetIdOrPath &&
                workspaceRoot &&
                !targetIdOrPath.startsWith('/') &&
                !targetIdOrPath.match(/^[a-zA-Z]:/)
            ) {
                resolvedPath = `${workspaceRoot}/${targetIdOrPath}`;
            }

            const buffer =
                buffers.find((b) => getBufferId(b) === targetIdOrPath) ||
                buffers.find(
                    (b) => b.kind === 'file' && b.filePath === resolvedPath,
                );

            if (buffer && buffer.kind === 'file') {
                filePath = buffer.filePath;
                bufferId = getBufferId(buffer);
            } else if (resolvedPath && typeof resolvedPath === 'string') {
                filePath = resolvedPath;
            } else if (activeBufferId) {
                const active = buffers.find(
                    (b) => getBufferId(b) === activeBufferId,
                );
                if (active && active.kind === 'file') {
                    filePath = active.filePath;
                    bufferId = getBufferId(active);
                }
            }

            if (!filePath) return;

            if (!window.confirm(`Delete ${filePath}?`)) return;

            const result = await electronAPI.filesystem.deleteFile(filePath);

            if (result.success) {
                setBuffers((prev) =>
                    prev.filter(
                        (b) =>
                            !(b.kind === 'file' && b.filePath === filePath),
                    ),
                );

                const activeIsDeleted =
                    activeBufferId &&
                    ((bufferId && activeBufferId === bufferId) ||
                        (buffers.find((b) => getBufferId(b) === activeBufferId)
                            ?.kind === 'file' &&
                            (
                                buffers.find(
                                    (b) => getBufferId(b) === activeBufferId,
                                ) as any
                            ).filePath === filePath));

                if (activeIsDeleted) {
                    const remaining = buffers.filter(
                        (b) =>
                            !(b.kind === 'file' && b.filePath === filePath),
                    );
                    if (remaining.length > 0) {
                        setActiveBufferId(getBufferId(remaining[0]));
                    } else {
                        setActiveBufferId(undefined);
                    }
                }

                await refreshFileTree();
            } else {
                throw new Error(result.error || 'Failed to delete file');
            }
        },
        [activeBufferId, buffers, refreshFileTree, workspaceRoot],
    );

    const closeBuffer = useCallback(
        async (bufferId: string) => {
            const buffer = buffers.find((b) => getBufferId(b) === bufferId);
            if (!buffer) return;

            if (buffer.dirty) {
                const response = await electronAPI.showUnsavedChangesDialog(
                    formatBufferLabel(buffer),
                );

                if (response === 2) {
                    return;
                } else if (response === 0) {
                    try {
                        await saveFile(bufferId);
                        performCloseBuffer(bufferId);
                    } catch (error) {
                        console.error('Error saving file:', error);
                        performCloseBuffer(bufferId);
                    }
                } else {
                    performCloseBuffer(bufferId);
                }
            } else {
                performCloseBuffer(bufferId);
            }
        },
        [buffers, saveFile],
    );

    const performCloseBuffer = useCallback(
        (bufferId: string) => {
            const buffer = buffers.find((b) => getBufferId(b) === bufferId);
            if (!buffer) return;

            setTimeout(() => {
                setBuffers((prev) =>
                    prev.filter((b) => getBufferId(b) !== bufferId),
                );

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
                    const remaining = buffers.filter(
                        (b) => getBufferId(b) !== bufferId,
                    );
                    if (remaining.length > 0) {
                        setActiveBufferId(getBufferId(remaining[0]));
                    } else {
                        setActiveBufferId(undefined);
                    }
                }
            }, 50);
        },
        [activeBufferId, buffers],
    );

    const keepBuffer = useCallback((bufferId: string) => {
        setBuffers((prev) =>
            prev.map((b) =>
                getBufferId(b) === bufferId ? { ...b, isPreview: false } : b,
            ),
        );
    }, []);

    const formatFileLabel = useCallback((buffer: EditorBuffer) => {
        return formatBufferLabel(buffer);
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