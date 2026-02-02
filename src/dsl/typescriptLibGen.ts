import { ModuleSchema } from '@modular/core';

const BASE_LIB_SOURCE = `
/** The **\`console\`** object provides access to the debugging console (e.g., the Web console in Firefox). */
/**
 * The **\`console\`** object provides access to the debugging console (e.g., the Web console in Firefox).
 *
 * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console)
 */
interface Console {
    /**
     * The **\`console.assert()\`** static method writes an error message to the console if the assertion is false.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/assert_static)
     */
    assert(condition?: boolean, ...data: any[]): void;
    /**
     * The **\`console.clear()\`** static method clears the console if possible.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/clear_static)
     */
    clear(): void;
    /**
     * The **\`console.count()\`** static method logs the number of times that this particular call to \`count()\` has been called.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/count_static)
     */
    count(label?: string): void;
    /**
     * The **\`console.countReset()\`** static method resets counter used with console/count_static.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/countReset_static)
     */
    countReset(label?: string): void;
    /**
     * The **\`console.debug()\`** static method outputs a message to the console at the 'debug' log level.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/debug_static)
     */
    debug(...data: any[]): void;
    /**
     * The **\`console.dir()\`** static method displays a list of the properties of the specified JavaScript object.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/dir_static)
     */
    dir(item?: any, options?: any): void;
    /**
     * The **\`console.dirxml()\`** static method displays an interactive tree of the descendant elements of the specified XML/HTML element.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/dirxml_static)
     */
    dirxml(...data: any[]): void;
    /**
     * The **\`console.error()\`** static method outputs a message to the console at the 'error' log level.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/error_static)
     */
    error(...data: any[]): void;
    /**
     * The **\`console.group()\`** static method creates a new inline group in the Web console log, causing any subsequent console messages to be indented by an additional level, until console/groupEnd_static is called.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/group_static)
     */
    group(...data: any[]): void;
    /**
     * The **\`console.groupCollapsed()\`** static method creates a new inline group in the console.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/groupCollapsed_static)
     */
    groupCollapsed(...data: any[]): void;
    /**
     * The **\`console.groupEnd()\`** static method exits the current inline group in the console.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/groupEnd_static)
     */
    groupEnd(): void;
    /**
     * The **\`console.info()\`** static method outputs a message to the console at the 'info' log level.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/info_static)
     */
    info(...data: any[]): void;
    /**
     * The **\`console.log()\`** static method outputs a message to the console.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/log_static)
     */
    log(...data: any[]): void;
    /**
     * The **\`console.table()\`** static method displays tabular data as a table.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/table_static)
     */
    table(tabularData?: any, properties?: string[]): void;
    /**
     * The **\`console.time()\`** static method starts a timer you can use to track how long an operation takes.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/time_static)
     */
    time(label?: string): void;
    /**
     * The **\`console.timeEnd()\`** static method stops a timer that was previously started by calling console/time_static.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/timeEnd_static)
     */
    timeEnd(label?: string): void;
    /**
     * The **\`console.timeLog()\`** static method logs the current value of a timer that was previously started by calling console/time_static.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/timeLog_static)
     */
    timeLog(label?: string, ...data: any[]): void;
    timeStamp(label?: string): void;
    /**
     * The **\`console.trace()\`** static method outputs a stack trace to the console.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/trace_static)
     */
    trace(...data: any[]): void;
    /**
     * The **\`console.warn()\`** static method outputs a warning message to the console at the 'warning' log level.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/warn_static)
     */
    warn(...data: any[]): void;
}

declare var console: Console;
type NoteNames = "a" | "A" | "b" | "B" | "c" | "C" | "d" | "D" | "e" | "E" | "f" | "F" | "g" | "G"
type Accidental = "" | "#" | "b"
type Note = \`\${NoteNames}\${Accidental}\${number | ''}\`

type HZ = \`\${number}hz\` | \`\${number}Hz\`

type MidiNote = \`\${number}m\`

type CaseVariants<T extends string> = 
  | Lowercase<T>
  | Uppercase<T>
  | Capitalize<T>;

type ModeString =
  // Ionian (Major)
  | \`M \${string}\`
  | "M"
  | \`\${string}\${CaseVariants<"maj">}\${string}\`
  | \`\${string}\${CaseVariants<"major">}\${string}\`
  | \`\${string}\${CaseVariants<"ionian">}\${string}\`
  
  // Harmonic Minor
  | \`\${string}\${CaseVariants<"har">} \${CaseVariants<"minor">}\${string}\`
  | \`\${string}\${CaseVariants<"harmonic">}\${CaseVariants<"minor">}\${string}\`
  | \`\${string}\${CaseVariants<"harmonic">} \${CaseVariants<"minor">}\${string}\`
  
  // Melodic Minor
  | \`\${string}\${CaseVariants<"mel">} \${CaseVariants<"minor">}\${string}\`
  | \`\${string}\${CaseVariants<"melodic">}\${CaseVariants<"minor">}\${string}\`
  | \`\${string}\${CaseVariants<"melodic">} \${CaseVariants<"minor">}\${string}\`
  
  // Pentatonic Major
  | \`\${string}\${CaseVariants<"pentatonic">} \${CaseVariants<"major">}\${string}\`
  | \`\${string}\${CaseVariants<"pentatonic">} \${CaseVariants<"maj">}\${string}\`
  | \`\${string}\${CaseVariants<"pent">} \${CaseVariants<"maj">}\${string}\`
  | \`\${string}\${CaseVariants<"pent">} \${CaseVariants<"major">}\${string}\`
  
  // Pentatonic Minor
  | \`\${string}\${CaseVariants<"pentatonic">} \${CaseVariants<"minor">}\${string}\`
  | \`\${string}\${CaseVariants<"pentatonic">} \${CaseVariants<"min">}\${string}\`
  | \`\${string}\${CaseVariants<"pent">} \${CaseVariants<"min">}\${string}\`
  | \`\${string}\${CaseVariants<"pent">} \${CaseVariants<"minor">}\${string}\`
  
  // Blues
  | \`\${string}\${CaseVariants<"blues">}\${string}\`
  
  // Chromatic
  | \`\${string}\${CaseVariants<"chromatic">}\${string}\`
  
  // Whole Tone
  | \`\${string}\${CaseVariants<"whole">} \${CaseVariants<"tone">}\${string}\`
  | \`\${string}\${CaseVariants<"whole">}\${CaseVariants<"tone">}\${string}\`
  
  // Aeolian (Minor)
  | \`m \${string}\`
  | "m"
  | \`\${string}\${CaseVariants<"min">}\${string}\`
  | \`\${string}\${CaseVariants<"minor">}\${string}\`
  | \`\${string}\${CaseVariants<"aeolian">}\${string}\`
  
  // Dorian (start of string)
  | \`\${CaseVariants<"dorian">}\${string}\`
  
  // Locrian (start of string)
  | \`\${CaseVariants<"locrian">}\${string}\`
  
  // Mixolydian (start of string)
  | \`\${CaseVariants<"mixolydian">}\${string}\`
  
  // Phrygian (start of string)
  | \`\${CaseVariants<"phrygian">}\${string}\`
  
  // Lydian (start of string)
  | \`\${CaseVariants<"lydian">}\${string}\`;

type Scale = \`\${number}s(\${Note}:\${ModeString})\`

type OrArray<T> = T | T[];

// Core DSL types used by the generated declarations
type Signal = number | Note | HZ | MidiNote | Scale | ModuleOutput;

type PolySignal = OrArray<Signal> | Iterable<ModuleOutput>;

/** Options for stereo output routing */
interface StereoOutOptions {
  /** Output gain. If set, a scaleAndShift module is added after the stereo mix */
  gain?: PolySignal;
  /** Pan position (-5 = left, 0 = center, +5 = right). Default 0 */
  pan?: PolySignal;
  /** Stereo width/spread (0 = no spread, 5 = full spread). Default 0 */
  width?: Signal;
}

interface ModuleOutput {
  readonly moduleId: string;
  readonly portName: string;
  readonly channel: number;
  gain(factor: PolySignal): ModuleOutput;
  shift(offset: PolySignal): ModuleOutput;
  /**
   * Add scope visualization for this output
   * @param config - Scope configuration options
   * @param config.msPerFrame - Time window in milliseconds (default 500)
   * @param config.triggerThreshold - Trigger threshold in volts (optional)
   * @param config.scale - Voltage scale for display, shows -scale to +scale (default 5)
   */
  scope(config?: { msPerFrame?: number; triggerThreshold?: number; scale?: number }): this;
  /**
   * Send this output to speakers as stereo
   * @param baseChannel - Base output channel (0-15, default 0). Left plays on baseChannel, right on baseChannel+1
   * @param options - Stereo output options (gain, pan, width)
   */
  out(baseChannel?: number, options?: StereoOutOptions): this;
  /**
   * Send this output to speakers as mono
   * @param channel - Output channel (0-15, default 0)
   * @param gain - Output gain (optional)
   */
  outMono(channel?: number, gain?: PolySignal): this;
}

interface ModuleOutputWithRange extends ModuleOutput {
  readonly minValue: number;
  readonly maxValue: number;
  range(outMin: PolySignal, outMax: PolySignal): ModuleOutput;
}

/**
 * Collection of ModuleOutput instances with chainable DSP methods.
 * Supports iteration, indexing, and spreading.
 */
interface Collection extends Iterable<ModuleOutput> {
  readonly length: number;
  readonly [index: number]: ModuleOutput;
  [Symbol.iterator](): Iterator<ModuleOutput>;
  gain(factor: PolySignal): Collection;
  shift(offset: PolySignal): Collection;
  /**
   * Add scope visualization for the first output in the collection
   * @param config - Scope configuration options
   * @param config.msPerFrame - Time window in milliseconds (default 500)
   * @param config.triggerThreshold - Trigger threshold in volts (optional)
   * @param config.scale - Voltage scale for display, shows -scale to +scale (default 5)
   */
  scope(config?: { msPerFrame?: number; triggerThreshold?: number; scale?: number }): this;
  /**
   * Send all outputs to speakers as stereo
   * @param baseChannel - Base output channel (0-14, default 0). Left plays on baseChannel, right on baseChannel+1
   * @param options - Stereo output options (gain, pan, width)
   */
  out(baseChannel?: number, options?: StereoOutOptions): this;
  /**
   * Send all outputs to speakers as mono
   * @param channel - Output channel (0-15, default 0)
   * @param gain - Output gain (optional)
   */
  outMono(channel?: number, gain?: PolySignal): this;
  range(inMin: PolySignal, inMax: PolySignal, outMin: PolySignal, outMax: PolySignal): Collection;
}

/**
 * Collection of ModuleOutputWithRange instances.
 * Use .range(outMin, outMax) to remap using stored min/max values.
 */
interface CollectionWithRange extends Iterable<ModuleOutputWithRange> {
  readonly length: number;
  readonly [index: number]: ModuleOutputWithRange;
  [Symbol.iterator](): Iterator<ModuleOutputWithRange>;
  gain(factor: PolySignal): Collection;
  shift(offset: PolySignal): Collection;
  /**
   * Add scope visualization for the first output in the collection
   * @param config - Scope configuration options
   * @param config.msPerFrame - Time window in milliseconds (default 500)
   * @param config.triggerThreshold - Trigger threshold in volts (optional)
   * @param config.scale - Voltage scale for display, shows -scale to +scale (default 5)
   */
  scope(config?: { msPerFrame?: number; triggerThreshold?: number; scale?: number }): this;
  /**
   * Send all outputs to speakers as stereo
   * @param baseChannel - Base output channel (0-14, default 0). Left plays on baseChannel, right on baseChannel+1
   * @param options - Stereo output options (gain, pan, width)
   */
  out(baseChannel?: number, options?: StereoOutOptions): this;
  /**
   * Send all outputs to speakers as mono
   * @param channel - Output channel (0-15, default 0)
   * @param gain - Output gain (optional)
   */
  outMono(channel?: number, gain?: PolySignal): this;
  range(outMin: PolySignal, outMax: PolySignal): Collection;
}

// Helper functions exposed by the DSL runtime
declare function hz(frequency: number): number;
declare function note(noteName: string): number;
declare function bpm(beatsPerMinute: number): number;

/**
 * Create a Collection from ModuleOutput instances.
 * @example $(osc1, osc2).gain(0.5).out()
 */
declare function $(...args: ModuleOutput[]): Collection;

/**
 * Create a CollectionWithRange from ModuleOutputWithRange instances.
 * @example $r(...lfo).range(note('c3'), note('c5'))
 */
declare function $r(...args: ModuleOutputWithRange[]): CollectionWithRange;

/**
 * Set the global tempo for the root clock.
 * @param tempo - Tempo as a Signal (use bpm() helper, e.g., setTempo(bpm(140)))
 * @example setTempo(bpm(120)) // 120 beats per minute
 * @example setTempo(hz(2)) // 2 Hz = 120 BPM
 * @example setTempo(lfo.sine) // modulate tempo from LFO
 */
declare function setTempo(tempo: Signal): void;

/**
 * Set the global output gain applied to the final mix.
 * @param gain - Gain as a Signal (2.5 is default, 5.0 is unity gain)
 * @example setOutputGain(2.5) // 50% gain (default)
 * @example setOutputGain(5.0) // unity gain
 * @example setOutputGain(env.out) // modulate gain from envelope
 */
declare function setOutputGain(gain: Signal): void;
`;

