import type {
    ModuleSchema,
    ModuleState,
    PatchGraph,
    Scope,
} from '@modular/core';
import type { ProcessedModuleSchema } from './paramsSchema';
import { processSchemas } from './paramsSchema';

const PORT_MAX_CHANNELS = 16;

// Extended OutputSchema interface that includes optional range
interface OutputSchemaWithRange {
    name: string;
    description: string;
    polyphonic?: boolean;
    minValue?: number;
    maxValue?: number;
}

// Type definitions for Collection system
type OrArray<T> = T | T[];
type Signal = number | string | ModuleOutput;
type PolySignal = OrArray<Signal> | Iterable<ModuleOutput>;

/**
 * BaseCollection provides iterable, indexable container for ModuleOutput arrays
 * with chainable DSP methods (gain, shift, scope, out).
 */
class BaseCollection<T extends ModuleOutput> {
    [index: number]: T;
    readonly items: T[] = [];

    constructor(...args: T[]) {
        this.items.push(...args);
        for (const [i, arg] of args.entries()) {
            this[i] = arg;
        }
    }

    get length(): number {
        return this.items.length;
    }

    [Symbol.iterator](): Iterator<T> {
        return this.items.values();
    }

    /**
     * Scale all outputs by a factor
     */
    gain(factor: PolySignal): Collection {
        if (this.items.length === 0) return new Collection();
        const factory = this.items[0].builder.getFactory('scaleAndShift');
        if (!factory) {
            throw new Error('Factory for scaleAndShift not registered');
        }
        return factory(this.items, factor) as Collection;
    }

    /**
     * Shift all outputs by an offset
     */
    shift(offset: PolySignal): Collection {
        if (this.items.length === 0) return new Collection();
        const factory = this.items[0].builder.getFactory('scaleAndShift');
        if (!factory) {
            throw new Error('Factory for scaleAndShift not registered');
        }
        return factory(this.items, undefined, offset) as Collection;
    }

    /**
     * Add scope visualization for the first output in the collection
     */
    scope(msPerFrame: number = 500, triggerThreshold?: number): this {
        if (this.items.length > 0) {
            this.items[0].builder.addScope(this.items[0], msPerFrame, triggerThreshold);
        }
        return this;
    }

    /**
     * Send all outputs to speakers
     */
    out(mode?: 'm'): this {
        if (this.items.length > 0 && mode !== 'm') {
            this.items[0].builder.addOut([...this.items]);
        }
        return this;
    }
}

/**
 * Collection of ModuleOutput instances.
 * Use .range(inMin, inMax, outMin, outMax) to remap with explicit input range.
 */
export class Collection extends BaseCollection<ModuleOutput> {
    constructor(...args: ModuleOutput[]) {
        super(...args);
    }

    /**
     * Remap outputs from explicit input range to output range
     */
    range(inMin: PolySignal, inMax: PolySignal, outMin: PolySignal, outMax: PolySignal): Collection {
        if (this.items.length === 0) return new Collection();
        const factory = this.items[0].builder.getFactory('remap');
        if (!factory) {
            throw new Error('Factory for remap not registered');
        }
        return factory(this.items,
            inMin,
            inMax,
            outMin,
            outMax,
        ) as Collection;
    }
}

/**
 * Collection of ModuleOutputWithRange instances.
 * Use .range(outMin, outMax) to remap using stored min/max values.
 */
export class CollectionWithRange extends BaseCollection<ModuleOutputWithRange> {
    constructor(...args: ModuleOutputWithRange[]) {
        super(...args);
    }

    /**
     * Remap outputs from their known range to a new output range
     */
    range(outMin: PolySignal, outMax: PolySignal): Collection {
        if (this.items.length === 0) return new Collection();
        const factory = this.items[0].builder.getFactory('remap');
        if (!factory) {
            throw new Error('Factory for remap not registered');
        }
        return factory(
            this.items,
            this.items.map((o) => o.minValue),
            this.items.map((o) => o.maxValue),
            outMin,
            outMax,
        ) as Collection;
    }
}

/**
 * Create a Collection from ModuleOutput instances
 */
export const $ = (...args: ModuleOutput[]): Collection => new Collection(...args);

/**
 * Create a CollectionWithRange from ModuleOutputWithRange instances
 */
