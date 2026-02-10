import { ModuleSchema } from '@modular/core';
import { GraphBuilder, ModuleOutput, Collection, CollectionWithRange } from './GraphBuilder';
import type { SourceSpan } from '../../shared/dsl/spanTypes';
import type { SpanRegistry } from './sourceSpanAnalyzer';
/**
 * Line offset for DSL code wrapper.
 * The executePatchScript creates a function body with 'use strict' which adds lines
 * before user code. This offset is set by executor.ts at runtime.
 */
export declare let DSL_WRAPPER_LINE_OFFSET: number;
/**
 * Configure the line offset for DSL code wrapper.
 */
export declare function setDSLWrapperLineOffset(offset: number): void;
/**
 * Set the active span registry for argument span capture.
 * Called by executor.ts before and after DSL execution.
 */
export declare function setActiveSpanRegistry(registry: SpanRegistry | null): void;
/**
 * Type for argument spans attached to module params
 */
export interface ArgumentSpans {
    [argName: string]: SourceSpan;
}
type SingleOutput = ModuleOutput;
type PolyOutput = Collection | CollectionWithRange;
type MultiOutput = (SingleOutput | PolyOutput) & Record<string, ModuleOutput | Collection | CollectionWithRange>;
type ModuleReturn = SingleOutput | PolyOutput | MultiOutput;
type FactoryFunction = (...args: any[]) => ModuleReturn;
type NamespaceTree = {
    [key: string]: NamespaceTree | FactoryFunction;
};
/**
 * DSL Context holds the builder and provides factory functions
 */
export declare class DSLContext {
    factories: Record<string, FactoryFunction>;
    namespaceTree: NamespaceTree;
    private builder;
    constructor(schemas: ModuleSchema[]);
    /**
     * Create a module factory function that returns outputs directly
     */
    private createFactory;
    /**
     * Get the builder instance
     */
    getBuilder(): GraphBuilder;
    scope<T extends ModuleOutput | Collection | CollectionWithRange>(target: T, config?: {
        msPerFrame?: number;
        triggerThreshold?: number;
        scale?: number;
    }): T;
}
export declare function hz(frequency: number): number;
/**
 * Note name to V/oct conversion
 * Supports notes like "c4", "c#4", "db4", etc.
 */
export declare function note(noteName: string): number;
/**
 * Convert BPM (beats per minute) to V/oct frequency
 * BPM is tempo, where 1 beat = 1 quarter note
 * At 120 BPM, that's 2 beats per second = 2 Hz
 */
export declare function bpm(beatsPerMinute: number): number;
export {};
