/**
 * JSON Schema for the Modular app configuration file.
 * Used by Monaco editor for validation and autocomplete.
 */

const audioConfigSchema = {
    type: 'object',
    description: 'Audio device configuration',
    properties: {
        hostId: {
            type: 'string',
            description: "Audio host identifier (e.g., 'CoreAudio', 'WASAPI')",
        },
        inputDeviceId: {
            type: ['string', 'null'],
            description: 'Input device ID, or null for no input',
        },
        outputDeviceId: {
            type: 'string',
            description: 'Output device ID',
        },
        sampleRate: {
            type: 'number',
            description: 'Sample rate in Hz (e.g., 44100, 48000)',
        },
        bufferSize: {
            type: 'number',
            description: 'Buffer size in samples (e.g., 256, 512)',
        },
    },
    additionalProperties: false,
};

export const configSchema = {
    $schema: 'http://json-schema.org/draft-07/schema#',
    title: 'Modular Configuration',
    description: 'Configuration file for the Modular synthesizer application',
    type: 'object',
    properties: {
        theme: {
            type: 'string',
            description: 'The color theme for the application and editor',
            enum: [
                'modular-dark',
                'one-dark-pro',
                'dracula',
                'gruvbox-dark',
                'tokyo-night',
                'catppuccin-mocha',
            ],
            default: 'modular-dark',
        },
        cursorStyle: {
            type: 'string',
            description: 'The cursor style in the editor',
            enum: [
                'line',
                'block',
                'underline',
                'line-thin',
                'block-outline',
                'underline-thin',
            ],
            default: 'block',
        },
        font: {
            type: 'string',
            description: 'The monospace font family used in the editor',
            enum: [
                'Fira Code',
                'JetBrains Mono',
                'Cascadia Code',
                'Source Code Pro',
                'IBM Plex Mono',
                'Hack',
                'Inconsolata',
                'Monaspace Neon',
                'Monaspace Argon',
                'Monaspace Xenon',
                'Monaspace Krypton',
                'Monaspace Radon',
                'Geist Mono',
                'Iosevka',
                'Victor Mono',
                'Roboto Mono',
                'Maple Mono',
                'Commit Mono',
                '0xProto',
                'Intel One Mono',
                'Mononoki',
                'Anonymous Pro',
                'Recursive',
                'SF Mono',
                'Monaco',
                'Menlo',
                'Consolas',
            ],
            default: 'Fira Code',
        },
        fontLigatures: {
            type: 'boolean',
            description: 'Enable or disable font ligatures in the editor',
            default: true,
        },
        fontSize: {
            type: 'number',
            description: 'The font size in the editor (8–72)',
            minimum: 8,
            maximum: 72,
            default: 17,
        },
        prettier: {
            type: 'object',
            description:
                "Prettier formatting options. Merged with defaults: singleQuote=true, trailingComma='all', semi=false, tabWidth=2, printWidth=60. The parser and plugins cannot be overridden.",
            properties: {
                singleQuote: {
                    type: 'boolean',
                    description: 'Use single quotes instead of double quotes',
                    default: true,
                },
                trailingComma: {
                    type: 'string',
                    description: 'Trailing comma style',
                    enum: ['all', 'es5', 'none'],
                    default: 'all',
                },
                semi: {
                    type: 'boolean',
                    description: 'Add semicolons at the end of statements',
                    default: false,
                },
                tabWidth: {
                    type: 'number',
                    description: 'Number of spaces per indentation level',
                    default: 2,
                },
                printWidth: {
                    type: 'number',
                    description: 'Line width before wrapping',
                    default: 60,
                },
            },
            additionalProperties: true,
        },
        lastOpenedFolder: {
            type: 'string',
            description:
                'Path to the last opened workspace folder (managed automatically)',
        },
        audioConfig: audioConfigSchema,
    },
    additionalProperties: false,
};