export const $r = (...args: ModuleOutputWithRange[]): CollectionWithRange => new CollectionWithRange(...args);

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
export class GraphBuilder {
    private modules: Map<string, ModuleState> = new Map();
    private counters: Map<string, number> = new Map();
    private schemas: ProcessedModuleSchema[] = [];
    private schemaByName: Map<string, ProcessedModuleSchema> = new Map();
    private scopes: Scope[] = [];
    private outModules: ModuleOutput[] = [];
    private factoryRegistry: Map<string, FactoryFunction> = new Map();
    private sourceLocationMap: Map<string, SourceLocation> = new Map();

    constructor(schemas: ModuleSchema[]) {
        this.schemas = processSchemas(schemas);
        this.schemaByName = new Map(this.schemas.map((s) => [s.name, s]));
    }

    /**
     * Generate a deterministic ID for a module type
     */
    private generateId(moduleType: string, explicitId?: string): string {
        if (explicitId) {
            return explicitId;
        }

        let counter = (this.counters.get(moduleType) || 0) + 1;
        let id = `${moduleType}-${counter}`;

        // If the generated ID is already taken (e.g. by an explicit ID),
        // keep incrementing until we find a free one.
        while (this.modules.has(id)) {
            counter++;
            id = `${moduleType}-${counter}`;
        }

        this.counters.set(moduleType, counter);
        return id;
    }

    /**
     * Add or update a module in the graph
     */
    addModule(moduleType: string, explicitId?: string, sourceLocation?: { line: number; column: number }): ModuleNode {
        const id = this.generateId(moduleType, explicitId);

        if (this.modules.has(id)) {
            throw new Error(`Duplicate module id: ${id}`);
        }

        // Check if module type exists in schemas
        const schema = this.schemaByName.get(moduleType);
        if (!schema) {
            throw new Error(`Unknown module type: ${moduleType}`);
        }

        // Store source location for error mapping
        if (sourceLocation) {
            this.sourceLocationMap.set(id, {
                line: sourceLocation.line,
                column: sourceLocation.column,
                idIsExplicit: Boolean(explicitId),
            });
        }

        // Initialize module params: default all signal params to disconnected.
        // Other params are left unset unless the DSL sets them explicitly.
        const params: Record<string, unknown> = {};
        for (const param of schema.params) {
            if (param.kind === 'signal' || param.kind === 'polySignal') {
                params[param.name] = { type: 'disconnected' };
            } else if (param.kind === 'signalArray') {
                // Required arrays (e.g. sum.signals) should be valid by default.
                params[param.name] = [];
            }
        }

        const moduleState: ModuleState = {
            id,
            moduleType,
            idIsExplicit: Boolean(explicitId),
            params,
        };

        this.modules.set(id, moduleState);
        return new ModuleNode(this, id, moduleType, schema);
    }

    /**
     * Get a module by ID
     */
    getModule(id: string): ModuleState | undefined {
        return this.modules.get(id);
    }

    /**
     * Set a parameter value for a module
     */
    setParam(moduleId: string, paramName: string, value: unknown): void {
        const module = this.modules.get(moduleId);
        if (!module) {
            throw new Error(`Module not found: ${moduleId}`);
        }
        module.params[paramName] = value;
    }

    /**
     * Register factory functions for late binding.
     * Called by DSLContext after factory creation to enable internal factory usage.
     */
    setFactoryRegistry(factories: Map<string, FactoryFunction>): void {
        this.factoryRegistry = factories;
    }

    /**
     * Get a factory function by module type name.
     * Returns undefined if factories haven't been registered yet.
     */
    getFactory(moduleType: string): FactoryFunction | undefined {
        return this.factoryRegistry.get(moduleType);
    }

    /**
     * Build the final PatchGraph
     * 
     * Note: Uses factory functions for signal/mix modules for consistency,
     * which adds overhead from channel count derivation on every patch build.
     */
    toPatch(): PatchGraph {
        // If there are any modules registered with out(), create a mix module
        // and connect to root signal
        if (this.outModules.length > 0) {
            const mixFactory = this.getFactory('mix');
            const signalFactory = this.getFactory('signal');

            if (!mixFactory || !signalFactory) {
                throw new Error('Factory for mix or signal not registered');
            }

            // Create mix module with inputs
            const mixOutput = mixFactory(this.outModules) as ModuleOutput;

            // Create root signal module with the mix output as source
            signalFactory(mixOutput, { id: 'ROOT_OUTPUT' });
        } else {
            // No outputs registered - create empty root signal
            const signalFactory = this.getFactory('signal');
            if (!signalFactory) {
                throw new Error('Factory for signal not registered');
            }
            signalFactory(undefined, { id: 'ROOT_OUTPUT' });
        }

        const ret = {
            modules: Array.from(this.modules.values()).map((m) => ({
                ...m,
                params: replaceSignals(m.params),
            })),
            scopes: Array.from(this.scopes),
        };

        return ret;
    }

