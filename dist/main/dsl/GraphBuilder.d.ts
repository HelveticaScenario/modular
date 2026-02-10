import type { ModuleSchema, ModuleState, PatchGraph } from '@modular/core';
import type { ProcessedModuleSchema } from './paramsSchema';
import z from 'zod';
export declare const PORT_MAX_CHANNELS = 16;
export interface OutputSchemaWithRange {
    name: string;
    description: string;
    polyphonic?: boolean;
    minValue?: number;
    maxValue?: number;
}
declare const ResolvedModuleOutput: z.ZodObject<{
    type: z.ZodLiteral<"cable">;
    module: z.ZodString;
    port: z.ZodString;
    channel: z.ZodOptional<z.ZodNumber>;
}, z.core.$strip>;
export type ResolvedModuleOutput = z.infer<typeof ResolvedModuleOutput>;
export type OrArray<T> = T | T[];
export type Signal = number | string | ModuleOutput;
export type PolySignal = OrArray<Signal> | Iterable<ModuleOutput>;
/** Options for stereo output routing */
export interface StereoOutOptions {
    /** Base output channel (0-14, default 0). Left plays on baseChannel, right on baseChannel+1 */
    baseChannel?: number;
    /** Output gain. If set, a scaleAndShift module is added after the stereo mix */
    gain?: PolySignal;
    /** Pan position (-5 = left, 0 = center, +5 = right). Default 0 */
    pan?: PolySignal;
    /** Stereo width/spread (0 = no spread, 5 = full spread). Default 0 */
    width?: Signal;
}
/** Options for mono output routing */
export interface MonoOutOptions {
    /** Output channel (0-15, default 0) */
    channel?: number;
    /** Output gain. If set, a scaleAndShift module is added after the mix */
    gain?: PolySignal;
}
/** Internal storage for a stereo output group */
export interface StereoOutGroup {
    type: 'stereo';
    outputs: ModuleOutput[];
    gain?: PolySignal;
    pan?: PolySignal;
    width?: PolySignal;
}
/** Internal storage for a mono output group */
export interface MonoOutGroup {
    type: 'mono';
    outputs: ModuleOutput[];
    gain?: PolySignal;
}
export type OutGroup = StereoOutGroup | MonoOutGroup;
/**
 * BaseCollection provides iterable, indexable container for ModuleOutput arrays
 * with chainable DSP methods (gain, shift, scope, out).
 */
export declare class BaseCollection<T extends ModuleOutput> {
    [index: number]: T;
    readonly items: T[];
    constructor(...args: T[]);
    get length(): number;
    [Symbol.iterator](): Iterator<T>;
    /**
     * Scale all outputs by a factor
     */
    gain(factor: PolySignal): Collection;
    /**
     * Shift all outputs by an offset
     */
    shift(offset: PolySignal): Collection;
    /**
     * Add scope visualization for the first output in the collection
     */
    scope(config?: {
        msPerFrame?: number;
        triggerThreshold?: number;
        range?: [number, number];
    }): this;
    /**
     * Send all outputs to speakers as stereo
     * @param baseChannel - Base output channel (0-15, default 0)
     * @param options.gain - Output gain (adds scaleAndShift after stereo mix)
     * @param options.pan - Pan position (-5 = left, 0 = center, +5 = right)
     * @param options.width - Stereo width/spread (0 = no spread, 5 = full spread)
     */
    out(baseChannel?: number, options?: StereoOutOptions): this;
    /**
     * Send all outputs to speakers as mono
     * @param channel - Output channel (0-15, default 0)
     * @param gain - Output gain
     */
    outMono(channel?: number, gain?: PolySignal): this;
    toString(): string;
}
/**
 * Collection of ModuleOutput instances.
 * Use .range(inMin, inMax, outMin, outMax) to remap with explicit input range.
 */
export declare class Collection extends BaseCollection<ModuleOutput> {
    constructor(...args: ModuleOutput[]);
    /**
     * Remap outputs from explicit input range to output range
     */
    range(inMin: PolySignal, inMax: PolySignal, outMin: PolySignal, outMax: PolySignal): Collection;
}
/**
 * Collection of ModuleOutputWithRange instances.
 * Use .range(outMin, outMax) to remap using stored min/max values.
 */
export declare class CollectionWithRange extends BaseCollection<ModuleOutputWithRange> {
    constructor(...args: ModuleOutputWithRange[]);
    /**
     * Remap outputs from their known range to a new output range
     */
    range(outMin: PolySignal, outMax: PolySignal): Collection;
}
/**
 * Create a Collection from ModuleOutput instances
 */
