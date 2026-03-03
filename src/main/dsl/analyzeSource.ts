/**
 * Source Analysis Orchestration
 *
 * Thin entry point that accepts a pre-parsed ts-morph SourceFile and
 * delegates to the argument span analyzer and call site span analyzer.
 * Returns a combined AnalysisResult.
 */

import type { SourceFile } from 'ts-morph';
import type { ModuleSchema } from '@modular/core';

import type { AnalysisResult } from './sourceAnalysisTypes';
import { analyzeArgumentSpans } from './argumentSpanAnalyzer';
import { analyzeCallSiteSpans } from './callSiteSpanAnalyzer';

// Re-export types and utilities so existing consumers can import from here
export type {
    SourceSpan,
    ResolvedInterpolation,
    InterpolationResolutionMap,
    CallSiteSpans,
    CallSiteKey,
    SpanRegistry,
    CallExpressionSpan,
    CallSiteSpanRegistry,
    AnalysisResult,
} from './sourceAnalysisTypes';
export {
    setActiveInterpolationResolutions,
    getActiveInterpolationResolutions,
} from './sourceAnalysisTypes';

/**
 * Analyze DSL source code and build registries for argument locations
 * and call expression spans.
 *
 * @param sourceFile - A pre-parsed ts-morph SourceFile to analyze
 * @param schemas - Module schemas to determine which calls to track
 * @returns Analysis result with span registry, interpolation resolution map,
 *          and call site span registry
 */
export function analyzeSourceSpans(
    sourceFile: SourceFile,
    schemas: ModuleSchema[],
): AnalysisResult {
    // Pass 1: Argument spans for factory calls
    const { registry, interpolationResolutions } = analyzeArgumentSpans(
        sourceFile,
        schemas,
    );

    // Pass 2: Full call expression spans for DSL methods
    const callSiteSpans = analyzeCallSiteSpans(sourceFile);

    return { registry, interpolationResolutions, callSiteSpans };
}
