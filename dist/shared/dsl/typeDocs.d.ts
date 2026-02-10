/**
 * Shared type documentation for DSL types.
 * Used by both TypeScript lib generator (JSDoc) and HelpWindow (rendered docs).
 */
export interface TypeMethod {
    name: string;
    signature: string;
    description: string;
    example?: string;
}
export interface TypeDocumentation {
    name: string;
    description: string;
    definition?: string;
    examples: string[];
    seeAlso: string[];
    methods?: TypeMethod[];
}
/**
 * All DSL type names that should be linkified in documentation.
 */
export declare const DSL_TYPE_NAMES: readonly ["Signal", "PolySignal", "ModuleOutput", "ModuleOutputWithRange", "Collection", "CollectionWithRange", "Note", "HZ", "MidiNote", "Scale", "StereoOutOptions"];
export type DslTypeName = (typeof DSL_TYPE_NAMES)[number];
/**
 * Comprehensive documentation for all DSL types.
 */
export declare const TYPE_DOCS: Record<DslTypeName, TypeDocumentation>;
/**
 * Check if a string is a known DSL type name.
 */
export declare function isDslType(name: string): name is DslTypeName;
/**
 * Get documentation for a DSL type by name.
 */
export declare function getTypeDoc(name: string): TypeDocumentation | undefined;