export declare const $c: (...args: ModuleOutput[]) => Collection;
/**
 * Create a CollectionWithRange from ModuleOutputWithRange instances
 */
export declare const $r: (...args: ModuleOutputWithRange[]) => CollectionWithRange;
/**
 * Factory function type for creating modules via DSL.
 * Returns the module's output(s) directly rather than the ModuleNode.
 */
export type FactoryFunction = (...args: unknown[]) => ModuleOutput | Collection | CollectionWithRange;
/**
 * Source location information for mapping validation errors back to DSL code.
 */
export interface SourceLocation {
    /** 1-based line number in the DSL source */
    line: number;
    /** 1-based column number in the DSL source */
    column: number;
    /** Whether the module ID was explicitly set by the user */
    idIsExplicit: boolean;
}
/**
 * GraphBuilder manages the construction of a PatchGraph from DSL code.
 * It tracks modules, generates deterministic IDs, and builds the final graph.
 *
 * Note: Factory functions add overhead from channel count derivation but provide
 * consistency across all module creation paths.
 */
export declare class GraphBuilder {
    private modules;
    private counters;
    private schemas;
    private schemaByName;
    private scopes;
    /** Output groups keyed by baseChannel */
    private outGroups;
    private factoryRegistry;
    private sourceLocationMap;
    /** Track all deferred outputs for string replacement during toPatch */
    private deferredOutputs;
    /** Global tempo signal for ROOT_CLOCK (default: bpm(120) = hz(2)) */
    private tempo;
    /** Global output gain signal (default: 0.5) */
    private outputGain;
    constructor(schemas: ModuleSchema[]);
    /**
     * Generate a deterministic ID for a module type
     */
    private generateId;
    /**
     * Add or update a module in the graph
     */
    addModule(moduleType: string, explicitId?: string, sourceLocation?: {
        line: number;
        column: number;
    }): ModuleNode;
    /**
     * Get a module by ID
     */
    getModule(id: string): ModuleState | undefined;
    /**
     * Set a parameter value for a module
     */
    setParam(moduleId: string, paramName: string, value: unknown): void;
    /**
     * Register factory functions for late binding.
     * Called by DSLContext after factory creation to enable internal factory usage.
     */
    setFactoryRegistry(factories: Map<string, FactoryFunction>): void;
    /**
     * Set the global tempo for ROOT_CLOCK
     * @param tempo - Signal value for tempo (use bpm() or hz() helpers)
     */
    setTempo(tempo: Signal): void;
    /**
     * Set the global output gain
     * @param gain - Signal value for output gain (2.5 is default, 5.0 is unity)
     */
    setOutputGain(gain: Signal): void;
    /**
     * Get a factory function by module type name.
     * Returns undefined if factories haven't been registered yet.
     */
    getFactory(moduleType: string): FactoryFunction | undefined;
    /**
     * Build the final PatchGraph
     *
     * Note: Uses factory functions for signal/mix modules for consistency,
     * which adds overhead from channel count derivation on every patch build.
     */
    toPatch(): PatchGraph;
    /**
     * Reset the builder state
     */
    reset(): void;
    /**
     * Get the source location map for error reporting.
     * Maps module IDs to their source locations in the DSL code.
     */
    getSourceLocationMap(): Map<string, SourceLocation>;
    /**
     * Register module output(s) for stereo output routing
     */
    addOut(value: ModuleOutput | ModuleOutput[], options?: StereoOutOptions): void;
    /**
     * Register module output(s) for mono output routing
     */
    addOutMono(value: ModuleOutput | ModuleOutput[], options?: MonoOutOptions): void;
    /**
     * Register a deferred output for tracking.
     * Called by DeferredModuleOutput constructor.
     */
    registerDeferred(deferred: DeferredModuleOutput): void;
    addScope(value: ModuleOutput | ModuleOutput[], config?: {
        msPerFrame?: number;
        triggerThreshold?: number;
        range?: [number, number];
    }): void;
}
type Value = number | ModuleOutput | ModuleOutput[] | ModuleNode | ModuleNode[];
/**
 * ModuleNode represents a module instance in the DSL (internal use only)
 * Users interact with ModuleOutput directly, not ModuleNode
 */
