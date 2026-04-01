import type { Monaco } from '../../hooks/useCustomMonaco';
import { applyDslLibToMonaco } from './monacoHelpers';
import type { ModuleSchema } from '@modular/core';

export interface MonacoSetupOptions {
    /** Module schemas for building symbol sets */
    schemas?: ModuleSchema[];
}

export function setupMonacoJavascript(
    monaco: Monaco,
    libSource: string,
    _options: MonacoSetupOptions = {},
) {
    const ts = monaco.typescript;
    console.log('Monaco TS version:', ts);
    const jsDefaults = ts.javascriptDefaults;

    jsDefaults.setCompilerOptions({
        allowJs: true,
        allowNonTsExtensions: true,
        checkJs: true,
        lib: ['esnext'],
        module: ts.ModuleKind.ESNext,
        moduleResolution: ts.ModuleResolutionKind.NodeJs,
        noEmit: true,
        target: ts.ScriptTarget.ES2020,
    });

    jsDefaults.setDiagnosticsOptions({
        noSemanticValidation: false,
        noSyntaxValidation: false,
    });

    jsDefaults.setEagerModelSync(true);

    const { extraLib, extraLibModel } = applyDslLibToMonaco(monaco, libSource);

    return () => {
        extraLib?.dispose();
        extraLibModel?.dispose();
    };
}
