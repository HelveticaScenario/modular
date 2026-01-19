import type { ModuleSchema } from '@modular/core';
import { useCustomMonaco } from '../../hooks/useCustomMonaco';
import { applyDslLibToMonaco } from './monacoHelpers';
import { findSliderCalls } from './sliderWidgets';

type Monaco = NonNullable<ReturnType<typeof useCustomMonaco>>;

export function setupMonacoJavascript(monaco: Monaco, schemas: ModuleSchema[]) {
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

    const extraLib = applyDslLibToMonaco(monaco, schemas);
    const inlayHints = monaco.languages.registerInlayHintsProvider(
        'javascript',
        {
            provideInlayHints(model, range) {
                const code = model.getValueInRange(range);
                const sliderCalls = findSliderCalls(code);
                console.log('Providing inlay hints for slider calls:', sliderCalls);
                return {
                    hints: sliderCalls.map((call, i) => {
                        const position = model.getPositionAt(
                            call.openParenIndex + 1,
                        );
                        return {
                            position,
                            label: ' '
                                .repeat(10)
                                .concat('\u200C')
                                .concat('\u200B'.repeat(i))
                                .concat('\u200C'),
                        };
                    }),
                    dispose() {},
                };
            },
        },
    );

    return () => {
        extraLib?.dispose();
        inlayHints?.dispose();
    };
}