export function buildLibSource(schemas: ModuleSchema[]): string {
    // console.log('buildLibSource schemas:', schemas);
    const schemaLib = generateDSL(schemas);
    return `declare global {\n${BASE_LIB_SOURCE}\n\n${schemaLib} \n}\n\n export {};\n`;
}

type JSONSchema = any;

type ClassSpec = {
    description?: string;
    outputs: Array<{ name: string; description?: string }>;
    properties: Array<{
        name: string;
        schema: JSONSchema;
        description?: string;
    }>;
    rootSchema: JSONSchema;
    moduleSchema: ModuleSchema;
};

type NamespaceNode = {
    namespaces: Map<string, NamespaceNode>;
    classes: Map<string, ClassSpec>;
    order: Array<{ kind: 'namespace' | 'class'; name: string }>;
};

function makeNamespaceNode(): NamespaceNode {
    return {
        namespaces: new Map(),
        classes: new Map(),
        order: [],
    };
}

function isValidIdentifier(name: string): boolean {
    return /^[$A-Z_][0-9A-Z_$]*$/i.test(name);
}

function renderPropertyKey(name: string): string {
    return isValidIdentifier(name) ? name : JSON.stringify(name);
}

function renderReadonlyPropertyKey(name: string): string {
    return isValidIdentifier(name) ? name : `[${JSON.stringify(name)}]`;
}

