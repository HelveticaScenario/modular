import type { Monaco } from '../../hooks/useCustomMonaco';
import { applyDslLibToMonaco } from './monacoHelpers';
import { registerDslDefinitionProvider, buildSymbolSets, DefinitionProviderDeps } from './definitionProvider';
import type { ModuleSchema } from '@modular/core';

export interface MonacoSetupOptions {
    /** Module schemas for building symbol sets */
    schemas?: ModuleSchema[];
}

export function setupMonacoJavascript(
    monaco: Monaco,
    libSource: string,
    options: MonacoSetupOptions = {}
) {
    const ts = monaco.typescript;
    console.log('Monaco TS version:', ts);
    const jsDefaults = ts.javascriptDefaults;

    jsDefaults.setCompilerOptions({
        allowJs: true,
        checkJs: true,
        lib: ['esnext'],
        allowNonTsExtensions: true,
        target: ts.ScriptTarget.ES2020,
        module: ts.ModuleKind.ESNext,
        moduleResolution: ts.ModuleResolutionKind.NodeJs,
        noEmit: true,
    });

    jsDefaults.setDiagnosticsOptions({
        noSemanticValidation: false,
        noSyntaxValidation: false,
    });

    jsDefaults.setEagerModelSync(true);

    const { extraLib, extraLibModel } = applyDslLibToMonaco(monaco, libSource);


    // Register definition provider if schemas are provided
    let definitionProvider: { dispose: () => void } | null = null;
    if (options.schemas) {
        const { moduleNames, namespaceNames } = buildSymbolSets(options.schemas);
        const deps: DefinitionProviderDeps = {
            moduleNames,
            namespaceNames,
        };
        // definitionProvider = registerDslDefinitionProvider(monaco, deps);
    }

    return () => {
        extraLib?.dispose();
        extraLibModel?.dispose();
    };
}