import type { Monaco } from '../../hooks/useCustomMonaco';

// Apply the generated DSL .d.ts library to Monaco and expose some
// debug handles on window so we can inspect schemas and lib source
// from the browser console.
export function applyDslLibToMonaco(monaco: Monaco, libSource: string) {
    if (!monaco || !libSource) return null;

    const ts = monaco.typescript;
    const jsDefaults = ts.javascriptDefaults;
    return jsDefaults.addExtraLib(libSource, 'file:///modular/dsl-lib.d.ts');
}

export function formatPath(currentFile: string) {
    if (!currentFile.startsWith('/')) {
        currentFile = '/' + currentFile;
    }
    if (!currentFile.endsWith('.js') && !currentFile.endsWith('.mjs')) {
        currentFile = currentFile + '.mjs';
    }
    return `file://${currentFile}`;
}