function renderDocComment(description?: string, indent: string = ''): string[] {
    if (!description) return [];
    const lines = description.split(/\r?\n/);
    return [
        `${indent}/**`,
        ...lines.map((l) => `${indent} * ${l}`),
        `${indent} */`,
    ];
}

function extractParamNamesFromDoc(description?: string): string[] {
    if (!description) return [];
    const names: string[] = [];
    const re = /@param\s+([^\s]+)/g;
    for (const match of description.matchAll(re)) {
        names.push(match[1]);
    }
    return names;
}

function resolveRef(
    ref: string,
    rootSchema: JSONSchema,
): JSONSchema | 'Signal' | 'PolySignal' {
    if (ref === 'Signal') return 'Signal';

    const defsPrefix = '#/$defs/';
    if (!ref.startsWith(defsPrefix)) {
        throw new Error(`Unsupported $ref: ${ref}`);
    }

    const defName = ref.slice(defsPrefix.length);
    if (defName === 'Signal') return 'Signal';
    if (defName === 'PolySignal') return 'PolySignal';

    const defs = rootSchema?.$defs;
    if (!defs || typeof defs !== 'object') {
        throw new Error(`Unresolved $ref: ${ref}`);
    }

    const resolved = defs[defName];
    if (!resolved) {
        throw new Error(`Unresolved $ref: ${ref}`);
    }

    if (resolved?.title === 'Signal') return 'Signal';
    if (resolved?.title === 'PolySignal') return 'PolySignal';
    return resolved;
}

