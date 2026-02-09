import * as prettier from 'prettier/standalone';
import * as prettierBabel from 'prettier/plugins/babel';
import * as prettierEstree from 'prettier/plugins/estree';
import type { Monaco } from '../../hooks/useCustomMonaco';

export function registerDslFormattingProvider(monaco: Monaco) {
    return monaco.languages.registerDocumentFormattingEditProvider(
        'javascript',
        {
            async provideDocumentFormattingEdits(model) {
                const formatted = await prettier.format(model.getValue(), {
                    parser: 'babel',
                    plugins: [prettierBabel, prettierEstree],
                    singleQuote: true,
                    trailingComma: 'all',
                    semi: false,
                    tabWidth: 2,
                    printWidth: 60,
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