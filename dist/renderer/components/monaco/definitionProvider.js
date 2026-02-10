"use strict";
/**
 * Custom Monaco definition provider that intercepts Go to Definition for DSL symbols
 * and opens the help window instead of navigating to the generated .d.ts file.
 */
Object.defineProperty(exports, "__esModule", { value: true });
exports.registerDslDefinitionProvider = registerDslDefinitionProvider;
exports.resolveDslSymbol = resolveDslSymbol;
exports.resolveDslSymbolAtPosition = resolveDslSymbolAtPosition;
exports.buildSymbolSets = buildSymbolSets;
const typeDocs_1 = require("../../../shared/dsl/typeDocs");
/**
 * Resolve the full dotted path at the cursor position.
 * For example, if cursor is on "adsr" in "env.adsr()", returns "env.adsr".
 * If cursor is on "env" in "env.adsr()", returns "env".
 */
function resolveDottedPath(model, position) {
    const word = model.getWordAtPosition(position);
    if (!word)
        return null;
    const line = model.getLineContent(position.lineNumber);
    const wordStart = word.startColumn - 1;
    const wordEnd = word.endColumn - 1;
    // Collect identifiers connected by dots (allowing whitespace around dots)
    // We'll build the path from discrete identifier strings rather than slicing
    // the raw line, so "osc .saw" resolves to "osc.saw".
    // Start with the current word
    const currentWord = word.word;
    // Look backwards for dot-separated identifiers
    const prefixParts = [];
    let i = wordStart - 1;
    while (i >= 0) {
        // Skip whitespace
        while (i >= 0 && /\s/.test(line[i]))
            i--;
        if (i < 0 || line[i] !== '.')
            break;
        // Found a dot, skip it
        i--;
        // Skip whitespace before the dot
        while (i >= 0 && /\s/.test(line[i]))
            i--;
        if (i < 0 || !/[\w$]/.test(line[i]))
            break;
        // Read identifier backwards
        let identEnd = i + 1;
        while (i > 0 && /[\w$]/.test(line[i - 1]))
            i--;
        prefixParts.unshift(line.slice(i, identEnd));
        i--;
    }
    // Look forwards for dot-separated identifiers
    const suffixParts = [];
    i = wordEnd;
    while (i < line.length) {
        // Skip whitespace
        while (i < line.length && /\s/.test(line[i]))
            i++;
        if (i >= line.length || line[i] !== '.')
            break;
        // Found a dot, skip it
        i++;
        // Skip whitespace after the dot
        while (i < line.length && /\s/.test(line[i]))
            i++;
        if (i >= line.length || !/[\w$]/.test(line[i]))
            break;
        // Read identifier forwards
        let identStart = i;
        while (i < line.length && /[\w$]/.test(line[i]))
            i++;
        suffixParts.push(line.slice(identStart, i));
    }
    const fullPath = [...prefixParts, currentWord, ...suffixParts].join('.');
    return { fullPath, word: word.word };
}
/**
 * Register a custom definition provider that opens the help window
 * for DSL types, modules, and namespaces.
 */
function registerDslDefinitionProvider(monaco, deps) {
    const provider = {
        provideDefinition(model, position, _token) {
            const resolved = resolveDottedPath(model, position);
            if (!resolved)
                return null;
            const match = resolveDslSymbol(resolved, deps.moduleNames, deps.namespaceNames);
            // For DSL symbols, return null to suppress TypeScript navigating to .d.ts
            // Help is opened separately via editor.onMouseDown (Cmd+Click)
            if (match)
                return null;
            // Not a DSL symbol - let TypeScript handle it
            return null;
        }
    };
    const disposable = monaco.languages.registerDefinitionProvider('javascript', provider);
    return { dispose: () => disposable.dispose() };
}
/**
 * Given a resolved dotted path, determine if it matches a DSL symbol.
 * Returns the match details or null.
 */
function resolveDslSymbol(resolved, moduleNames, namespaceNames) {
    const { fullPath, word } = resolved;
    if ((0, typeDocs_1.isDslType)(word)) {
        return { symbolType: 'type', symbolName: word };
    }
    if (moduleNames.has(fullPath)) {
        return { symbolType: 'module', symbolName: fullPath };
    }
    if (moduleNames.has(word)) {
        return { symbolType: 'module', symbolName: word };
    }
    if (namespaceNames.has(word)) {
        return { symbolType: 'namespace', symbolName: word };
    }
    return null;
}
/**
 * Resolve the DSL symbol at a given model position.
 * Convenience wrapper combining path resolution and symbol matching.
 */
function resolveDslSymbolAtPosition(model, position, moduleNames, namespaceNames) {
    const resolved = resolveDottedPath(model, position);
    if (!resolved)
        return null;
    return resolveDslSymbol(resolved, moduleNames, namespaceNames);
}
/**
 * Build sets of module names and namespace names from schemas.
 */
function buildSymbolSets(schemas) {
    const moduleNames = new Set();
    const namespaceNames = new Set();
    for (const schema of schemas) {
        moduleNames.add(schema.name);
        // Extract namespace from dotted name
        const parts = schema.name.split('.');
        if (parts.length > 1) {
            // Add all namespace prefixes
            for (let i = 0; i < parts.length - 1; i++) {
                namespaceNames.add(parts.slice(0, i + 1).join('.'));
                // Also add individual namespace segments
                namespaceNames.add(parts[i]);
            }
        }
    }
    return { moduleNames, namespaceNames };
}
//# sourceMappingURL=definitionProvider.js.map