function schemaToTypeExpr(schema: JSONSchema, rootSchema: JSONSchema): string {
    if (schema === null || schema === undefined) {
        throw new Error('Unsupported schema: null/undefined');
    }
    if (typeof schema === 'boolean') {
        throw new Error('Unsupported schema: boolean schema');
    }

    // Handle oneOf/anyOf - check if all variants resolve to Signal
    if (schema.oneOf || schema.anyOf) {
        const variants = schema.oneOf || schema.anyOf;
        if (Array.isArray(variants)) {
            // Check if this is an enum (all variants have 'const')
            const isEnum = variants.every((v: JSONSchema) => v.const !== undefined);
            if (isEnum) {
                return variants.map((v: any) => JSON.stringify(v.const)).join(' | ');
            }

            const types = variants.map((v: JSONSchema) => {
                try {
                    return schemaToTypeExpr(v, rootSchema);
                } catch {
                    return 'any';
                }
            });
            // If all variants are Signal, return Signal
            if (types.every((t) => t === 'Signal')) {
                return 'PolySignal';
            }
            // If it's a mix but includes Signal[], treat as Signal (for PolySignal)
            if (types.includes('Signal') && types.includes('Signal[]')) {
                return 'PolySignal';
            }
        }
        console.log('schema:', schema);
        return 'any';
    }
    if (schema.allOf) {
        console.log('schema:', schema);
        return 'any';
    }
    if (Array.isArray(schema.type)) {
        // Handle union type arrays like ["integer", "null"] or ["string", "null"]
        const types = schema.type as string[];
        const nonNullTypes = types.filter((t) => t !== 'null');
        if (nonNullTypes.length === 1) {
            // This is a nullable type, treat it as the non-null type (optional in TS)
            const singleType = nonNullTypes[0];
            if (singleType === 'integer' || singleType === 'number') return 'number';
            if (singleType === 'string') return 'string';
            if (singleType === 'boolean') return 'boolean';
        }
        // Fall back to union of all non-null types
        const mapped = nonNullTypes.map((t) => {
            if (t === 'integer' || t === 'number') return 'number';
            if (t === 'string') return 'string';
            if (t === 'boolean') return 'boolean';
            return 'any';
        });
        return mapped.length > 0 ? mapped.join(' | ') : 'any';
    }

    if (schema.$ref) {
        const resolved = resolveRef(String(schema.$ref), rootSchema);
        if (resolved === 'Signal') return 'Signal';
        if (resolved === 'PolySignal') return 'PolySignal';
        return schemaToTypeExpr(resolved, rootSchema);
    }

    if (schema.enum) {
        if (!Array.isArray(schema.enum) || schema.enum.length === 0) {
            throw new Error('Unsupported enum schema');
        }
        return schema.enum.map((v: any) => JSON.stringify(v)).join(' | ');
    }

    const type = schema.type;

    if (type === 'integer' || type === 'number') return 'number';
    if (type === 'string') return 'string';
    if (type === 'boolean') return 'boolean';

    const looksLikeObject =
        type === 'object' ||
        (!!schema.properties && typeof schema.properties === 'object');
    if (looksLikeObject) {
        const props = schema.properties;
        if (!props || typeof props !== 'object') return '{}';

        const requiredSet = new Set<string>(
            Array.isArray(schema.required) ? schema.required : [],
        );
        const entries = Object.entries(props as Record<string, JSONSchema>);
        if (entries.length === 0) return '{}';

        const parts: string[] = [];
        for (const [propName, propSchema] of entries) {
            const optional = requiredSet.has(propName) ? '' : '?';
            parts.push(
                `${renderPropertyKey(propName)}${optional}: ${schemaToTypeExpr(propSchema, rootSchema)}`,
            );
        }
        return `{ ${parts.join('; ')} }`;
    }

    if (type === 'array') {
        if (Array.isArray(schema.prefixItems)) {
            const items = schema.prefixItems as JSONSchema[];
            const tuple = items
                .map((s) => schemaToTypeExpr(s, rootSchema))
                .join(', ');
            return `[${tuple}]`;
        }
        if (schema.items) {
            return `${schemaToTypeExpr(schema.items, rootSchema)}[]`;
        }
        throw new Error('Unsupported array schema: missing items/prefixItems');
    }

    if (type === undefined) {
        // If there's a $ref we didn't catch, or other structural hints, try to handle
        if (schema.$ref) {
            const resolved = resolveRef(String(schema.$ref), rootSchema);
            if (resolved === 'Signal') return 'Signal';
            if (resolved === 'PolySignal') return 'PolySignal';
            return schemaToTypeExpr(resolved, rootSchema);
        }
        // Schema with only 'const' (used in tagged unions)
        if (schema.const !== undefined) {
            return JSON.stringify(schema.const);
        }
        console.error('Schema with missing type:', JSON.stringify(schema, null, 2));
        throw new Error('Unsupported schema: missing type');
    }

    throw new Error(`Unsupported scalar type: ${type}`);
}

