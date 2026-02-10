/**
 * Custom Monaco definition provider that intercepts Go to Definition for DSL symbols
 * and opens the help window instead of navigating to the generated .d.ts file.
 */

import type { Monaco } from '../../hooks/useCustomMonaco';
import type { languages, editor, Position, CancellationToken } from 'monaco-editor';
import { DSL_TYPE_NAMES, isDslType } from '../../../shared/dsl/typeDocs';

export interface DefinitionProviderDeps {
    /** Set of module factory names (including namespaced ones like "osc.sine") */
    moduleNames: Set<string>;
    /** Set of namespace names (like "osc", "env", "filter") */
    namespaceNames: Set<string>;
}

/**
 * Resolve the full dotted path at the cursor position.
 * For example, if cursor is on "adsr" in "env.adsr()", returns "env.adsr".
 * If cursor is on "env" in "env.adsr()", returns "env".
 */
function resolveDottedPath(
    model: editor.ITextModel,
    position: Position
): { fullPath: string; word: string } | null {
    const word = model.getWordAtPosition(position);
    if (!word) return null;

    const line = model.getLineContent(position.lineNumber);
    const wordStart = word.startColumn - 1;
    const wordEnd = word.endColumn - 1;

    // Collect identifiers connected by dots (allowing whitespace around dots)
    // We'll build the path from discrete identifier strings rather than slicing
    // the raw line, so "osc .saw" resolves to "osc.saw".

    // Start with the current word
    const currentWord = word.word;

    // Look backwards for dot-separated identifiers
    const prefixParts: string[] = [];
    let i = wordStart - 1;
    while (i >= 0) {
        // Skip whitespace
        while (i >= 0 && /\s/.test(line[i])) i--;
        if (i < 0 || line[i] !== '.') break;
        // Found a dot, skip it
        i--;
        // Skip whitespace before the dot
        while (i >= 0 && /\s/.test(line[i])) i--;
        if (i < 0 || !/[\w$]/.test(line[i])) break;
        // Read identifier backwards
        let identEnd = i + 1;
        while (i > 0 && /[\w$]/.test(line[i - 1])) i--;
        prefixParts.unshift(line.slice(i, identEnd));
        i--;
    }

    // Look forwards for dot-separated identifiers
    const suffixParts: string[] = [];
    i = wordEnd;
    while (i < line.length) {
        // Skip whitespace
        while (i < line.length && /\s/.test(line[i])) i++;
        if (i >= line.length || line[i] !== '.') break;
        // Found a dot, skip it
        i++;
        // Skip whitespace after the dot
        while (i < line.length && /\s/.test(line[i])) i++;
        if (i >= line.length || !/[\w$]/.test(line[i])) break;
        // Read identifier forwards
        let identStart = i;
        while (i < line.length && /[\w$]/.test(line[i])) i++;
        suffixParts.push(line.slice(identStart, i));
    }

    const fullPath = [...prefixParts, currentWord, ...suffixParts].join('.');
    return { fullPath, word: word.word };
}

/**
 * Register a custom definition provider that opens the help window
 * for DSL types, modules, and namespaces.
 */
export function registerDslDefinitionProvider(
    monaco: Monaco,
    deps: DefinitionProviderDeps
): { dispose: () => void } {
    const provider: languages.DefinitionProvider = {
        provideDefinition(
            model: editor.ITextModel,
            position: Position,
            _token: CancellationToken
        ) {
            const resolved = resolveDottedPath(model, position);
            if (!resolved) return null;

            const match = resolveDslSymbol(resolved, deps.moduleNames, deps.namespaceNames);
            // For DSL symbols, return null to suppress TypeScript navigating to .d.ts
            // Help is opened separately via editor.onMouseDown (Cmd+Click)
            if (match) return null;

            // Not a DSL symbol - let TypeScript handle it
            return null;
        }
    };

    const disposable = monaco.languages.registerDefinitionProvider('javascript', provider);
    return { dispose: () => disposable.dispose() };
}

export interface DslSymbolMatch {
    symbolType: 'type' | 'module' | 'namespace';
    symbolName: string;
}

/**
 * Given a resolved dotted path, determine if it matches a DSL symbol.
 * Returns the match details or null.
 */
export function resolveDslSymbol(
    resolved: { fullPath: string; word: string },
    moduleNames: Set<string>,
    namespaceNames: Set<string>
): DslSymbolMatch | null {
    const { fullPath, word } = resolved;

    if (isDslType(word)) {
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
export function resolveDslSymbolAtPosition(
    model: editor.ITextModel,
    position: Position,
    moduleNames: Set<string>,
    namespaceNames: Set<string>
): DslSymbolMatch | null {
    const resolved = resolveDottedPath(model, position);
    if (!resolved) return null;
    return resolveDslSymbol(resolved, moduleNames, namespaceNames);
}

/**
 * Build sets of module names and namespace names from schemas.
 */
export function buildSymbolSets(schemas: Array<{ name: string }>): {
    moduleNames: Set<string>;
    namespaceNames: Set<string>;
} {
    const moduleNames = new Set<string>();
    const namespaceNames = new Set<string>();

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
