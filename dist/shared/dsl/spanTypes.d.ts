/**
 * Shared types and state for source span analysis.
 *
 * This file is deliberately free of ts-morph (or any Node.js-only) imports
 * so it can be consumed from the renderer process without pulling in
 * Node.js built-ins via webpack.
 */
/**
 * Span representing a character range in source code
 */
export interface SourceSpan {
    /** Absolute start offset (0-based) */
    start: number;
    /** Absolute end offset (exclusive) */
    end: number;
}
/**
 * Describes a resolved interpolation within a template literal.
 * When `${someConst}` appears in a template and `someConst` is a const with a
 * literal initializer, this records the mapping from the interpolation's
 * evaluated position range to the const literal's document span.
 *
 * For recursive resolution (const with template literal initializer that itself
 * has const interpolations), `nestedResolutions` contains the inner resolutions.
 */
export interface ResolvedInterpolation {
    /** Evaluated character offset where this interpolation's result starts */
    evaluatedStart: number;
    /** Length of the interpolation's evaluated result */
    evaluatedLength: number;
    /** Document span of the const literal (including quotes) */
    constLiteralSpan: SourceSpan;
    /** Nested resolutions within the const literal (for recursive template consts) */
    nestedResolutions?: ResolvedInterpolation[];
}
/**
 * Map from argument span key (`${start}:${end}`) to its resolved interpolations.
 * The key is the SourceSpan of the argument as recorded in the span registry.
 */
export type InterpolationResolutionMap = Map<string, ResolvedInterpolation[]>;
/**
 * Set the active interpolation resolution map.
 * Called by executor.ts after analysis and before/after execution.
 */
export declare function setActiveInterpolationResolutions(map: InterpolationResolutionMap | null): void;
/**
 * Get the active interpolation resolution map.
 * Read by moduleStateTracking.ts during decoration polling.
 */
export declare function getActiveInterpolationResolutions(): InterpolationResolutionMap | null;
