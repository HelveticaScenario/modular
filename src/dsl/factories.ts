import { ModuleSchema, getPatternPolyphony } from '@modular/core';
import { GraphBuilder, ModuleNode, ModuleOutput } from './GraphBuilder';

// LRU-style cache for pattern polyphony analysis to avoid re-parsing on every patch update
// Stores up to 100 patterns (sufficient for typical workflow)
const POLYPHONY_CACHE_MAX_SIZE = 100;
const patternPolyphonyCache = new Map<string, number>();

/**
 * Get cached polyphony or return undefined if not yet computed.
 * This is synchronous and non-blocking - returns immediately.
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

// Return type for module factories - varies by output configuration
type SingleOutput = ModuleOutput;
type PolyOutput = ModuleOutput[];
type MultiOutput = Record<string, ModuleOutput | ModuleOutput[]>;
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

        // Build namespace tree (only way to access modules)
        this.namespaceTree = buildNamespaceTree(schemas, this.factories);
    }

    /**
     * Create a module factory function that returns outputs directly
     */
    private createFactory(schema: ModuleSchema) {
        const isSeqModule = schema.name === 'seq';
        const outputs = schema.outputs || [];

        return (...args: any[]): ModuleReturn => {
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
            console.log(
                `Creating module: ${schema.name} with id: ${id} and params:`,
                params,
            );

            // Auto-set channels for seq module based on pattern polyphony analysis
            // Uses cached value if available, otherwise defaults to 4 and triggers async analysis
            if (
                isSeqModule &&
                params.channels === undefined &&
                params.pattern !== undefined
            ) {
                console.log('[seq] Analyzing pattern for polyphony:', params.pattern);
                const patternStr = String(params.pattern);
                const cached = getCachedPolyphony(patternStr);
                if (cached !== undefined) {
                    console.log(`[seq] Using cached polyphony: ${cached} for pattern "${patternStr.substring(0, 50)}..."`);
                    params.channels = cached;
                } else {
                    try {
                        const polyphony = getPatternPolyphony(patternStr);

                        // Evict oldest entry if at capacity
                        if (
                            patternPolyphonyCache.size >=
                            POLYPHONY_CACHE_MAX_SIZE
                        ) {
                            const oldestKey = patternPolyphonyCache
                                .keys()
                                .next().value;
                            if (oldestKey !== undefined) {
                                patternPolyphonyCache.delete(oldestKey);
                            }
                        }

                        patternPolyphonyCache.set(patternStr, polyphony);

                        params.channels = polyphony;
                        console.log(
                            `[seq] Auto-detected polyphony: ${polyphony} for pattern "${patternStr.substring(0, 50)}..."`,
                        );
                    } catch (e) {
                        // Fall back to default (4) on error
                        params.channels = 4;
                        console.warn(
                            `[seq] Failed to analyze pattern polyphony, using default 4:`,
                            e,
                        );
                    }
                }
            }

            // Create the module node internally
            const node = this.builder.addModule(schema.name, id);

            // Set all params
            for (const [key, value] of Object.entries(params)) {
                if (value !== undefined) {
                    node._setParam(key, value);
                }
            }

            // Return based on output configuration
            if (outputs.length === 0) {
                // No outputs - return empty object (shouldn't happen in practice)
                return {} as MultiOutput;
            } else if (outputs.length === 1) {
                // Single output - return ModuleOutput or ModuleOutput[]
                const output = outputs[0];
                return node._output(output.name, output.polyphonic ?? false);
            } else {
                // Multiple outputs - return object with camelCased property names
                const result: MultiOutput = {};
                for (const output of outputs) {
                    const camelName = toCamelCase(output.name);
                    result[camelName] = node._output(
                        output.name,
                        output.polyphonic ?? false,
                    );
                }
                return result;
            }
        };
    }

    /**
     * Get the builder instance
     */
    getBuilder(): GraphBuilder {
        return this.builder;
    }

    scope(
        target: ModuleOutput | ModuleOutput[],
        msPerFrame: number = 500,
        triggerThreshold?: number,
    ) {
        this.builder.addScope(target, msPerFrame, triggerThreshold);
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
