"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.registerMidiCompletionProvider = registerMidiCompletionProvider;
/**
 * Creates and registers a completion provider for MIDI device names.
 * The provider triggers when the user types a quote inside a `device:` property.
 *
 * @param monaco Monaco instance
 * @param fetchMidiDevices Function to fetch available MIDI devices
 * @returns Disposable to unregister the provider
 */
function registerMidiCompletionProvider(monaco, fetchMidiDevices) {
    const provider = {
        // Trigger on quotes and after typing
        triggerCharacters: ['"', "'", ':'],
        async provideCompletionItems(model, position, _context, _token) {
            // Get the text before the cursor on the current line
            const lineContent = model.getLineContent(position.lineNumber);
            const textBeforeCursor = lineContent.substring(0, position.column - 1);
            // Check if we're in a device property context
            // Patterns to match:
            // - device: "
            // - device: '
            // - device:"
            // - device:'
            // - { device: "
            const devicePatterns = [
                /device\s*:\s*["']$/, // device: " or device: '
                /device\s*:\s*$/, // device:  (just typed colon)
                /,\s*device\s*:\s*["']?$/, // , device: " (in object)
                /{\s*device\s*:\s*["']?$/, // { device: " (start of object)
            ];
            const isDeviceContext = devicePatterns.some(pattern => pattern.test(textBeforeCursor));
            if (!isDeviceContext) {
                return undefined;
            }
            // Fetch available MIDI devices
            let midiInputs;
            try {
                midiInputs = await fetchMidiDevices();
            }
            catch (error) {
                console.error('[MIDI Completion] Failed to fetch devices:', error);
                return undefined;
            }
            if (midiInputs.length === 0) {
                return undefined;
            }
            // Determine if we're already inside quotes
            const endsWithQuote = textBeforeCursor.endsWith('"') || textBeforeCursor.endsWith("'");
            const quoteChar = textBeforeCursor.endsWith('"') ? '"' : "'";
            // Determine the range for the completion
            const wordAtPosition = model.getWordAtPosition(position);
            const range = {
                startLineNumber: position.lineNumber,
                endLineNumber: position.lineNumber,
                startColumn: wordAtPosition?.startColumn ?? position.column,
                endColumn: wordAtPosition?.endColumn ?? position.column,
            };
            // Build suggestions
            const suggestions = midiInputs.map((device, index) => {
                // Determine what to insert
                let insertText;
                if (endsWithQuote) {
                    // Already typed opening quote, just insert name and closing quote
                    insertText = device.name + quoteChar;
                }
                else {
                    // Need to add quotes around the name
                    insertText = `"${device.name}"`;
                }
                return {
                    label: device.name,
                    kind: monaco.languages.CompletionItemKind.Value,
                    detail: `MIDI Input #${device.index}`,
                    documentation: {
                        value: `Connect to MIDI device: **${device.name}**`,
                    },
                    insertText,
                    range,
                    sortText: String(index).padStart(3, '0'), // Preserve order from system
                };
            });
            return {
                suggestions,
                incomplete: false, // List is complete
            };
        },
    };
    const disposable = monaco.languages.registerCompletionItemProvider('javascript', provider);
    return {
        dispose: () => disposable.dispose(),
    };
}
//# sourceMappingURL=midiCompletionProvider.js.map