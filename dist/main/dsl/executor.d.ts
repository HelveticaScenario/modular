import { ModuleSchema, PatchGraph } from '@modular/core';
import { SourceLocation } from './GraphBuilder';
import type { InterpolationResolutionMap } from '../../shared/dsl/spanTypes';
import type { SliderDefinition } from '../../shared/dsl/sliderTypes';
/**
 * Result of executing a DSL script.
 */
export interface DSLExecutionResult {
    /** The generated patch graph */
    patch: PatchGraph;
    /** Map from module ID to source location in DSL code */
    sourceLocationMap: Map<string, SourceLocation>;
    /** Interpolation resolution map for template literal const redirects */
    interpolationResolutions: InterpolationResolutionMap;
    /** Slider definitions created by slider() DSL function calls */
    sliders: SliderDefinition[];
}
/**
 * Execute a DSL script and return the resulting PatchGraph with source locations.
 */
export declare function executePatchScript(source: string, schemas: ModuleSchema[]): DSLExecutionResult;
/**
 * Validate DSL script syntax without executing
 */
export declare function validateDSLSyntax(source: string): {
    valid: boolean;
    error?: string;
};
