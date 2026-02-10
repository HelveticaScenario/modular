"use strict";
var __importDefault = (this && this.__importDefault) || function (mod) {
    return (mod && mod.__esModule) ? mod : { "default": mod };
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.DeferredCollection = exports.DeferredModuleOutput = exports.ModuleOutputWithRange = exports.ModuleOutput = exports.ModuleNode = exports.GraphBuilder = exports.$r = exports.$c = exports.CollectionWithRange = exports.Collection = exports.BaseCollection = exports.PORT_MAX_CHANNELS = void 0;
exports.replaceValues = replaceValues;
exports.replaceSignals = replaceSignals;
exports.replaceDeferredStrings = replaceDeferredStrings;
const paramsSchema_1 = require("./paramsSchema");
const factories_1 = require("./factories");
const zod_1 = __importDefault(require("zod"));
exports.PORT_MAX_CHANNELS = 16;
const ResolvedModuleOutput = zod_1.default.object({
    type: zod_1.default.literal('cable'),
    module: zod_1.default.string(),
    port: zod_1.default.string(),
    channel: zod_1.default.number().optional(),
});
/**
 * BaseCollection provides iterable, indexable container for ModuleOutput arrays
 * with chainable DSP methods (gain, shift, scope, out).
 */
class BaseCollection {
    items = [];
    constructor(...args) {
        this.items.push(...args);
        for (const [i, arg] of args.entries()) {
            this[i] = arg;
        }
    }
    get length() {
        return this.items.length;
    }
    [Symbol.iterator]() {
        return this.items.values();
    }
    /**
     * Scale all outputs by a factor
     */
    gain(factor) {
        if (this.items.length === 0)
            return new Collection();
        const factory = this.items[0].builder.getFactory('scaleAndShift');
        if (!factory) {
            throw new Error('Factory for util.scaleAndShift not registered');
        }
        return factory(this.items, factor);
    }
    /**
     * Shift all outputs by an offset
     */
    shift(offset) {
        if (this.items.length === 0)
            return new Collection();
        const factory = this.items[0].builder.getFactory('scaleAndShift');
        if (!factory) {
            throw new Error('Factory for util.scaleAndShift not registered');
        }
        return factory(this.items, undefined, offset);
    }
    /**
     * Add scope visualization for the first output in the collection
     */
    scope(config) {
        if (this.items.length > 0) {
            this.items[0].builder.addScope(this.items[0], config);
        }
        return this;
    }
    /**
     * Send all outputs to speakers as stereo
     * @param baseChannel - Base output channel (0-15, default 0)
     * @param options.gain - Output gain (adds scaleAndShift after stereo mix)
     * @param options.pan - Pan position (-5 = left, 0 = center, +5 = right)
     * @param options.width - Stereo width/spread (0 = no spread, 5 = full spread)
     */
    out(baseChannel = 0, options = {}) {
        if (this.items.length > 0) {
            this.items[0].builder.addOut([...this.items], {
                ...options,
                baseChannel,
            });
        }
        return this;
    }
    /**
     * Send all outputs to speakers as mono
     * @param channel - Output channel (0-15, default 0)
     * @param gain - Output gain
     */
    outMono(channel = 0, gain) {
        if (this.items.length > 0) {
            this.items[0].builder.addOutMono([...this.items], {
                channel,
                gain,
            });
        }
        return this;
    }
    toString() {
        return `[${this.items.map((item) => item.toString()).join(',')}]`;
    }
}
exports.BaseCollection = BaseCollection;
/**
 * Collection of ModuleOutput instances.
 * Use .range(inMin, inMax, outMin, outMax) to remap with explicit input range.
 */
class Collection extends BaseCollection {
    constructor(...args) {
        super(...args);
    }
    /**
     * Remap outputs from explicit input range to output range
     */
    range(inMin, inMax, outMin, outMax) {
        if (this.items.length === 0)
            return new Collection();
        const factory = this.items[0].builder.getFactory('remap');
        if (!factory) {
            throw new Error('Factory for util.remap not registered');
        }
        return factory(this.items, inMin, inMax, outMin, outMax);
    }
}
exports.Collection = Collection;
/**
 * Collection of ModuleOutputWithRange instances.
 * Use .range(outMin, outMax) to remap using stored min/max values.
 */
class CollectionWithRange extends BaseCollection {
    constructor(...args) {
        super(...args);
    }
    /**
     * Remap outputs from their known range to a new output range
     */
    range(outMin, outMax) {
        if (this.items.length === 0)
            return new Collection();
        const factory = this.items[0].builder.getFactory('remap');
        if (!factory) {
            throw new Error('Factory for util.remap not registered');
        }
        return factory(this.items, this.items.map((o) => o.minValue), this.items.map((o) => o.maxValue), outMin, outMax);
    }
}
exports.CollectionWithRange = CollectionWithRange;
/**
 * Create a Collection from ModuleOutput instances
 */
const $c = (...args) => new Collection(...args);
exports.$c = $c;
/**
 * Create a CollectionWithRange from ModuleOutputWithRange instances
 */
const $r = (...args) => new CollectionWithRange(...args);
exports.$r = $r;
/**
 * GraphBuilder manages the construction of a PatchGraph from DSL code.
 * It tracks modules, generates deterministic IDs, and builds the final graph.
 *
 * Note: Factory functions add overhead from channel count derivation but provide
 * consistency across all module creation paths.
 */
class GraphBuilder {
    modules = new Map();
    counters = new Map();
    schemas = [];
    schemaByName = new Map();
    scopes = [];
    /** Output groups keyed by baseChannel */
    outGroups = new Map();
    factoryRegistry = new Map();
    sourceLocationMap = new Map();
    /** Track all deferred outputs for string replacement during toPatch */
    deferredOutputs = new Map();
    /** Global tempo signal for ROOT_CLOCK (default: bpm(120) = hz(2)) */
    tempo = (0, factories_1.bpm)(120); // hz(2) = bpm(120), using constant to avoid circular dep
    /** Global output gain signal (default: 0.5) */
    outputGain = 0.5;
    constructor(schemas) {
        this.schemas = (0, paramsSchema_1.processSchemas)(schemas);
        this.schemaByName = new Map(this.schemas.map((s) => [s.name, s]));
    }
    /**
     * Generate a deterministic ID for a module type
     */
    generateId(moduleType, explicitId) {
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
    addModule(moduleType, explicitId, sourceLocation) {
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
        const params = {};
        for (const param of schema.params) {
            if (param.kind === 'signal' || param.kind === 'polySignal') {
                params[param.name] = { type: 'disconnected' };
            }
            else if (param.kind === 'signalArray') {
                // Required arrays (e.g. sum.signals) should be valid by default.
                params[param.name] = [];
            }
        }
        const moduleState = {
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
    getModule(id) {
        return this.modules.get(id);
    }
    /**
     * Set a parameter value for a module
     */
    setParam(moduleId, paramName, value) {
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
    setFactoryRegistry(factories) {
        this.factoryRegistry = factories;
    }
    /**
     * Set the global tempo for ROOT_CLOCK
     * @param tempo - Signal value for tempo (use bpm() or hz() helpers)
     */
    setTempo(tempo) {
        this.tempo = tempo;
    }
    /**
     * Set the global output gain
     * @param gain - Signal value for output gain (2.5 is default, 5.0 is unity)
     */
    setOutputGain(gain) {
        this.outputGain = gain;
    }
    /**
     * Get a factory function by module type name.
     * Returns undefined if factories haven't been registered yet.
     */
    getFactory(moduleType) {
        return this.factoryRegistry.get(moduleType);
    }
    /**
     * Build the final PatchGraph
     *
     * Note: Uses factory functions for signal/mix modules for consistency,
     * which adds overhead from channel count derivation on every patch build.
     */
    toPatch() {
        const signalFactory = this.getFactory('signal');
        const mixFactory = this.getFactory('mix');
        const stereoMixerFactory = this.getFactory('stereoMix');
        const scaleAndShiftFactory = this.getFactory('scaleAndShift');
        if (!signalFactory ||
            !mixFactory ||
            !stereoMixerFactory ||
            !scaleAndShiftFactory) {
            throw new Error('Required factories (signal, mix, stereoMixer, util.scaleAndShift) not registered');
        }
        // Process output groups and build channel collections
        if (this.outGroups.size > 0) {
            // Collect all channel collections to mix together
            const allChannelCollections = [];
            // Sort by baseChannel for deterministic processing
            const sortedChannels = [...this.outGroups.keys()].sort((a, b) => a - b);
            for (const baseChannel of sortedChannels) {
                const groups = this.outGroups.get(baseChannel);
                for (const group of groups) {
                    let outputSignals;
                    if (group.type === 'stereo') {
                        // Create stereoMixer with the outputs
                        const stereoOut = stereoMixerFactory(group.outputs, {
                            pan: group.pan ?? 0,
                            width: group.width ?? 0,
                        });
                        // Apply gain if specified
                        if (group.gain !== undefined) {
                            const gained = scaleAndShiftFactory([...stereoOut], group.gain);
                            outputSignals = [...gained];
                        }
                        else {
                            outputSignals = [...stereoOut];
                        }
                    }
                    else {
                        // Mono: use mix module
                        const mixOut = stereoMixerFactory(group.outputs, {
                            pan: -5,
                            width: 0,
                        })[0];
                        // Apply gain if specified
                        let finalOut;
                        if (group.gain !== undefined) {
                            finalOut = scaleAndShiftFactory(mixOut, group.gain);
                        }
                        else {
                            finalOut = mixOut;
                        }
                        outputSignals = [finalOut];
                    }
                    // Build channel collection with baseChannel silent channels prepended
                    const channelCollection = [];
                    // Add silent/disconnected channels for baseChannel offset
                    for (let i = 0; i < baseChannel; i++) {
                        // Create a signal module with disconnected input (outputs silence)
                        channelCollection.push(undefined);
                    }
                    // Add the actual output signals
                    channelCollection.push(...outputSignals);
                    allChannelCollections.push(channelCollection);
                }
            }
            // Mix all channel collections together using poly mix
            // Each collection contributes to corresponding output channels
            const finalMix = mixFactory(allChannelCollections);
            // Apply global output gain
            const gainedMix = finalMix.gain(this.outputGain);
            // Create root signal module with the final mix
            signalFactory(gainedMix, { id: 'ROOT_OUTPUT' });
        }
        else {
            // No outputs registered - create empty root signal
            signalFactory(undefined, { id: 'ROOT_OUTPUT' });
        }
        // Update ROOT_CLOCK tempo with the current tempo setting
        const rootClock = this.modules.get('ROOT_CLOCK');
        if (rootClock) {
            rootClock.params.tempo = this.tempo;
        }
        // Build a map of deferred output strings to their resolved output strings
        const deferredStringMap = new Map();
        for (const deferred of this.deferredOutputs.values()) {
            const deferredStr = deferred.toString();
            const resolved = deferred.resolve();
            if (resolved) {
                deferredStringMap.set(deferredStr, resolved.toString());
            }
            else {
                deferredStringMap.set(deferredStr, null);
            }
        }
        const ret = {
            modules: Array.from(this.modules.values()).map((m) => {
                console.log('Building module:', m);
                // First replace signals (ModuleOutput -> cable objects)
                const replacedParams = replaceDeferred(replaceSignals(m.params), this.deferredOutputs);
                // Then replace any deferred strings with resolved strings
                const finalParams = replaceDeferredStrings(replacedParams, deferredStringMap);
                return {
                    ...m,
                    params: finalParams,
                };
            }),
            scopes: this.scopes
                .map((scope) => {
                const deferredOutput = this.deferredOutputs.get(scope.item.moduleId);
                if (deferredOutput) {
                    const resolved = deferredOutput.resolve();
                    if (resolved) {
                        const newScope = {
                            ...scope,
                            item: {
                                type: 'ModuleOutput',
                                moduleId: resolved.moduleId,
                                portName: resolved.portName,
                            },
                        };
                        return newScope;
                    }
                    else {
                        return null;
                    }
                }
                return scope;
            })
                .filter((s) => s !== null),
        };
        console.log('Built PatchGraph:', ret);
        return ret;
    }
    /**
     * Reset the builder state
     */
    reset() {
        this.modules.clear();
        this.scopes = [];
        this.counters.clear();
        this.outGroups.clear();
        this.sourceLocationMap.clear();
        this.deferredOutputs.clear();
        this.tempo = 2; // hz(2) = bpm(120)
        this.outputGain = 2.5;
    }
    /**
     * Get the source location map for error reporting.
     * Maps module IDs to their source locations in the DSL code.
     */
    getSourceLocationMap() {
        return this.sourceLocationMap;
    }
    /**
     * Register module output(s) for stereo output routing
     */
    addOut(value, options = {}) {
        const baseChannel = options.baseChannel ?? 0;
        if (baseChannel < 0 || baseChannel > 14) {
            throw new Error(`baseChannel must be 0-14, got ${baseChannel}`);
        }
        const outputs = Array.isArray(value) ? [...value] : [value];
        const group = {
            type: 'stereo',
            outputs,
            gain: options.gain,
            pan: options.pan,
            width: options.width,
        };
        const existing = this.outGroups.get(baseChannel) ?? [];
        existing.push(group);
        this.outGroups.set(baseChannel, existing);
    }
    /**
     * Register module output(s) for mono output routing
     */
    addOutMono(value, options = {}) {
        const channel = options.channel ?? 0;
        if (channel < 0 || channel > 15) {
            throw new Error(`channel must be 0-15, got ${channel}`);
        }
        const outputs = Array.isArray(value) ? [...value] : [value];
        const group = {
            type: 'mono',
            outputs,
            gain: options.gain,
        };
        const existing = this.outGroups.get(channel) ?? [];
        existing.push(group);
        this.outGroups.set(channel, existing);
    }
    /**
     * Register a deferred output for tracking.
     * Called by DeferredModuleOutput constructor.
     */
    registerDeferred(deferred) {
        this.deferredOutputs.set(deferred.moduleId, deferred);
    }
    addScope(value, config = {}) {
        const { msPerFrame = 500, triggerThreshold, range = [-5, 5] } = config;
        let realTriggerThreshold = triggerThreshold !== undefined
            ? triggerThreshold * 1000
            : undefined;
        let output;
        if (Array.isArray(value)) {
            output = value[0];
        }
        else {
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
            range,
        });
    }
}
exports.GraphBuilder = GraphBuilder;
/**
 * ModuleNode represents a module instance in the DSL (internal use only)
 * Users interact with ModuleOutput directly, not ModuleNode
 */
class ModuleNode {
    builder;
    id;
    moduleType;
    schema;
    _channelCount = 1;
    constructor(builder, id, moduleType, schema) {
        this.builder = builder;
        this.id = id;
        this.moduleType = moduleType;
        this.schema = schema;
    }
    /**
     * Get the number of channels this module produces.
     * Set by Rust-side derivation via _setDerivedChannelCount.
     */
    get channelCount() {
        return this._channelCount;
    }
    _setParam(paramName, value) {
        this.builder.setParam(this.id, paramName, replaceSignals(value));
        return this;
    }
    /**
     * Get a snapshot of the current params for this module.
     * Used for Rust-side channel count derivation.
     */
    getParamsSnapshot() {
        return this.builder.getModule(this.id)?.params ?? {};
    }
    /**
     * Set the channel count derived from Rust-side analysis.
     */
    _setDerivedChannelCount(channels) {
        this._channelCount = channels;
    }
    /**
     * Get an output port of this module
     */
    _output(portName, polyphonic = false) {
        // Verify output exists
        const outputSchema = this.schema.outputs.find((o) => o.name === portName);
        if (!outputSchema) {
            throw new Error(`Module ${this.moduleType} does not have output: ${portName}`);
        }
        // Check if this output has range metadata
        const hasRange = outputSchema.minValue !== undefined &&
            outputSchema.maxValue !== undefined;
        if (polyphonic) {
            // Return Collection(WithRange) for each channel (based on derived channel count)
            if (hasRange) {
                const outputs = [];
                for (let i = 0; i < this.channelCount; i++) {
                    outputs.push(new ModuleOutputWithRange(this.builder, this.id, portName, i, outputSchema.minValue, outputSchema.maxValue));
                }
                return new CollectionWithRange(...outputs);
            }
            else {
                const outputs = [];
                for (let i = 0; i < this.channelCount; i++) {
                    outputs.push(new ModuleOutput(this.builder, this.id, portName, i));
                }
                return new Collection(...outputs);
            }
        }
        if (hasRange) {
            return new ModuleOutputWithRange(this.builder, this.id, portName, 0, outputSchema.minValue, outputSchema.maxValue);
        }
        return new ModuleOutput(this.builder, this.id, portName);
    }
}
exports.ModuleNode = ModuleNode;
/**
 * ModuleOutput represents an output port that can be connected or transformed
 */
class ModuleOutput {
    builder;
    moduleId;
    portName;
    channel = 0;
    constructor(builder, moduleId, portName, channel = 0) {
        this.builder = builder;
        this.moduleId = moduleId;
        this.portName = portName;
        this.channel = channel;
    }
    /**
     * Scale this output by a factor
     */
    gain(factor) {
        const factory = this.builder.getFactory('scaleAndShift');
        if (!factory) {
            throw new Error('Factory for util.scaleAndShift not registered');
        }
        return factory(this, factor);
    }
    /**
     * Shift this output by an offset
     */
    shift(offset) {
        const factory = this.builder.getFactory('scaleAndShift');
        if (!factory) {
            throw new Error('Factory for util.scaleAndShift not registered');
        }
        return factory(this, undefined, offset);
    }
    scope(config) {
        this.builder.addScope(this, config);
        return this;
    }
    /**
     * Send this output to speakers as stereo
     * @param baseChannel - Base output channel (0-15, default 0)
     * @param options.gain - Output gain (adds util.scaleAndShift after stereo mix)
     * @param options.pan - Pan position (-5 = left, 0 = center, +5 = right)
     * @param options.width - Stereo width/spread (0 = no spread, 5 = full spread)
     */
    out(baseChannel = 0, options = {}) {
        this.builder.addOut(this, { ...options, baseChannel });
        return this;
    }
    /**
     * Send this output to speakers as mono
     * @param channel - Output channel (0-15, default 0)
     * @param gain - Output gain
     */
    outMono(channel = 0, gain) {
        this.builder.addOutMono(this, { channel, gain });
        return this;
    }
    toString() {
        return `module(${this.moduleId}:${this.portName}:${this.channel})`;
    }
}
exports.ModuleOutput = ModuleOutput;
/**
 * ModuleOutputWithRange extends ModuleOutput with known output range metadata.
 * Provides .range() method to easily remap the output to a new range.
 */
class ModuleOutputWithRange extends ModuleOutput {
    minValue;
    maxValue;
    constructor(builder, moduleId, portName, channel = 0, minValue, maxValue) {
        super(builder, moduleId, portName, channel);
        this.minValue = minValue;
        this.maxValue = maxValue;
    }
    /**
     * Remap this output from its known range to a new range.
     * Creates a remap module internally.
     */
    range(outMin, outMax) {
        const factory = this.builder.getFactory('remap');
        if (!factory) {
            throw new Error('Factory for remap not registered');
        }
        return factory(this, this.minValue, this.maxValue, outMin, outMax);
    }
}
exports.ModuleOutputWithRange = ModuleOutputWithRange;
/**
 * DeferredModuleOutput is a placeholder for a signal that will be assigned later.
 * Useful for feedback loops and forward references in the DSL.
 * Supports the same chainable methods as ModuleOutput (gain, shift, scope, out, outMono).
 * Transforms are stored and applied when the deferred signal is resolved.
 */
class DeferredModuleOutput extends ModuleOutput {
    resolvedModuleOutput = null;
    resolving = false;
    static idCounter = 0;
    constructor(builder) {
        super(builder, `DEFERRED-${DeferredModuleOutput.idCounter++}`, 'output');
        // Register this deferred output with the builder for string replacement during toPatch
        builder.registerDeferred(this);
    }
    /**
     * Set the actual signal this deferred output should resolve to.
     * @param signal - The signal to resolve to (number, string, or ModuleOutput)
     */
    set(signal) {
        this.resolvedModuleOutput = signal;
    }
    /**
     * Resolve this deferred output to an actual ModuleOutput.
     * @returns The resolved ModuleOutput, or null if not set.
     */
    resolve() {
        if (this.resolving) {
            throw new Error('Circular reference detected while resolving DeferredModuleOutput');
        }
        if (this.resolvedModuleOutput === null) {
            return null;
        }
        let output = this.resolvedModuleOutput;
        if (output instanceof DeferredModuleOutput) {
            this.resolving = true;
            let resolved = output.resolve();
            this.resolving = false;
            if (resolved === null) {
                return null;
            }
            output = resolved;
        }
        return output;
    }
}
exports.DeferredModuleOutput = DeferredModuleOutput;
/**
 * DeferredCollection is a collection of DeferredModuleOutput instances.
 * Provides a .set() method to assign ModuleOutputs to all contained deferred outputs.
 */
class DeferredCollection extends BaseCollection {
    constructor(...args) {
        super(...args);
    }
    /**
     * Set the values for all deferred outputs in this collection.
     * @param outputs - A ModuleOutput or iterable of ModuleOutputs to distribute across outputs
     */
    set(outputs) {
        if (outputs instanceof ModuleOutput) {
            outputs = [outputs];
        }
        const outputsArr = Array.from(outputs);
        // Distribute signals across deferred outputs
        for (let i = 0; i < this.items.length; i++) {
            this.items[i].set(outputsArr[i % outputsArr.length]);
        }
    }
}
exports.DeferredCollection = DeferredCollection;
function replaceValues(input, replacer) {
    function walk(key, value) {
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
        const out = {};
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
function replaceSignals(input) {
    return replaceValues(input, (_key, value) => {
        // Replace Collection instances with their items array
        if (value instanceof BaseCollection) {
            return [...value];
        }
        return valueToSignal(value);
    });
}
/**
 * Recursively replace deferred output strings with resolved output strings in params.
 * This handles cases where a DeferredModuleOutput was stringified (e.g., in pattern strings).
 */
function replaceDeferredStrings(input, deferredStringMap) {
    if (typeof input === 'string') {
        // Replace all occurrences of deferred strings with resolved strings
        let result = input;
        for (const [deferredStr, resolvedStr] of deferredStringMap) {
            const splitResult = result.split(deferredStr);
            if (splitResult.length > 1) {
                if (resolvedStr === null) {
                    throw new Error(`Unset DeferredModuleOutput used in string: "${input}"`);
                }
                result = splitResult.join(resolvedStr);
            }
        }
        return result;
    }
    if (Array.isArray(input)) {
        return input.map((item) => replaceDeferredStrings(item, deferredStringMap));
    }
    if (typeof input === 'object' && input !== null) {
        const result = {};
        for (const [key, value] of Object.entries(input)) {
            result[key] = replaceDeferredStrings(value, deferredStringMap);
        }
        return result;
    }
    return input;
}
function replaceDeferred(input, deferredOutputs) {
    function replace(value) {
        const maybeResolvedModuleOutput = ResolvedModuleOutput.safeParse(value);
        if (maybeResolvedModuleOutput.success) {
            const resolved = deferredOutputs.get(maybeResolvedModuleOutput.data.module);
            if (resolved) {
                return valueToSignal(resolved.resolve());
            }
            else {
                return maybeResolvedModuleOutput.data;
            }
        }
        return value;
    }
    return replaceValues(input, (_key, value) => {
        // Replace Collection instances with their items array
        if (value instanceof BaseCollection) {
            return [...value];
        }
        return replace(value);
    });
}
function valueToSignal(value) {
    if (value instanceof ModuleOutput) {
        return {
            type: 'cable',
            module: value.moduleId,
            port: value.portName,
            channel: value.channel,
        };
    }
    else if (value === null || value === undefined) {
        return { type: 'disconnected' };
    }
    // It's a number
    return value;
}
//# sourceMappingURL=GraphBuilder.js.map