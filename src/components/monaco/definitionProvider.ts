/**
 * Custom Monaco definition provider that intercepts Go to Definition for DSL symbols
 * and opens the help window instead of navigating to the generated .d.ts file.
 */

import type { Monaco } from '../../hooks/useCustomMonaco';
import type { languages, editor, Position, CancellationToken } from 'monaco-editor';
import { DSL_TYPE_NAMES, isDslType } from '../../dsl/typeDocs';

export interface DefinitionProviderDeps {
    /** Function to open help window for a symbol */
    openHelpForSymbol: (symbolType: 'type' | 'module' | 'namespace', symbolName: string) => void;
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

    // Look backwards for dot-separated identifiers
    let pathStart = wordStart;
    let i = wordStart - 1;
    while (i >= 0) {
        if (line[i] === '.') {
            // Check if there's an identifier before the dot
            const beforeDot = i - 1;
            if (beforeDot >= 0 && /[\w$]/.test(line[beforeDot])) {
                // Find the start of that identifier
                let identStart = beforeDot;
                while (identStart > 0 && /[\w$]/.test(line[identStart - 1])) {
                    identStart--;
                }
                pathStart = identStart;
                i = identStart - 1;
            } else {
                break;
            }
        } else if (/\s/.test(line[i])) {
            // Skip whitespace
            i--;
        } else {
            break;
        }
    }

    // Look forwards for dot-separated identifiers
    let pathEnd = wordEnd;
    i = wordEnd;
    while (i < line.length) {
        if (line[i] === '.') {
            // Check if there's an identifier after the dot
            const afterDot = i + 1;
            if (afterDot < line.length && /[\w$]/.test(line[afterDot])) {
                // Find the end of that identifier
                let identEnd = afterDot + 1;
                while (identEnd < line.length && /[\w$]/.test(line[identEnd])) {
                    identEnd++;
                }
                pathEnd = identEnd;
                i = identEnd;
            } else {
                break;
            }
        } else if (/\s/.test(line[i])) {
            // Skip whitespace
            i++;
        } else {
            break;
        }
    }

    const fullPath = line.slice(pathStart, pathEnd);
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

            const { fullPath, word } = resolved;

            // Check if the word is a DSL type
            if (isDslType(word)) {
                deps.openHelpForSymbol('type', word);
                // Return null to prevent Monaco from showing "no definition found"
                // The help window will open as a side effect
                return null;
            }

            // Check if the full path is a module name (e.g., "osc.sine", "clock")
            if (deps.moduleNames.has(fullPath)) {
                deps.openHelpForSymbol('module', fullPath);
                return null;
            }

            // Check if just the word is a module name (root-level modules)
            if (deps.moduleNames.has(word)) {
                deps.openHelpForSymbol('module', word);
                return null;
            }

            // Check if the word is a namespace (e.g., "osc", "env")
            if (deps.namespaceNames.has(word)) {
                deps.openHelpForSymbol('namespace', word);
                return null;
            }

            // Not a DSL symbol - let TypeScript handle it
            return null;
        }
    };

    const disposable = monaco.languages.registerDefinitionProvider('javascript', provider);
    return { dispose: () => disposable.dispose() };
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
