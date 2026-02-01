import { ModuleSchema, deriveChannelCount } from '@modular/core';
import { GraphBuilder, ModuleNode, ModuleOutput, Collection, CollectionWithRange } from './GraphBuilder';

// LRU-style cache for seq pattern polyphony analysis to avoid re-parsing on every patch update
// The Rust-side analysis runs 300 cycles which can be expensive for complex patterns
const POLYPHONY_CACHE_MAX_SIZE = 100;
const patternPolyphonyCache = new Map<string, number>();

/**
 * Get cached polyphony for a pattern string, or undefined if not cached.
 * Uses LRU eviction - moves accessed entries to end of map.
 */
function getCachedPolyphony(patternStr: string): number | undefined {
    const cached = patternPolyphonyCache.get(patternStr);
    if (cached !== undefined) {
        // Move to end (most recently used) by re-inserting
        patternPolyphonyCache.delete(patternStr);
        patternPolyphonyCache.set(patternStr, cached);
        return cached;
    }
    return undefined;
}

/**
 * Cache a polyphony result for a pattern string with LRU eviction.
 */
function cachePolyphony(patternStr: string, polyphony: number): void {
    // Evict oldest entry if at capacity
    if (patternPolyphonyCache.size >= POLYPHONY_CACHE_MAX_SIZE) {
        const oldestKey = patternPolyphonyCache.keys().next().value;
        if (oldestKey !== undefined) {
            patternPolyphonyCache.delete(oldestKey);
        }
    }
    patternPolyphonyCache.set(patternStr, polyphony);
}

/**
 * Line offset for DSL code wrapper.
 * The executePatchScript creates a function body with 'use strict' which adds lines
 * before user code. This offset is set by executor.ts at runtime.
 */
export let DSL_WRAPPER_LINE_OFFSET = 4;

/**
 * Configure the line offset for DSL code wrapper.
 */
export function setDSLWrapperLineOffset(offset: number): void {
    DSL_WRAPPER_LINE_OFFSET = offset;
}

/**
 * Capture source location from the current stack trace.
 * Looks for the "<anonymous>" frame which corresponds to DSL code execution.
 * Returns undefined if source location cannot be determined.
 */
function captureSourceLocation(): { line: number; column: number } | undefined {
    const stackHolder: { stack?: string } = {};
    Error.captureStackTrace(stackHolder, captureSourceLocation);

    if (!stackHolder.stack) {
        return undefined;
    }

    // Parse stack trace to find the DSL code frame
    // Stack frames from evaluated code look like:
    // "    at eval (eval at executePatchScript ..., <anonymous>:5:12)"
    // or in some V8 versions:
    // "    at <anonymous>:5:12"
    const lines = stackHolder.stack.split('\n');

    for (const line of lines) {
        // Look for <anonymous>:line:col pattern
        const anonymousMatch = line.match(/<anonymous>:(\d+):(\d+)/);
        if (anonymousMatch) {
            const rawLine = parseInt(anonymousMatch[1], 10);
            const column = parseInt(anonymousMatch[2], 10);
            // Adjust for wrapper code offset
            const adjustedLine = rawLine - DSL_WRAPPER_LINE_OFFSET;
            if (adjustedLine > 0) {
                return { line: adjustedLine, column };
            }
        }
    }

    return undefined;
}

// Return type for module factories - varies by output configuration
type SingleOutput = ModuleOutput;
type PolyOutput = Collection | CollectionWithRange;
type MultiOutput = (SingleOutput | PolyOutput) & Record<string, ModuleOutput | Collection | CollectionWithRange>;
type ModuleReturn = SingleOutput | PolyOutput | MultiOutput;

type FactoryFunction = (...args: any[]) => ModuleReturn;

type NamespaceTree = {
    [key: string]: NamespaceTree | FactoryFunction;
};

function sanitizeIdentifier(name: string): string {
    let id = name.replace(
        /[^a-zA-Z0-9_$]+(.)?/g,
        (_match, chr: string | undefined) => (chr ? chr.toUpperCase() : ''),
    );
    if (!/^[A-Za-z_$]/.test(id)) {
        id = `_${id}`;
    }
    return id || '_';
}

/**
 * Convert snake_case to camelCase
 */
function toCamelCase(str: string): string {
    return str.replace(/_([a-z])/g, (_, letter) => letter.toUpperCase());
}

/**
 * Reserved property names that conflict with ModuleOutput, Collection, or CollectionWithRange methods/properties.
 * Output names matching these will be suffixed with an underscore.
 *
 * IMPORTANT: When adding new methods to any type that a factory function could return
 * (ModuleOutput, ModuleOutputWithRange, BaseCollection, Collection, CollectionWithRange),
 * the method name MUST be added to this list. Keep in sync with:
 * - crates/modular_derive/src/lib.rs (RESERVED_OUTPUT_NAMES)
 * - src/dsl/typescriptLibGen.ts (RESERVED_OUTPUT_NAMES)
 */