function getMethodArgsForProperty(
    propertySchema: JSONSchema,
    rootSchema: JSONSchema,
    propertyDescription?: string,
): Array<{ name: string; type: string }> {
    const paramNames = extractParamNamesFromDoc(propertyDescription);

    // Top-level tuple expansion into multiple arguments.
    if (
        propertySchema &&
        typeof propertySchema === 'object' &&
        propertySchema.type === 'array' &&
        Array.isArray(propertySchema.prefixItems)
    ) {
        const items: JSONSchema[] = propertySchema.prefixItems;
        return items.map((itemSchema, index) => {
            const name =
                paramNames.length > 0
                    ? (paramNames[index] ?? `arg${index + 1}`)
                    : `arg${index + 1}`;
            return { name, type: schemaToTypeExpr(itemSchema, rootSchema) };
        });
    }

    // Single-argument method.
    const name = paramNames.length > 0 ? (paramNames[0] ?? 'arg1') : 'arg';
    return [{ name, type: schemaToTypeExpr(propertySchema, rootSchema) }];
}

function buildTreeFromSchemas(schemas: ModuleSchema[]): NamespaceNode {
    const root = makeNamespaceNode();

    for (const moduleSchema of schemas) {
        const fullName = String(moduleSchema.name).trim();
        if (!fullName) {
            throw new Error('ModuleSchema is missing a non-empty name');
        }

        const paramsSchema = moduleSchema.paramsSchema;
        if (!paramsSchema || typeof paramsSchema !== 'object') {
            throw new Error(`ModuleSchema ${fullName} is missing paramsSchema`);
        }

        const parts = fullName.split('.').filter((p: string) => p.length > 0);
        if (parts.length === 0) {
            throw new Error(`Invalid ModuleSchema name: ${fullName}`);
        }

        const className = parts[parts.length - 1];
        const namespacePath = parts.slice(0, -1);

        let node = root;
        for (const ns of namespacePath) {
            let child = node.namespaces.get(ns);
            if (!child) {
                child = makeNamespaceNode();
                node.namespaces.set(ns, child);
                node.order.push({ kind: 'namespace', name: ns });
            }
            node = child;
        }

        if (node.classes.has(className)) {
            throw new Error(`Duplicate class name detected: ${fullName}`);
        }
        if ('properties' in paramsSchema === false) {
            throw new Error(
                `ModuleSchema ${fullName} paramsSchema is missing properties`,
            );
        }
        const propsObj = paramsSchema.properties;
        const propsEntries =
            propsObj && typeof propsObj === 'object'
                ? Object.entries(propsObj as Record<string, JSONSchema>)
                : [];

        const properties = propsEntries.map(([name, propSchema]) => ({
            name,
            schema: propSchema,
            description: propSchema?.description,
        }));

        const outputs = (
            Array.isArray(moduleSchema.outputs) ? moduleSchema.outputs : []
        )
            .map((o) => ({
                name: String(o?.name ?? '').trim(),
                description: o?.description,
            }))
            .filter((o) => o.name.length > 0);

        node.classes.set(className, {
            description: moduleSchema.description,
            outputs,
            properties,
            rootSchema: paramsSchema,
            moduleSchema,
        });
        node.order.push({ kind: 'class', name: className });
    }

    return root;
}

