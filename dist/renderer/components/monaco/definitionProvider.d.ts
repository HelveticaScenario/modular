/**
 * Custom Monaco definition provider that intercepts Go to Definition for DSL symbols
 * and opens the help window instead of navigating to the generated .d.ts file.
 */
import type { Monaco } from '../../hooks/useCustomMonaco';
import type { editor, Position } from 'monaco-editor';
export interface DefinitionProviderDeps {
    /** Set of module factory names (including namespaced ones like "osc.sine") */
    moduleNames: Set<string>;
    /** Set of namespace names (like "osc", "env", "filter") */
    namespaceNames: Set<string>;
}
/**
 * Register a custom definition provider that opens the help window
 * for DSL types, modules, and namespaces.
 */
export declare function registerDslDefinitionProvider(monaco: Monaco, deps: DefinitionProviderDeps): {
    dispose: () => void;
};
export interface DslSymbolMatch {
    symbolType: 'type' | 'module' | 'namespace';
    symbolName: string;
}
/**
 * Given a resolved dotted path, determine if it matches a DSL symbol.
 * Returns the match details or null.
 */
export declare function resolveDslSymbol(resolved: {
    fullPath: string;
    word: string;
}, moduleNames: Set<string>, namespaceNames: Set<string>): DslSymbolMatch | null;
/**
 * Resolve the DSL symbol at a given model position.
 * Convenience wrapper combining path resolution and symbol matching.
 */
export declare function resolveDslSymbolAtPosition(model: editor.ITextModel, position: Position, moduleNames: Set<string>, namespaceNames: Set<string>): DslSymbolMatch | null;
/**
 * Build sets of module names and namespace names from schemas.
 */
export declare function buildSymbolSets(schemas: Array<{
    name: string;
}>): {
    moduleNames: Set<string>;
    namespaceNames: Set<string>;
};
