/**
 * Source Span Analyzer using ts-morph
 *
 * Parses DSL source code and extracts absolute character offsets for literal
 * arguments in module factory calls. The registry is keyed by call-site
 * (line:column) for lookup from factory functions at runtime.
 *
 * Additionally builds an interpolation resolution map for template literals
 * containing const variable references. When a template like `${root} e4 g4`
 * interpolates a const string, the resolution map records the const's literal
 * span so that highlights landing inside the interpolation result can be
 * redirected to the original const declaration site. This works recursively
 * for nested template const chains.
 */
import type { ModuleSchema } from '@modular/core';
export type { SourceSpan, ResolvedInterpolation, InterpolationResolutionMap } from '../../shared/dsl/spanTypes';
export { setActiveInterpolationResolutions, getActiveInterpolationResolutions } from '../../shared/dsl/spanTypes';
import type { SourceSpan, InterpolationResolutionMap } from '../../shared/dsl/spanTypes';
/**
 * Registry entry for a single call expression's argument spans
 */
export interface CallSiteSpans {
    /** Spans for each positional argument, keyed by argument name */
    args: Map<string, SourceSpan>;
    /** The module type being called (e.g., "seq", "sine") */
    moduleType: string;
}
/**
 * Call site key using line:column format (1-based line, 0-based column)
 * This matches the format produced by Error.captureStackTrace
 */
export type CallSiteKey = `${number}:${number}`;
/**
 * Registry mapping call sites to their argument spans
 */
export type SpanRegistry = Map<CallSiteKey, CallSiteSpans>;
/**
 * Result of analyzing source spans, including both the span registry
 * (for Rust-side argument highlighting) and the interpolation resolution map
 * (for TS-side redirect of highlights into const declarations).
 */
export interface AnalysisResult {
    /** Registry mapping call sites to argument spans */
    registry: SpanRegistry;
    /** Map from argument span key to resolved interpolations within that span */
    interpolationResolutions: InterpolationResolutionMap;
}
/**
 * Analyze DSL source code and build a span registry for argument locations.
 *
 * @param source - The DSL source code to analyze
 * @param schemas - Module schemas to determine which calls to track
 * @param lineOffset - Line offset to add (for wrapped code in new Function)
 * @returns Analysis result with span registry and interpolation resolution map
 */
export declare function analyzeSourceSpans(source: string, schemas: ModuleSchema[], lineOffset?: number, firstLineColumnOffset?: number): AnalysisResult;
/**
 * Create an empty span registry (for when analysis is not needed)
 */
export declare function emptySpanRegistry(): SpanRegistry;
/**
 * Debug helper: print registry contents
 */
export declare function debugPrintRegistry(registry: SpanRegistry): void;