function renderNodeInterfaceName(baseName: string): string {
    return baseName.endsWith('Node') ? baseName : `${baseName}Node`;
}

function capitalizeName(name: string): string {
    if (!name) return name;
    return name.charAt(0).toUpperCase() + name.slice(1);
}

function renderParamsInterface(
    baseName: string,
    classSpec: ClassSpec,
    indent: string,
): string[] {
    const lines: string[] = [];
    const paramsInterfaceName = `${capitalizeName(baseName)}Params`;
    lines.push(`${indent}export interface ${paramsInterfaceName} {`);

    for (const prop of classSpec.properties) {
        lines.push('');
        lines.push(...renderDocComment(prop.description, indent + '  '));
        const type = schemaToTypeExpr(prop.schema, classSpec.rootSchema);
        lines.push(`${indent}  ${renderPropertyKey(prop.name)}?: ${type};`);
    }
    lines.push(`${indent}}`);
    return lines;
}

/**
 * Convert snake_case to camelCase
 */
function toCamelCase(str: string): string {
    return str.replace(/_([a-z])/g, (_, letter: string) => letter.toUpperCase());
}

/**
 * Reserved property names that conflict with ModuleOutput, Collection, or CollectionWithRange methods/properties.
 * Output names matching these will be suffixed with an underscore.
 *
 * IMPORTANT: When adding new methods to any type that a factory function could return
 * (ModuleOutput, ModuleOutputWithRange, BaseCollection, Collection, CollectionWithRange),
 * the method name MUST be added to this list. Keep in sync with:
 * - crates/modular_derive/src/lib.rs (RESERVED_OUTPUT_NAMES)
 * - src/dsl/factories.ts (RESERVED_OUTPUT_NAMES)
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
 * Get the output type for a single output definition
 */
