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

// Extend Array prototype for ModuleOutput arrays
declare global {
    interface Array<T> {
        gain(this: T extends ModuleOutput ? T[] : never, factor: Value): ModuleOutput[];
        offset(this: T extends ModuleOutput ? T[] : never, offset: Value): ModuleOutput[];
        out(this: T extends ModuleOutput ? T[] : never, mode?: 'm'): T[];
        scope(this: T extends ModuleOutput ? T[] : never, msPerFrame?: number, triggerThreshold?: number): T[];
        range(this: T extends ModuleOutputWithRange ? T[] : never, outMin: Value, outMax: Value): ModuleOutput[];
    }
}


/**
 * GraphBuilder manages the construction of a PatchGraph from DSL code.
 * It tracks modules, generates deterministic IDs, and builds the final graph.
 */
export class GraphBuilder {
    private modules: Map<string, ModuleState> = new Map();
    private counters: Map<string, number> = new Map();
    private schemas: ProcessedModuleSchema[] = [];
    private schemaByName: Map<string, ProcessedModuleSchema> = new Map();
    private scopes: Scope[] = [];
    private outModules: ModuleOutput[] = [];

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
    addModule(moduleType: string, explicitId?: string): ModuleNode {
        const id = this.generateId(moduleType, explicitId);

        if (this.modules.has(id)) {
            throw new Error(`Duplicate module id: ${id}`);
        }

        // Check if module type exists in schemas
        const schema = this.schemaByName.get(moduleType);
        if (!schema) {
            throw new Error(`Unknown module type: ${moduleType}`);
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
     * Build the final PatchGraph
     */
    toPatch(): PatchGraph {
        // Create the root signal module that will receive the final output
        const rootSignal = this.addModule('signal', 'root');

        // If there are any modules registered with out(), create a sum module
        if (this.outModules.length > 0) {
            const sumModule = this.addModule('sum');

            // Set the signals parameter on the sum module
            this.setParam(sumModule.id, 'signals', this.outModules);

            // Connect the sum output to the root signal's source
            const sumOutput = sumModule._output('output', false) as ModuleOutput;
            rootSignal._setParam('source', sumOutput);
        }
        // console.log('modules', new Map(this.modules.entries()));
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
        let realTriggerThreshold: number | undefined = triggerThreshold !== undefined
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
            },
            msPerFrame,
            triggerThreshold: realTriggerThreshold,
        });
    }
}

type Value = number | ModuleOutput | ModuleOutput[] | ModuleNode | ModuleNode[];

/**
 * Extract channel count from a signal value.
 * - scalar (number, single ModuleOutput, single ModuleNode) → 1
 * - array → array.length
 * - ModuleNode with poly output → node's channelCount
 * - ModuleOutput[] → array.length
 */
