import { useCustomMonaco } from '../../hooks/useCustomMonaco';

type Monaco = NonNullable<ReturnType<typeof useCustomMonaco>>;

export function registerConfigSchema(monaco: Monaco, schema: object) {
    const jsonDefaults = monaco.json.jsonDefaults;
    jsonDefaults.setDiagnosticsOptions({
        validate: true,
        allowComments: true,
        schemas: [
            {
                uri: 'modular://config-schema.json',
                fileMatch: [
                    '*/config.json',
                    '**/config.json',
                    'config.json',
                    '*.config.json',
                ],
                schema,
            },
        ],
    });
}

export function registerConfigSchemaForFile(
    monaco: Monaco,
    schema: object,
    currentFile: string,
) {
    const jsonDefaults = monaco.json.jsonDefaults;
    const fileUri = `file://${currentFile}`;
    jsonDefaults.setDiagnosticsOptions({
        validate: true,
        allowComments: true,
        schemas: [
            {
                uri: 'modular://config-schema.json',
                fileMatch: ['*'],
                schema,
            },
        ],
    });
    return fileUri;
}