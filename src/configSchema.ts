/**
 * JSON Schema for the Modular app configuration file.
 * Used by Monaco editor for validation and autocomplete.
 */

export const configSchema = {
    // $schema: "http://json-schema.org/draft-07/schema#",
    // title: "Modular Configuration",
    // description: "Configuration file for the Modular synthesizer application",
    type: "object",
    properties: {
        theme: {
            type: "string",
            description: "The color theme for the application and editor",
            enum: [
                "modular-dark",
                "one-dark-pro",
                "dracula",
                "gruvbox-dark",
                "tokyo-night",
                "catppuccin-mocha"
            ],
            default: "modular-dark"
        },
        cursorStyle: {
            type: "string",
            description: "The cursor style in the editor",
            enum: [
                "line",
                "block",
                "underline",
                "line-thin",
                "block-outline",
                "underline-thin"
            ],
            default: "block"
        },
        lastOpenedFolder: {
            type: "string",
            description: "Path to the last opened workspace folder (managed automatically)"
        }
    },
    additionalProperties: false
};