function getOutputType(output: { polyphonic?: boolean; minValue?: number; maxValue?: number }): string {
    const hasRange = output.minValue !== undefined && output.maxValue !== undefined;
    if (output.polyphonic) {
        return hasRange ? 'CollectionWithRange' : 'Collection';
    }
    return hasRange ? 'ModuleOutputWithRange' : 'ModuleOutput';
}

/**
 * Generate interface name for multi-output modules
 */
function getMultiOutputInterfaceName(moduleSchema: ModuleSchema): string {
    const parts = moduleSchema.name.split('.').filter((p: string) => p.length > 0);
    const baseName = parts[parts.length - 1];
    return `${capitalizeName(baseName)}Outputs`;
}

/**
 * Generate interface definition for multi-output modules.
 * The interface extends from the default output's type and includes properties for other outputs.
 */
function generateMultiOutputInterface(moduleSchema: ModuleSchema, indent: string): string[] {
    const outputs = moduleSchema.outputs || [];
    if (outputs.length <= 1) return [];

    // Find the default output
    const defaultOutput = outputs.find((o: any) => o.default) || outputs[0];
    const defaultOutputMeta = defaultOutput as { polyphonic?: boolean; minValue?: number; maxValue?: number };
    const baseType = getOutputType(defaultOutputMeta);
    
    const interfaceName = getMultiOutputInterfaceName(moduleSchema);
    
    const lines: string[] = [];
    lines.push(`${indent}/**`);
    lines.push(`${indent} * Output type for ${moduleSchema.name} module.`);
    lines.push(`${indent} * Extends ${baseType} (default output: ${defaultOutput.name})`);
    lines.push(`${indent} */`);
    lines.push(`${indent}export interface ${interfaceName} extends ${baseType} {`);
    
    // Add properties for non-default outputs
    for (const output of outputs) {
        if (output.name === defaultOutput.name) continue;
        
        const outputMeta = output as { polyphonic?: boolean; minValue?: number; maxValue?: number; description?: string };
        const outputType = getOutputType(outputMeta);
        const safeName = sanitizeOutputName(output.name);
        
        if (output.description) {
            lines.push(`${indent}  /** ${output.description} */`);
        }
        lines.push(`${indent}  readonly ${safeName}: ${outputType};`);
    }
    
    lines.push(`${indent}}`);
    return lines;
}

/**
 * Get the return type for a module factory based on its outputs
 */
function getFactoryReturnType(moduleSchema: ModuleSchema): string {
    const outputs = moduleSchema.outputs || [];
    
    if (outputs.length === 0) {
        return 'void';
    } else if (outputs.length === 1) {
        const output = outputs[0] as { polyphonic?: boolean; minValue?: number; maxValue?: number };
        return getOutputType(output);
    } else {
        // Multiple outputs - return the generated interface name
        return getMultiOutputInterfaceName(moduleSchema);
    }
}

