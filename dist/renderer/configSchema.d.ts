/**
 * JSON Schema for the Modular app configuration file.
 * Used by Monaco editor for validation and autocomplete.
 */
export declare const configSchema: {
    $schema: string;
    title: string;
    description: string;
    type: string;
    properties: {
        theme: {
            type: string;
            description: string;
            enum: string[];
            default: string;
        };
        cursorStyle: {
            type: string;
            description: string;
            enum: string[];
            default: string;
        };
        font: {
            type: string;
            description: string;
            enum: string[];
            default: string;
        };
        fontLigatures: {
            type: string;
            description: string;
            default: boolean;
        };
        fontSize: {
            type: string;
            description: string;
            minimum: number;
            maximum: number;
            default: number;
        };
        prettier: {
            type: string;
            description: string;
            properties: {
                singleQuote: {
                    type: string;
                    description: string;
                    default: boolean;
                };
                trailingComma: {
                    type: string;
                    description: string;
                    enum: string[];
                    default: string;
                };
                semi: {
                    type: string;
                    description: string;
                    default: boolean;
                };
                tabWidth: {
                    type: string;
                    description: string;
                    default: number;
                };
                printWidth: {
                    type: string;
                    description: string;
                    default: number;
                };
            };
            additionalProperties: boolean;
        };
        lastOpenedFolder: {
            type: string;
            description: string;
        };
        audioConfig: {
            type: string;
            description: string;
            properties: {
                hostId: {
                    type: string;
                    description: string;
                };
                inputDeviceId: {
                    type: string[];
                    description: string;
                };
                outputDeviceId: {
                    type: string;
                    description: string;
                };
                sampleRate: {
                    type: string;
                    description: string;
                };
                bufferSize: {
                    type: string;
                    description: string;
                };
            };
            additionalProperties: boolean;
        };
    };
    additionalProperties: boolean;
};