function getChannelCount(value: unknown): number {
    if (Array.isArray(value)) {
        return value.length;
    }
    if (value instanceof ModuleNode) {
        // If the node's default output is polyphonic, use its channel count
        return value.channelCount;
    }
    if (value instanceof ModuleOutput) {
        return 1;
    }
    // Scalar value (number, string, etc.)
    return 1;
}

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
    private readonly polySignalParamNames: Set<string>;

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
        // Build set of polySignal param names for O(1) lookup
        this.polySignalParamNames = new Set(
            schema.params
                .filter((p) => p.kind === 'polySignal')
                .map((p) => p.name),
        );
    }

    /**
     * Get the number of channels this module produces.
     * Priority:
     * 1. Hardcoded channels from module schema (if present)
     * 2. Channels param value if set, otherwise channels param default
     * 3. Max channel count of all polySignal inputs (inferred)
     */
    get channelCount(): number {
        // Check for hardcoded channel count in module schema
        if (this.schema.channels != null) {
            return this.schema.channels;
        }
        // Check for channels param in module schema
        if (this.schema.channelsParam != null) {
            const moduleState = this.builder.getModule(this.id);
            const paramValue = moduleState?.params[this.schema.channelsParam];
            if (typeof paramValue === 'number') {
                return paramValue;
            }
            // Fall back to channels param default
            if (this.schema.channelsParamDefault != null) {
                return this.schema.channelsParamDefault;
            }
        }
        // Fall back to inferred channel count from inputs
        return this._channelCount;
    }

    _setParam(paramName: string, value: unknown): this {
        this.builder.setParam(this.id, paramName, replaceSignals(value));
        // Track channel count for polySignal params
        if (this.polySignalParamNames.has(paramName)) {
            const inputChannels = getChannelCount(value);
            if (inputChannels > this._channelCount) {
                this._channelCount = inputChannels;
            }
        }

        return this;
    }

    /**
     * Get an output port of this module
     */
    _output(portName: string, polyphonic: boolean = false): ModuleOutput | ModuleOutput[] | ModuleOutputWithRange | ModuleOutputWithRange[] {
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
        const hasRange = outputSchema.minValue !== undefined && outputSchema.maxValue !== undefined;
        
        if (polyphonic) {
            // Return array of ModuleOutput(WithRange) for each channel (based on derived channel count)
            const outputs: (ModuleOutput | ModuleOutputWithRange)[] = [];
            for (let i = 0; i < this.channelCount; i++) {
                if (hasRange) {
                    outputs.push(new ModuleOutputWithRange(
                        this.builder, this.id, portName, i,
                        outputSchema.minValue!, outputSchema.maxValue!
                    ));
                } else {
                    outputs.push(new ModuleOutput(this.builder, this.id, portName, i));
                }
            }
            return outputs;
        }
        
        if (hasRange) {
            return new ModuleOutputWithRange(
                this.builder, this.id, portName, 0,
                outputSchema.minValue!, outputSchema.maxValue!
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

    constructor(builder: GraphBuilder, moduleId: string, portName: string, channel: number = 0) {
        this.builder = builder;
        this.moduleId = moduleId;
        this.portName = portName;
        this.channel = channel;
    }

    /**
     * Scale this output by a factor
     */
    gain(factor: Value): ModuleOutput {
        const scaleNode = this.builder.addModule('scaleAndShift');
        scaleNode._setParam('input', this);
        scaleNode._setParam('scale', factor);
        return scaleNode._output('output', false) as ModuleOutput;
    }

    /**
     * Shift this output by an offset
     */
    shift(offset: Value): ModuleOutput {
        const shiftNode = this.builder.addModule('scaleAndShift');
        shiftNode._setParam('input', this);
        shiftNode._setParam('shift', offset);
        return shiftNode._output('output', false) as ModuleOutput;
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
        maxValue: number
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
        const remapNode = this.builder.addModule('remap');
        remapNode._setParam('input', this);
        remapNode._setParam('inMin', this.minValue);
        remapNode._setParam('inMax', this.maxValue);
        remapNode._setParam('outMin', outMin);
        remapNode._setParam('outMax', outMax);
        return remapNode._output('output', false) as ModuleOutput;
    }
}

// Add Array prototype methods for ModuleOutput arrays
// These need to be added after ModuleOutput is defined
Array.prototype.gain = function (this: ModuleOutput[], factor: Value): ModuleOutput[] {
    const scaleNode = this[0].builder.addModule('scaleAndShift');
    scaleNode._setParam('input', this);
    scaleNode._setParam('scale', factor);
    return scaleNode._output('output', true) as ModuleOutput[];
};

Array.prototype.offset = function (this: ModuleOutput[], offset: Value): ModuleOutput[] {
    const shiftNode = this[0].builder.addModule('scaleAndShift');
    shiftNode._setParam('input', this);
    shiftNode._setParam('shift', offset);
    return shiftNode._output('output', true) as ModuleOutput[];
};

Array.prototype.out = function (this: ModuleOutput[], mode?: 'm'): ModuleOutput[] {
    for (const output of this) {
        output.out(mode);
    }
    return this;
};

// Add Array prototype method for range remapping on ModuleOutputWithRange arrays
Array.prototype.range = function (this: ModuleOutputWithRange[], outMin: Value, outMax: Value): ModuleOutput[] {
    if (this.length === 0) {
        return [];
    }
    
    // Create a single remap module with poly inputs
    const remapNode = this[0].builder.addModule('remap');
    
    // Pass the array of outputs to input
    remapNode._setParam('input', this);
    
    // Collect the min/max values from each output's known range
    const inMins = this.map(o => o.minValue);
    const inMaxs = this.map(o => o.maxValue);
    
    remapNode._setParam('inMin', inMins);
    remapNode._setParam('inMax', inMaxs);
    remapNode._setParam('outMin', outMin);
    remapNode._setParam('outMax', outMax);
    
    return remapNode._output('output', true) as ModuleOutput[];
};

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