export declare class ModuleNode {
    readonly builder: GraphBuilder;
    readonly id: string;
    readonly moduleType: string;
    readonly schema: ProcessedModuleSchema;
    private _channelCount;
    constructor(builder: GraphBuilder, id: string, moduleType: string, schema: ProcessedModuleSchema);
    /**
     * Get the number of channels this module produces.
     * Set by Rust-side derivation via _setDerivedChannelCount.
     */
    get channelCount(): number;
    _setParam(paramName: string, value: unknown): this;
    /**
     * Get a snapshot of the current params for this module.
     * Used for Rust-side channel count derivation.
     */
    getParamsSnapshot(): Record<string, unknown>;
    /**
     * Set the channel count derived from Rust-side analysis.
     */
    _setDerivedChannelCount(channels: number): void;
    /**
     * Get an output port of this module
     */
    _output(portName: string, polyphonic?: boolean): ModuleOutput | Collection | ModuleOutputWithRange | CollectionWithRange;
}
/**
 * ModuleOutput represents an output port that can be connected or transformed
 */
export declare class ModuleOutput {
    readonly builder: GraphBuilder;
    readonly moduleId: string;
    readonly portName: string;
    readonly channel: number;
    constructor(builder: GraphBuilder, moduleId: string, portName: string, channel?: number);
    /**
     * Scale this output by a factor
     */
    gain(factor: Value): ModuleOutput;
    /**
     * Shift this output by an offset
     */
    shift(offset: Value): ModuleOutput;
    scope(config?: {
        msPerFrame?: number;
        triggerThreshold?: number;
        range?: [number, number];
    }): this;
    /**
     * Send this output to speakers as stereo
     * @param baseChannel - Base output channel (0-15, default 0)
     * @param options.gain - Output gain (adds util.scaleAndShift after stereo mix)
     * @param options.pan - Pan position (-5 = left, 0 = center, +5 = right)
     * @param options.width - Stereo width/spread (0 = no spread, 5 = full spread)
     */
    out(baseChannel?: number, options?: StereoOutOptions): this;
    /**
     * Send this output to speakers as mono
     * @param channel - Output channel (0-15, default 0)
     * @param gain - Output gain
     */
    outMono(channel?: number, gain?: PolySignal): this;
    toString(): string;
}
/**
 * ModuleOutputWithRange extends ModuleOutput with known output range metadata.
 * Provides .range() method to easily remap the output to a new range.
 */
export declare class ModuleOutputWithRange extends ModuleOutput {
    readonly minValue: number;
    readonly maxValue: number;
    constructor(builder: GraphBuilder, moduleId: string, portName: string, channel: number | undefined, minValue: number, maxValue: number);
    /**
     * Remap this output from its known range to a new range.
     * Creates a remap module internally.
     */
    range(outMin: Value, outMax: Value): ModuleOutput;
}
/**
 * DeferredModuleOutput is a placeholder for a signal that will be assigned later.
 * Useful for feedback loops and forward references in the DSL.
 * Supports the same chainable methods as ModuleOutput (gain, shift, scope, out, outMono).
 * Transforms are stored and applied when the deferred signal is resolved.
 */
export declare class DeferredModuleOutput extends ModuleOutput {
    private resolvedModuleOutput;
    private resolving;
    static idCounter: number;
    constructor(builder: GraphBuilder);
    /**
     * Set the actual signal this deferred output should resolve to.
     * @param signal - The signal to resolve to (number, string, or ModuleOutput)
     */
    set(signal: ModuleOutput): void;
    /**
     * Resolve this deferred output to an actual ModuleOutput.
     * @returns The resolved ModuleOutput, or null if not set.
     */
    resolve(): ModuleOutput | null;
}
/**
 * DeferredCollection is a collection of DeferredModuleOutput instances.
 * Provides a .set() method to assign ModuleOutputs to all contained deferred outputs.
 */
export declare class DeferredCollection extends BaseCollection<DeferredModuleOutput> {
    constructor(...args: DeferredModuleOutput[]);
    /**
     * Set the values for all deferred outputs in this collection.
     * @param outputs - A ModuleOutput or iterable of ModuleOutputs to distribute across outputs
     */
    set(outputs: ModuleOutput | Iterable<ModuleOutput>): void;
}
type Replacer = (key: string, value: unknown) => unknown;
export declare function replaceValues(input: unknown, replacer: Replacer): unknown;
export declare function replaceSignals(input: unknown): unknown;
/**
 * Recursively replace deferred output strings with resolved output strings in params.
 * This handles cases where a DeferredModuleOutput was stringified (e.g., in pattern strings).
 */
export declare function replaceDeferredStrings(input: unknown, deferredStringMap: Map<string, string | null>): unknown;
export {};
