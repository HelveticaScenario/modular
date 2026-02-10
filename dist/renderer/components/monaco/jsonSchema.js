"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.registerConfigSchema = registerConfigSchema;
exports.registerConfigSchemaForFile = registerConfigSchemaForFile;
function registerConfigSchema(monaco, schema) {
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
function registerConfigSchemaForFile(monaco, schema, currentFile) {
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
//# sourceMappingURL=jsonSchema.js.map