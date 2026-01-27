import type {
    ModuleSchema,
    ModuleState,
    PatchGraph,
    Scope,
} from '@modular/core';
import type { ProcessedModuleSchema } from './paramsSchema';
import { processSchemas } from './paramsSchema';

const PORT_MAX_CHANNELS = 16;

// Extend Array prototype for ModuleOutput arrays
declare global {
    interface Array<T> {
        gain(this: ModuleOutput[], factor: Value): ModuleNode;
        offset(this: ModuleOutput[], offset: Value): ModuleNode;
        out(this: ModuleOutput[], mode?: 'm'): ModuleOutput[];
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
            rootSignal._setParam('source', sumModule);
        }
        console.log('modules', new Map(this.modules.entries()));
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
     * Register a module or output to be sent to speakers
     */
    addOut(value: ModuleOutput | ModuleOutput[] | ModuleNode): void {
        let output: ModuleOutput | ModuleOutput[];
        if (value instanceof ModuleNode) {
            output = value.o;
        } else {
            output = value;
        }

        if (Array.isArray(output)) {
            this.outModules.push(...output);
        } else {
            this.outModules.push(output);
        }
    }

    addScope(
        value: ModuleOutput | ModuleOutput[] | ModuleNode,
        msPerFrame: number = 500,
        triggerThreshold?: number,
    ) {
        let realTriggerThreshold: number | undefined = triggerThreshold !== undefined
            ? triggerThreshold * 1000
            : undefined;
        let output: ModuleOutput;
        if (value instanceof ModuleNode) {
            const o = value.o;
            output = Array.isArray(o) ? o[0] : o;
        } else if (Array.isArray(value)) {
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
 * ModuleNode represents a module instance in the DSL with fluent API
 */
export class ModuleNode {
    // Dynamic parameter methods will be added via Proxy
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    [key: string]: any;

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
        // Create a proxy to intercept parameter method calls
        const proxy = new Proxy(this, {
            get(target, prop: string) {
                // Check if it's a param name (derived from schemars JSON schema)
                const param = target.schema.paramsByName[prop];
                if (param && Object.hasOwn(target.schema.paramsByName, prop)) {
                    return (...args: unknown[]) => {
                        if (args.length > 1) {
                            target._setParam(prop, args);
                        } else {
                            target._setParam(prop, args[0]);
                        }
                        return proxy;
                    };
                }

                // Check if it's an output name
                const outputSchema =
                    target.schema.outputs.find((o) => o.name === prop) ??
                    (prop === 'output'
                        ? (target.schema.outputs.find((o) => o.default) ??
                            target.schema.outputs[0])
                        : undefined);
                if (outputSchema) {
                    return target._output(outputSchema.name, outputSchema.polyphonic ?? false);
                }

                if (prop in target) {
                    // eslint-disable-next-line @typescript-eslint/no-explicit-any
                    const thing = (target as any)[prop];
                    if (typeof thing === 'function') {
                        return thing.bind(proxy);
                    }
                    return thing;
                }

                return undefined;
            },
        });

        return proxy as unknown as ModuleNode;
    }

    get o(): ModuleOutput | ModuleOutput[] {
        const defaultOutput = this.schema.outputs.find((o) => o.default);
        if (!defaultOutput) {
            throw new Error(`Module ${this.moduleType} has no default output`);
        }
        return this._output(defaultOutput.name, defaultOutput.polyphonic ?? false);
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

    gain(factor: Value | Value[]): ModuleNode {
        const scaleNode = this.builder.addModule('scaleAndShift');
        scaleNode._setParam('input', this);
        scaleNode._setParam('scale', factor);
        return scaleNode;
    }

    shift(factor: Value | Value[]): ModuleNode {
        const scaleNode = this.builder.addModule('scaleAndShift');
        scaleNode._setParam('input', this);
        scaleNode._setParam('shift', factor);
        return scaleNode;
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
    _output(portName: string, polyphonic: boolean = false): ModuleOutput | ModuleOutput[] {
        // Verify output exists
        const outputSchema = this.schema.outputs.find(
            (o) => o.name === portName,
        );
        if (!outputSchema) {
            throw new Error(
                `Module ${this.moduleType} does not have output: ${portName}`,
            );
        }
        if (polyphonic) {
            // Return array of ModuleOutput for each channel (based on derived channel count)
            const outputs: ModuleOutput[] = [];
            for (let i = 0; i < this.channelCount; i++) {
                outputs.push(new ModuleOutput(this.builder, this.id, portName, i));
            }
            return outputs;
        }
        return new ModuleOutput(this.builder, this.id, portName);
    }

    toString(): string {
        return this.o.toString();
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
    gain(factor: Value): ModuleNode {
        const scaleNode = this.builder.addModule('scaleAndShift');
        scaleNode._setParam('input', this);
        scaleNode._setParam('scale', factor);
        return scaleNode;
    }

    /**
     * Shift this output by an offset
     */
    shift(offset: Value): ModuleNode {
        const shiftNode = this.builder.addModule('scaleAndShift');
        shiftNode._setParam('input', this);
        shiftNode._setParam('shift', offset);
        return shiftNode;
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

// Add Array prototype methods for ModuleOutput arrays
// These need to be added after ModuleOutput is defined
Array.prototype.gain = function (this: ModuleOutput[], factor: Value): ModuleNode {
    const scaleNode = this[0].builder.addModule('scaleAndShift');
    scaleNode._setParam('input', this);
    scaleNode._setParam('scale', factor);
    return scaleNode;
};

Array.prototype.offset = function (this: ModuleOutput[], offset: Value): ModuleNode {
    const shiftNode = this[0].builder.addModule('scaleAndShift');
    shiftNode._setParam('input', this);
    shiftNode._setParam('shift', offset);
    return shiftNode;
};

Array.prototype.out = function (this: ModuleOutput[], mode?: 'm'): ModuleOutput[] {
    for (const output of this) {
        output.out(mode);
    }
    return this;
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
        // Replace Signal instances with their JSON representation
        if (value instanceof ModuleNode || value instanceof ModuleOutput) {
            return valueToSignal(value);
        } else {
            return value;
        }
    });
}

type SignalValue = number | ModuleOutput | ModuleNode;

function valueToSignal(value: SignalValue): unknown {
    if (value instanceof ModuleNode) {
        const out = value.o;
        if (Array.isArray(out)) {
            // For poly outputs, create an array of cables (one per channel)
            return out.map((o) => ({
                type: 'cable',
                module: o.moduleId,
                port: o.portName,
                channel: o.channel,
            }));
        }
        return {
            type: 'cable',
            module: out.moduleId,
            port: out.portName,
            channel: 0,
        };
    }
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