    /**
     * Reset the builder state
     */
    reset(): void {
        this.modules.clear();
        this.scopes = [];
        this.counters.clear();
        this.outModules = [];
        this.sourceLocationMap.clear();
    }

    /**
     * Get the source location map for error reporting.
     * Maps module IDs to their source locations in the DSL code.
     */
    getSourceLocationMap(): Map<string, SourceLocation> {
        return this.sourceLocationMap;
    }

    /**
     * Register a module output to be sent to speakers
     */
    addOut(value: ModuleOutput | ModuleOutput[]): void {
        if (Array.isArray(value)) {
            this.outModules.push(...value);
        } else {
            this.outModules.push(value);
        }
    }

    addScope(
        value: ModuleOutput | ModuleOutput[],
        msPerFrame: number = 500,
        triggerThreshold?: number,
    ) {
        let realTriggerThreshold: number | undefined =
            triggerThreshold !== undefined
                ? triggerThreshold * 1000
                : undefined;
        let output: ModuleOutput;
        if (Array.isArray(value)) {
            output = value[0];
        } else {
            output = value;
        }
        this.scopes.push({
            item: {
                type: 'ModuleOutput',
                moduleId: output.moduleId,
                portName: output.portName,
                channel: output.channel,
            },
            msPerFrame,
            triggerThreshold: realTriggerThreshold,
        });
    }
}

type Value = number | ModuleOutput | ModuleOutput[] | ModuleNode | ModuleNode[];

/**
 * ModuleNode represents a module instance in the DSL (internal use only)
 * Users interact with ModuleOutput directly, not ModuleNode
 */
export class ModuleNode {
    readonly builder: GraphBuilder;
    readonly id: string;
    readonly moduleType: string;
    readonly schema: ProcessedModuleSchema;
    private _channelCount: number = 1;

    constructor(
        builder: GraphBuilder,
        id: string,
        moduleType: string,
        schema: ProcessedModuleSchema,
    ) {
        this.builder = builder;
        this.id = id;
        this.moduleType = moduleType;
        this.schema = schema;
    }

    /**
     * Get the number of channels this module produces.
     * Set by Rust-side derivation via _setDerivedChannelCount.
     */
    get channelCount(): number {
        return this._channelCount;
    }

    _setParam(paramName: string, value: unknown): this {
        this.builder.setParam(this.id, paramName, replaceSignals(value));
        return this;
    }

    /**
     * Get a snapshot of the current params for this module.
     * Used for Rust-side channel count derivation.
     */
    getParamsSnapshot(): Record<string, unknown> {
        return this.builder.getModule(this.id)?.params ?? {};
    }

    /**
     * Set the channel count derived from Rust-side analysis.
     */
    _setDerivedChannelCount(channels: number): void {
        this._channelCount = channels;
    }

    /**
     * Get an output port of this module
     */
    _output(
        portName: string,
        polyphonic: boolean = false,
    ):
        | ModuleOutput
        | Collection
        | ModuleOutputWithRange
        | CollectionWithRange {
        // Verify output exists
        const outputSchema = this.schema.outputs.find(
            (o) => o.name === portName,
        ) as OutputSchemaWithRange | undefined;
        if (!outputSchema) {
            throw new Error(
                `Module ${this.moduleType} does not have output: ${portName}`,
            );
        }

        // Check if this output has range metadata
        const hasRange =
            outputSchema.minValue !== undefined &&
            outputSchema.maxValue !== undefined;

        if (polyphonic) {
            // Return Collection(WithRange) for each channel (based on derived channel count)
            if (hasRange) {
                const outputs: ModuleOutputWithRange[] = [];
                for (let i = 0; i < this.channelCount; i++) {
                    outputs.push(
                        new ModuleOutputWithRange(
                            this.builder,
                            this.id,
                            portName,
                            i,
                            outputSchema.minValue!,
                            outputSchema.maxValue!,
                        ),
                    );
                }
                return new CollectionWithRange(...outputs);
            } else {
                const outputs: ModuleOutput[] = [];
                for (let i = 0; i < this.channelCount; i++) {
                    outputs.push(
                        new ModuleOutput(this.builder, this.id, portName, i),
                    );
                }
                return new Collection(...outputs);
            }
        }

        if (hasRange) {
            return new ModuleOutputWithRange(
                this.builder,
                this.id,
                portName,
                0,
                outputSchema.minValue!,
                outputSchema.maxValue!,
            );
        }
        return new ModuleOutput(this.builder, this.id, portName);
    }
}