const RESERVED_OUTPUT_NAMES = new Set([
    // ModuleOutput properties
    'builder',
    'moduleId',
    'portName',
    'channel',
    // ModuleOutput methods
    'gain',
    'shift',
    'scope',
    'out',
    'outMono',
    'toString',
    // ModuleOutputWithRange properties
    'minValue',
    'maxValue',
    'range',
    // Collection/CollectionWithRange properties
    'items',
    'length',
    // JavaScript built-ins
    'constructor',
    'prototype',
    '__proto__',
]);

/**
 * Sanitize output name to avoid conflicts with reserved properties/methods.
 * Appends underscore if the camelCase name is reserved.
 */
function sanitizeOutputName(name: string): string {
    const camelName = toCamelCase(name);
    return RESERVED_OUTPUT_NAMES.has(camelName) ? `${camelName}_` : camelName;
}

/**
 * Build a nested namespace tree from module schemas
 * Mirrors the logic in typescriptLibGen.ts buildTreeFromSchemas()
 */
function buildNamespaceTree(
    schemas: ModuleSchema[],
    factoryMap: Record<string, FactoryFunction>,
): NamespaceTree {
    const tree: NamespaceTree = {};

    for (const schema of schemas) {
        const fullName = schema.name.trim();
        const parts = fullName.split('.').filter((p) => p.length > 0);

        const factoryName = sanitizeIdentifier(fullName);
        const factory = factoryMap[factoryName];

        if (parts.length === 1) {
            // No namespace, add to root
            tree[parts[0]] = factory;
        } else {
            // Navigate/create namespace hierarchy
            const className = parts[parts.length - 1];
            const namespacePath = parts.slice(0, -1);

            let current: NamespaceTree = tree;
            for (const ns of namespacePath) {
                if (!current[ns]) {
                    current[ns] = {};
                } else if (typeof current[ns] === 'function') {
                    throw new Error(
                        `Namespace collision: ${ns} is both a module and a namespace`,
                    );
                }
                current = current[ns] as NamespaceTree;
            }

            if (
                current[className] &&
                typeof current[className] !== 'function'
            ) {
                throw new Error(
                    `Module name collision: ${className} already exists as a namespace`,
                );
            }
            current[className] = factory;
        }
    }

    return tree;
}

/**
 * DSL Context holds the builder and provides factory functions
 */
export class DSLContext {
    factories: Record<string, FactoryFunction> = {};
    namespaceTree: NamespaceTree = {};
    private builder: GraphBuilder;

    constructor(schemas: ModuleSchema[]) {
        this.builder = new GraphBuilder(schemas);

        // Build flat factory map (internal use for tree building)
        for (const schema of schemas) {
            const factoryName = sanitizeIdentifier(schema.name);
            this.factories[factoryName] = this.createFactory(schema);
        }

        // Register factories with the builder for internal use (late binding)
        // This allows GraphBuilder methods like .gain(), .shift(), .range() to use factories
        // Note: This adds overhead from channel count derivation but ensures consistency
        const factoryMap = new Map<string, FactoryFunction>();
        for (const schema of schemas) {
            factoryMap.set(schema.name, this.factories[sanitizeIdentifier(schema.name)]);
        }
        this.builder.setFactoryRegistry(factoryMap);

        // Build namespace tree (only way to access modules)
        this.namespaceTree = buildNamespaceTree(schemas, this.factories);
    }

