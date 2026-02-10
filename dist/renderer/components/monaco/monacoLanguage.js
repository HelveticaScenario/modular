"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.setupMonacoJavascript = setupMonacoJavascript;
const monacoHelpers_1 = require("./monacoHelpers");
const definitionProvider_1 = require("./definitionProvider");
function setupMonacoJavascript(monaco, libSource, options = {}) {
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
    const { extraLib, extraLibModel } = (0, monacoHelpers_1.applyDslLibToMonaco)(monaco, libSource);
    // Register definition provider if schemas are provided
    let definitionProvider = null;
    if (options.schemas) {
        const { moduleNames, namespaceNames } = (0, definitionProvider_1.buildSymbolSets)(options.schemas);
        const deps = {
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
//# sourceMappingURL=monacoLanguage.js.map