function renderFactoryFunction(
    moduleSchema: ModuleSchema,
    _interfaceName: string,
    indent: string,
): string[] {
    const functionName = moduleSchema.name.split('.').pop()!;

    let args: string[] = [];
    // @ts-ignore
    const positionalArgs = moduleSchema.positionalArgs || [];

    // Build docstring lines
    const docLines: string[] = [];
    if (moduleSchema.description) {
        docLines.push(...moduleSchema.description.split(/\r?\n/));
    }

    for (const arg of positionalArgs) {
        // @ts-ignore
        const propSchema = moduleSchema.paramsSchema.properties?.[arg.name];
        // @ts-ignore
        const type = propSchema
            ? schemaToTypeExpr(propSchema, moduleSchema.paramsSchema)
            : 'any';
        const optional = arg.optional ? '?' : '';
        args.push(`${arg.name}${optional}: ${type}`);

        // Add @param for positional arg
        const description = propSchema?.description;
        if (description) {
            const firstLine = description.split(/\r?\n/)[0];
            docLines.push(`@param ${arg.name} - ${firstLine}`);
        } else {
            docLines.push(`@param ${arg.name}`);
        }
    }

    // @ts-ignore
    const allParamKeys = Object.keys(
        moduleSchema.paramsSchema.properties || {},
    );
    // @ts-ignore
    const positionalKeys = new Set(positionalArgs.map((a: any) => a.name));

    const configProps: string[] = [];
    const configParamDocs: string[] = [];

    for (const key of allParamKeys) {
        if (!positionalKeys.has(key)) {
            // @ts-ignore
            const propSchema = moduleSchema.paramsSchema.properties[key];
            // @ts-ignore
            const type = schemaToTypeExpr(
                propSchema,
                moduleSchema.paramsSchema,
            );
            configProps.push(`${key}?: ${type}`);

            // Collect config param descriptions
            const description = propSchema?.description;
            if (description) {
                const firstLine = description.split(/\r?\n/)[0];
                configParamDocs.push(`${key} - ${firstLine}`);
            }
        }
    }

    configProps.push(`id?: string`);

    const configType = `{ ${configProps.join('; ')} }`;

    args.push(`config?: ${configType}`);

    // Add @param config with nested property descriptions
    if (configParamDocs.length > 0) {
        docLines.push(`@param config - Configuration object`);
        for (const doc of configParamDocs) {
            docLines.push(`  - ${doc}`);
        }
    } else {
        docLines.push(`@param config - Configuration object`);
    }

    // Get return type based on outputs
    const returnType = getFactoryReturnType(moduleSchema);

    const lines: string[] = [];
    if (docLines.length > 0) {
        lines.push(`${indent}/**`);
        for (const line of docLines) {
            lines.push(`${indent} * ${line}`);
        }
        lines.push(`${indent} */`);
    }
    lines.push(
        `${indent}export function ${functionName}(${args.join(', ')}): ${returnType};`,
    );

    return lines;
}

function getQualifiedNodeInterfaceType(moduleName: string): string {
    const parts = moduleName.split('.').filter((p) => p.length > 0);
    if (parts.length === 0) {
        throw new Error(`Invalid ModuleSchema name: ${moduleName}`);
    }
    const base = parts[parts.length - 1];
    const interfaceName = renderNodeInterfaceName(capitalizeName(base));
    const namespaces = parts.slice(0, -1);
    return namespaces.length > 0
        ? `${namespaces.join('.')}.${interfaceName}`
        : interfaceName;
}

function renderInterface(
    baseName: string,
    classSpec: ClassSpec,
    indent: string,
): string[] {
    const lines: string[] = [];

    // Generate multi-output interface if needed
    const multiOutputInterface = generateMultiOutputInterface(classSpec.moduleSchema, indent);
    if (multiOutputInterface.length > 0) {
        lines.push(...multiOutputInterface);
        lines.push('');
    }

    // Render the factory function
    lines.push(
        ...renderFactoryFunction(classSpec.moduleSchema, '', indent),
    );
    return lines;
}

function renderTree(node: NamespaceNode, indentLevel: number = 0): string[] {
    const indent = '  '.repeat(indentLevel);
    const lines: string[] = [];

    for (const item of node.order) {
        if (item.kind === 'class') {
            const classSpec = node.classes.get(item.name);
            if (!classSpec) continue;
            lines.push(...renderInterface(item.name, classSpec, indent));
            lines.push('');
            continue;
        }

        const child = node.namespaces.get(item.name);
        if (!child) continue;
        lines.push(`${indent}export declare namespace ${item.name} {`);
        lines.push(...renderTree(child, indentLevel + 1));
        lines.push(`${indent}}`);
        lines.push('');
    }

    // Trim extra blank lines at this level.
    while (lines.length > 0 && lines[lines.length - 1] === '') {
        lines.pop();
    }
    return lines;
}

export function generateDSL(schemas: ModuleSchema[]): string {
    if (!Array.isArray(schemas)) {
        throw new Error('generateDSL expects an array of ModuleSchema');
    }
    const tree = buildTreeFromSchemas(schemas);
    const lines = renderTree(tree, 0);

    const clockSchema = schemas.find((s) => s.name === 'clock');
    if (clockSchema) {
        lines.push('');
        lines.push('/** Default clock module running at 120 BPM. */');
        const clockReturnType = getFactoryReturnType(clockSchema);
        lines.push(
            `export declare const rootClock: ${clockReturnType};`,
        );
    }

    const signalSchema = schemas.find((s) => s.name === 'signal');
    if (signalSchema) {
        lines.push('');
        lines.push('/** Input signals. */');
        const signalReturnType = getFactoryReturnType(signalSchema);
        lines.push(
            `export declare const input: readonly ${signalReturnType};`,
        );
    }

    return lines.join('\n') + '\n';
}