    /**
     * Create a module factory function that returns outputs directly
     */
    private createFactory(schema: ModuleSchema) {
        const outputs = schema.outputs || [];

        return (...args: any[]): ModuleReturn => {
            // Capture source location from stack trace
            const sourceLocation = captureSourceLocation();

            // @ts-ignore
            const positionalArgs = schema.positionalArgs || [];
            const params: Record<string, any> = {};
            let config: any = {};
            let id: string | undefined;

            // Extract positional args
            for (let i = 0; i < positionalArgs.length; i++) {
                if (i < args.length) {
                    params[positionalArgs[i].name] = args[i];
                }
            }

            // The remaining arg (if any) is config.
            if (args.length > positionalArgs.length) {
                config = args[positionalArgs.length];
            }

            if (config) {
                if (typeof config === 'string') {
                    id = config;
                } else {
                    id = config.id;
                    // Merge other config params
                    for (const key in config) {
                        if (key !== 'id') {
                            params[key] = config[key];
                        }
                    }
                }
            }


            // Create the module node internally, passing source location
            const node = this.builder.addModule(schema.name, id, sourceLocation);

            // Set all params
            for (const [key, value] of Object.entries(params)) {
                if (value !== undefined) {
                    node._setParam(key, value);
                }
            }

            // Derive channel count from params using Rust-side derivation
            // This handles modules with custom derivation logic (like mix, seq)
            // as well as standard inference from PolySignal inputs
            let derivedChannels: number | null = null;

            // For seq module with a pattern, use LRU cache to avoid expensive re-analysis
            // (Rust-side pattern polyphony analysis runs 300 cycles)
            if (schema.name === 'seq' && params.pattern !== undefined) {
                if (params.channels !== undefined) {
                    // If channels explicitly set, use it directly
                    derivedChannels = params.channels as number;
                    node._setDerivedChannelCount(derivedChannels);
                } else {
                    const patternStr = String(params.pattern);
                    const cached = getCachedPolyphony(patternStr);
                    if (cached !== undefined) {
                        derivedChannels = cached;
                    } else {
                        derivedChannels = deriveChannelCount(schema.name, node.getParamsSnapshot());
                        if (derivedChannels !== null) {
                            cachePolyphony(patternStr, derivedChannels);
                        }
                    }
                    params.channels = derivedChannels
                    node._setParam('channels', derivedChannels);
                }
            } else {
                derivedChannels = deriveChannelCount(schema.name, node.getParamsSnapshot());
            }

            if (derivedChannels !== null) {
                node._setDerivedChannelCount(derivedChannels);
            }

            // Return based on output configuration
            if (outputs.length === 0) {
                // No outputs - return empty object (shouldn't happen in practice)
                return {} as MultiOutput;
            } else if (outputs.length === 1) {
                // Single output - return ModuleOutput, Collection, or CollectionWithRange
                const output = outputs[0];
                return node._output(output.name, output.polyphonic ?? false);
            } else {
                // Multiple outputs - create hybrid object extending the default output
                // Find the default output (or use first if none marked)
                const defaultOutput = outputs.find((o) => o.default) || outputs[0];
                const defaultValue = node._output(defaultOutput.name, defaultOutput.polyphonic ?? false);

                // Create the additional output properties
                const additionalOutputs: Record<string, ModuleOutput | Collection | CollectionWithRange> = {};
                for (const output of outputs) {
                    if (output.name === defaultOutput.name) continue;
                    const safeName = sanitizeOutputName(output.name);
                    additionalOutputs[safeName] = node._output(
                        output.name,
                        output.polyphonic ?? false,
                    );
                }

                // Return hybrid object: default output with additional properties
                return Object.assign(defaultValue, additionalOutputs);
            }
        };
    }

    /**
     * Get the builder instance
     */
    getBuilder(): GraphBuilder {
        return this.builder;
    }

    scope<T extends ModuleOutput | Collection | CollectionWithRange>(
        target: T,
        config?: { msPerFrame?: number; triggerThreshold?: number; scale?: number },
    ): T {
        if (target instanceof Collection || target instanceof CollectionWithRange) {
            this.builder.addScope([...target], config);
        } else {
            this.builder.addScope(target, config);
        }
        return target;
    }
}

/**
 * Helper function to convert Hz to V/oct
 * V/oct = log2(Hz / C4) where C4 = 261.6255653005986 Hz
 * Convention: 0V = C4 = MIDI 60 = ~261.626 Hz
 */
const C4_HZ = 261.6255653005986; // 440 / 2^(9/12)

export function hz(frequency: number): number {
    if (frequency <= 0) {
        throw new Error('Frequency must be positive');
    }
    return Math.log2(frequency / C4_HZ);
}

/**
 * Note name to V/oct conversion
 * Supports notes like "c4", "c#4", "db4", etc.
 */
export function note(noteName: string): number {
    const noteRegex = /^([a-g])([#b]?)(-?\d+)?$/i;
    const match = noteName.toLowerCase().match(noteRegex);

    if (!match) {
        throw new Error(`Invalid note name: ${noteName}`);
    }

    const [, noteLetter, accidental, octaveStr] = match;
    const octave = octaveStr ? parseInt(octaveStr, 10) : 3;

    // Map note letters to semitones (C = 0)
    const noteMap: Record<string, number> = {
        c: 0,
        d: 2,
        e: 4,
        f: 5,
        g: 7,
        a: 9,
        b: 11,
    };

    let semitone = noteMap[noteLetter];

    // Apply accidentals
    if (accidental === '#') {
        semitone += 1;
    } else if (accidental === 'b') {
        semitone -= 1;
    }

    // Calculate frequency: C4 = 261.6255653005986 Hz (middle C)
    const semitonesFromC4 = (octave - 4) * 12 + semitone;
    const frequency = C4_HZ * Math.pow(2, semitonesFromC4 / 12);

    return hz(frequency);
}

/**
 * Convert BPM (beats per minute) to V/oct frequency
 * BPM is tempo, where 1 beat = 1 quarter note
 * At 120 BPM, that's 2 beats per second = 2 Hz
 */
export function bpm(beatsPerMinute: number): number {
    if (beatsPerMinute <= 0) {
        throw new Error('BPM must be positive');
    }
    // Convert BPM to Hz: Hz = BPM / 60
    const frequency = beatsPerMinute / 60;
    return hz(frequency);
}