/**
 * ModuleOutput represents an output port that can be connected or transformed
 */
export class ModuleOutput {
    readonly builder: GraphBuilder;
    readonly moduleId: string;
    readonly portName: string;
    readonly channel: number = 0;

    constructor(
        builder: GraphBuilder,
        moduleId: string,
        portName: string,
        channel: number = 0,
    ) {
        this.builder = builder;
        this.moduleId = moduleId;
        this.portName = portName;
        this.channel = channel;
    }

    /**
     * Scale this output by a factor
     */
    gain(factor: Value): ModuleOutput {
        const factory = this.builder.getFactory('scaleAndShift');
        if (!factory) {
            throw new Error('Factory for scaleAndShift not registered');
        }
        return factory(this, factor) as ModuleOutput;
    }

    /**
     * Shift this output by an offset
     */
    shift(offset: Value): ModuleOutput {
        const factory = this.builder.getFactory('scaleAndShift');
        if (!factory) {
            throw new Error('Factory for scaleAndShift not registered');
        }
        return factory(this, undefined, offset) as ModuleOutput;
    }

    scope(msPerFrame: number = 500, triggerThreshold?: number): this {
        this.builder.addScope(this, msPerFrame, triggerThreshold);
        return this;
    }

    out(mode?: 'm'): this {
        if (mode !== 'm') {
            this.builder.addOut(this);
        }
        return this;
    }

    toString(): string {
        return `module(${this.moduleId}:${this.portName}:${this.channel})`;
    }
}

/**
 * ModuleOutputWithRange extends ModuleOutput with known output range metadata.
 * Provides .range() method to easily remap the output to a new range.
 */
export class ModuleOutputWithRange extends ModuleOutput {
    readonly minValue: number;
    readonly maxValue: number;

    constructor(
        builder: GraphBuilder,
        moduleId: string,
        portName: string,
        channel: number = 0,
        minValue: number,
        maxValue: number,
    ) {
        super(builder, moduleId, portName, channel);
        this.minValue = minValue;
        this.maxValue = maxValue;
    }

    /**
     * Remap this output from its known range to a new range.
     * Creates a remap module internally.
     */
    range(outMin: Value, outMax: Value): ModuleOutput {
        const factory = this.builder.getFactory('remap');
        if (!factory) {
            throw new Error('Factory for remap not registered');
        }
        return factory(this,
            this.minValue,
            this.maxValue,
            outMin,
            outMax,
        ) as ModuleOutput;
    }
}

type Replacer = (key: string, value: unknown) => unknown;

export function replaceValues(input: unknown, replacer: Replacer): unknown {
    function walk(key: string, value: unknown): unknown {
        const replaced = replacer(key, value);

        // Match JSON.stringify behavior
        if (replaced === undefined) {
            return undefined;
        }

        if (typeof replaced !== 'object' || replaced === null) {
            return replaced;
        }

        if (Array.isArray(replaced)) {
            return replaced
                .map((v, i) => walk(String(i), v))
                .filter((v) => v !== undefined);
        }

        const out: Record<string, unknown> = {};
        for (const [key, value] of Object.entries(replaced)) {
            const v = walk(key, value);
            if (v !== undefined) {
                out[key] = v;
            }
        }
        return out;
    }

    // JSON.stringify starts with key ""
    return walk('', input);
}

function replaceSignals(input: unknown): unknown {
    return replaceValues(input, (_key, value) => {
        // Replace Collection instances with their items array
        if (value instanceof Collection || value instanceof CollectionWithRange) {
            return [...value];
        }
        // Replace ModuleOutput instances with their JSON representation
        if (value instanceof ModuleOutput) {
            return valueToSignal(value);
        } else {
            return value;
        }
    });
}

type SignalValue = number | ModuleOutput;

function valueToSignal(value: SignalValue): unknown {
    if (value instanceof ModuleOutput) {
        return {
            type: 'cable',
            module: value.moduleId,
            port: value.portName,
            channel: value.channel,
        };
    }
    // It's a number
    return value;
}
