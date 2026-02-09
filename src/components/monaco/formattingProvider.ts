import * as prettier from 'prettier/standalone';
import * as prettierBabel from 'prettier/plugins/babel';
import * as prettierEstree from 'prettier/plugins/estree';
import type { Monaco } from '../../hooks/useCustomMonaco';
import type { PrettierConfig } from '../../ipcTypes';

const DEFAULT_PRETTIER_OPTIONS = {
    singleQuote: true,
    trailingComma: 'all' as const,
    semi: false,
    tabWidth: 2,
    printWidth: 60,
};

export function registerDslFormattingProvider(
    monaco: Monaco,
    userConfig: PrettierConfig = {},
) {
    return monaco.languages.registerDocumentFormattingEditProvider(
        'javascript',
        {
            async provideDocumentFormattingEdits(model) {
                const formatted = await prettier.format(model.getValue(), {
                    ...DEFAULT_PRETTIER_OPTIONS,
                    ...userConfig,
                    // parser and plugins must not be overridden
                    parser: 'babel',
                    plugins: [prettierBabel, prettierEstree],
                });

                return [
                    {
                        range: model.getFullModelRange(),
                        text: formatted.trimEnd(),
                    },
                ];
            },
        },
    );
}