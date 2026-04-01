/**
 * Shared types for source span analysis.
 *
 * Types used across the argument span analyzer, call site span analyzer,
 * and orchestration layer. Re-exports renderer-safe types from spanTypes.ts.
 */

import type {
    InterpolationResolutionMap,
    SourceSpan,
} from '../../shared/dsl/spanTypes';

// Re-export shared types/state from spanTypes (which has no Node.js dependencies)
export type {
    SourceSpan,
    ResolvedInterpolation,
    InterpolationResolutionMap,
} from '../../shared/dsl/spanTypes';
export {
    setActiveInterpolationResolutions,
    getActiveInterpolationResolutions,
} from '../../shared/dsl/spanTypes';

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
 * Call site key using line:column format (both 1-based).
 * This matches the format produced by both ts-morph and V8 Error.captureStackTrace.
 */
export type CallSiteKey = `${number}:${number}`;

/**
 * Registry mapping call sites to their argument spans
 */
export type SpanRegistry = Map<CallSiteKey, CallSiteSpans>;

/**
 * Full source span of a call expression, in terms of editor line numbers.
 * Used to position view zones after the closing paren of multi-line calls.
 */
export interface CallExpressionSpan {
    /** 1-based start line of the call expression (in user source, before wrapper offset) */
    startLine: number;
    /** 1-based end line of the call expression (line containing the closing paren) */
    endLine: number;
}

/**
 * Registry mapping call site keys to their full expression spans.
 * Covers DSL methods like .scope() and $slider() whose view zones need
 * to know the end line of the entire call expression.
 */
export type CallSiteSpanRegistry = Map<CallSiteKey, CallExpressionSpan>;

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
    /** Registry mapping call site keys to full call expression spans (start/end lines) */
    callSiteSpans: CallSiteSpanRegistry;
}
