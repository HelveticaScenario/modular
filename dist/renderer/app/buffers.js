"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.normalizeFileName = exports.formatBufferLabel = exports.getBufferId = exports.saveUnsavedBuffers = exports.readUnsavedBuffers = exports.DEFAULT_PATCH = void 0;
exports.DEFAULT_PATCH = `// Simple 440 Hz sine wave
sine('a3').out();
`;
const UNSAVED_STORAGE_KEY = 'modular_unsaved_buffers';
const readUnsavedBuffers = () => {
    if (typeof window === 'undefined') {
        return [];
    }
    try {
        const raw = window.localStorage.getItem(UNSAVED_STORAGE_KEY);
        if (!raw)
            return [];
        const parsed = JSON.parse(raw);
        return parsed.map((snapshot) => {
            if (snapshot.kind === 'file') {
                return {
                    kind: 'file',
                    filePath: snapshot.filePath,
                    content: snapshot.content,
                    id: snapshot.id,
                    dirty: true,
                };
            }
            return {
                kind: 'untitled',
                id: snapshot.id,
                content: snapshot.content,
                dirty: true,
            };
        });
    }
    catch (error) {
        console.error('Failed to read unsaved buffers:', error);
        return [];
    }
};
exports.readUnsavedBuffers = readUnsavedBuffers;
const saveUnsavedBuffers = (buffers) => {
    if (typeof window === 'undefined')
        return;
    try {
        const dirtyBuffers = buffers.filter((b) => b.dirty);
        const snapshots = dirtyBuffers.map((buffer) => {
            if (buffer.kind === 'file') {
                return {
                    kind: 'file',
                    id: buffer.filePath,
                    filePath: buffer.filePath,
                    content: buffer.content,
                };
            }
            return {
                kind: 'untitled',
                id: buffer.id,
                content: buffer.content,
            };
        });
        window.localStorage.setItem(UNSAVED_STORAGE_KEY, JSON.stringify(snapshots));
    }
    catch (error) {
        console.error('Failed to save unsaved buffers:', error);
    }
};
exports.saveUnsavedBuffers = saveUnsavedBuffers;
const getBufferId = (buffer) => {
    return buffer.kind === 'file' ? buffer.filePath : buffer.id;
};
exports.getBufferId = getBufferId;
const formatBufferLabel = (buffer) => {
    if (buffer.kind === 'untitled') {
        return buffer.id;
    }
    return buffer.filePath;
};
exports.formatBufferLabel = formatBufferLabel;
const normalizeFileName = (name) => {
    const trimmed = name.trim();
    if (!trimmed) {
        return trimmed;
    }
    return trimmed.endsWith('.js') || trimmed.endsWith('.mjs')
        ? trimmed
        : `${trimmed}.mjs`;
};
exports.normalizeFileName = normalizeFileName;
//# sourceMappingURL=buffers.js.map