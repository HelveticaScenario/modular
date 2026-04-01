/**
 * Shared type documentation for DSL types.
 * Used by both TypeScript lib generator (JSDoc) and HelpWindow (rendered docs).
 */

export interface TypeMethod {
    name: string;
    signature: string;
    description: string;
    example?: string;
}

export interface TypeDocumentation {
    name: string;
    description: string;
    definition?: string;
    examples: string[];
    seeAlso: string[];
    methods?: TypeMethod[];
}

/**
 * All DSL type names that should be linkified in documentation.
 */
export const DSL_TYPE_NAMES = [
    'Signal',
    'Poly<Signal>',
    'Mono<Signal>',
    'ModuleOutput',
    'ModuleOutputWithRange',
    'Collection',
    'CollectionWithRange',
    'Note',
    'HZ',
    'MidiNote',
    'Scale',
    'StereoOutOptions',
] as const;

export type DslTypeName = (typeof DSL_TYPE_NAMES)[number];

/**
 * Comprehensive documentation for all DSL types.
 */
export const TYPE_DOCS: Record<DslTypeName, TypeDocumentation> = {
    Signal: {
        name: 'Signal',
        description:
            'A single-channel audio signal value. This is the fundamental type for all audio connections in the modular system. ' +
            'Signals follow the 1V/octave convention where 0V corresponds to C4 (~261.63 Hz).',
        definition: 'number | Note | HZ | MidiNote | Scale | ModuleOutput',
        examples: [
            '$sine("C4").out()           // Note string - converted to 1V/oct',
            '$sine(440).out()            // Number - constant voltage',
            '$sine("440hz").out()        // Hz string - converted to voltage',
            '$sine("60m").out()          // MIDI note 60 (middle C)',
            '$sine($sine("1hz")).out()   // ModuleOutput from another module',
            '$sine("4s(C:major)").out()  // Scale pattern',
        ],
        seeAlso: [
            'Poly<Signal>',
            'Mono<Signal>',
            'ModuleOutput',
            'Note',
            'HZ',
            'MidiNote',
            'Scale',
        ],
    },

    'Poly<Signal>': {
        name: 'Poly<Signal>',
        description:
            'A potentially multi-channel signal. Can be an array of Signals for polyphonic patches, ' +
            'or an iterable of ModuleOutputs. When used as input to a module, arrays are expanded to create multiple voices.',
        definition: 'Signal | Signal[] | Iterable<ModuleOutput>',
        examples: [
            '$lpf($sine(["C3", "E3", "G3"]), "C5").out()  // 3-voice chord',
            '$sine(["C3", "E3", "G3"]).out()              // Array of notes for polyphony',
        ],
        seeAlso: ['Signal', 'Mono<Signal>', 'ModuleOutput', 'Collection'],
    },

    'Mono<Signal>': {
        name: 'Mono<Signal>',
        description:
            'A signal input that accepts polyphonic connections but sums all channels down to a single mono value. ' +
            'Structurally identical to Poly<Signal>, but signals that the module will not produce per-voice output from this parameter. ' +
            'Used for control parameters like tempo, stereo width, or math variables where a single combined value is needed.',
        definition: 'Signal | Signal[] | Iterable<ModuleOutput>',
        examples: [
            '$clockDivider($clock.beatTrigger, 4).out()       // Clock signal summed to mono',
            '$stereoMix($sine(["C3", "E3", "G3"]), { width: $sine("1hz") }).out()  // Width control summed to mono',
            '$math("x + y", { x: $sine("C4")[0], y: $saw("E4")[0] }).out()  // Variables summed to single values',
        ],
        seeAlso: ['Signal', 'Poly<Signal>', 'ModuleOutput'],
    },

    ModuleOutput: {
        name: 'ModuleOutput',
        description:
            'A single output from a module, representing a mono signal connection. ' +
            'ModuleOutputs are chainable - methods like amplitude(), shift(), and out() return the same output for fluent API usage. ' +
            'Every module factory returns either a ModuleOutput or a Collection of outputs.',
        definition:
            'interface { moduleId: string; portName: string; channel: number; ... }',
        examples: [
            '$sine("c4").amplitude(0.5).out()       // Chain methods',
            '$sine("c4").scope().out()              // Add visualization',
            "$lpf($sine('c4'), 'c3').out()          // Use as input to another module",
        ],
        seeAlso: ['ModuleOutputWithRange', 'Collection', 'Signal'],
        methods: [
            {
                name: 'amplitude',
                signature: 'amplitude(factor: Poly<Signal>): ModuleOutput',
                description:
                    'Scale the signal by a linear factor (5 = unity, 2.5 = half, 10 = 2x). Creates a $scaleAndShift module internally. For perceptual (audio-taper) volume control, use gain() instead.',
                example: '$sine("c4").amplitude(2.5).out()  // Half amplitude',
            },
            {
                name: 'amp',
                signature: 'amp(factor: Poly<Signal>): ModuleOutput',
                description:
                    'Alias for amplitude(). Scale the signal by a factor.',
                example: '$sine("c4").amp(2.5).out()  // Half amplitude',
            },
            {
                name: 'shift',
                signature: 'shift(offset: Poly<Signal>): ModuleOutput',
                description:
                    'Add a DC offset to the signal. Creates a $scaleAndShift module internally.',
                example: '$sine("1hz").shift(2.5).out()  // Shift LFO to 0-5V range',
            },
            {
                name: 'gain',
                signature: 'gain(level: Poly<Signal>): ModuleOutput',
                description:
                    'Scale the signal with a perceptual (audio taper) curve (5 = unity, 0 = silence). Chains $curve and $scaleAndShift internally with exponent 3. For linear amplitude scaling, use amplitude() instead.',
                example: '$sine("c4").gain(2.5).out()',
            },
            {
                name: 'exp',
                signature: 'exp(factor?: Poly<Signal>): ModuleOutput',
                description:
                    'Apply a power curve to the signal. Creates a $curve module internally. Default exponent is 3.',
                example: '$sine("1hz").exp(2).out()  // Quadratic curve',
            },
            {
                name: 'scope',
                signature:
                    'scope(config?: { msPerFrame?: number; triggerThreshold?: number; range?: [number, number] }): this',
                description:
                    'Add an oscilloscope visualization for this output. The scope appears as an overlay in the editor.',
                example:
                    '$sine("c4").scope({ msPerFrame: 100, range: [-10, 10] }).out()',
            },
            {
                name: 'out',
                signature: 'out(options?: StereoOutOptions): this',
                description:
                    'Send this output to the speakers as stereo audio. Left plays on baseChannel, right on baseChannel+1.',
                example: '$sine("c4").out({ baseChannel:0, gain: 2.5, pan: -2 })',
            },
            {
                name: 'outMono',
                signature:
                    'outMono(channel?: number, gain?: Poly<Signal>): this',
                description:
                    'Send this output to a single speaker channel as mono audio.',
                example: '$sine("1hz").outMono(2)',
            },
            {
                name: 'pipe',
                signature: 'pipe<T>(pipeFn: (self: this) => T): T',
                description:
                    'Pass this output through a transform function and return the result. ' +
                    'Enables inline transforms and reusable signal-processing helpers.',
                example: "$saw('c4').pipe(s => s.amplitude(0.5)).out()",
            },
            {
                name: 'pipe',
                signature:
                    'pipe<T>(pipeFn: (self: this, item: E) => T, array: E[]): Collection',
                description:
                    'Call pipeFn once for every element in the provided array, ' +
                    'collecting all results into a Collection. ' +
                    'Useful for generating a family of voices that each vary by one parameter.',
                example:
                    "$sine(['C4', 'E4', 'G4']).pipe((s, cut) => $lpf(s, cut), ['440hz', '880hz']).out()",
            },
            {
                name: 'pipeMix',
                signature:
                    'pipeMix(pipeFn: (self: this) => ModuleOutput | Collection, options?: { mode?: "sum" | "average" | "max" | "min"; gain?: Poly<Signal> }): Collection',
                description:
                    'Pipe this output through a transform, then mix the original and transformed signals together using a $mix module. ' +
                    'The callback receives this output and returns a second signal; both are passed as inputs to $mix.',
                example: "$saw('c4').pipeMix(s => $lpf(s, '1000hz')).out()",
            },
            {
                name: 'range',
                signature:
                    'range(outMin: Poly<Signal>, outMax: Poly<Signal>, inMin: Poly<Signal>, inMax: Poly<Signal>): ModuleOutput',
                description:
                    'Remap this output from an explicit input range to a new output range. Creates a $remap module internally.',
                example: "$sine('c4').range(0, 1, -5, 5)",
            },
        ],
    },

    ModuleOutputWithRange: {
        name: 'ModuleOutputWithRange',
        description:
            'An extension of ModuleOutput that knows its output value range (minValue, maxValue). ' +
            'Typically returned by LFOs, envelopes, and other modulation sources. ' +
            'The range() method uses the stored min/max for automatic scaling.',
        definition:
            'interface extends ModuleOutput { minValue: number; maxValue: number; range(...): ModuleOutput }',
        examples: [
            "$sine('1hz')[0].range(0, 5).out()              // LFO outputs 0 to +5",
        ],
        seeAlso: ['ModuleOutput', 'CollectionWithRange'],
        methods: [
            {
                name: 'range',
                signature:
                    'range(outMin: Poly<Signal>, outMax: Poly<Signal>): ModuleOutput',
                description:
                    'Remap the output from its native range (minValue, maxValue) to a new range (outMin, outMax). ' +
                    'Unlike Collection.range(), this uses the stored min/max values automatically.',
                example:
                    '$sine("1hz")[0].range($note("c3"), $note("c5")).out()  // Remap LFO to pitch range',
            },
        ],
    },

    Collection: {
        name: 'Collection',
        description:
            'A collection of ModuleOutput instances with chainable DSP methods. ' +
            'Created with the $() helper function. Supports iteration, indexing, and spreading. ' +
            'Methods operate on all outputs in the collection.',
        definition:
            'interface extends Iterable<ModuleOutput> { length: number; [index]: ModuleOutput; ... }',
        examples: [
            '$c(...$sine("C4"), ...$saw("E4")).amplitude(0.5).out()  // Apply amplitude to all, send to output',
            '$c($noise("white"), $noise("pink")).out()              // Collect and output',
        ],
        seeAlso: ['CollectionWithRange', 'ModuleOutput', 'Poly<Signal>'],
        methods: [
            {
                name: 'amplitude',
                signature: 'amplitude(factor: Poly<Signal>): Collection',
                description:
                    'Scale all signals in the collection by a linear factor (5 = unity, 2.5 = half, 10 = 2x). For perceptual (audio-taper) volume control, use gain() instead.',
                example: '$c(...$sine("C4"), ...$saw("E4")).amplitude(2.5).out()',
            },
            {
                name: 'amp',
                signature: 'amp(factor: Poly<Signal>): Collection',
                description:
                    'Alias for amplitude(). Scale all signals in the collection by a factor.',
                example: '$c(...$sine("C4"), ...$saw("E4")).amp(0.5).out()',
            },
            {
                name: 'shift',
                signature: 'shift(offset: Poly<Signal>): Collection',
                description:
                    'Add a DC offset to all signals in the collection.',
                example: '$c(...$sine("1hz"), ...$saw("2hz")).shift(2.5).out()',
            },
            {
                name: 'gain',
                signature: 'gain(level: Poly<Signal>): Collection',
                description:
                    'Scale all signals with a perceptual (audio taper) curve (5 = unity, 0 = silence). For linear amplitude scaling, use amplitude() instead.',
                example: '$c(...$sine("C4"), ...$saw("E4")).gain(2.5).out()',
            },
            {
                name: 'exp',
                signature: 'exp(factor?: Poly<Signal>): Collection',
                description:
                    'Apply a power curve to all signals in the collection. Default exponent is 3.',
                example: '$c(...$sine("1hz"), ...$saw("2hz")).exp(2).out()',
            },
            {
                name: 'scope',
                signature:
                    'scope(config?: { msPerFrame?: number; triggerThreshold?: number; range?: [number, number] }): this',
                description:
                    'Add scope visualization for the first output in the collection.',
                example: '$c(...$sine("C4"), ...$saw("E4")).scope().out()',
            },
            {
                name: 'out',
                signature: 'out(options?: StereoOutOptions): this',
                description:
                    'Send all outputs to speakers as stereo, summed together.',
                example: '$c(...$sine("C4"), ...$saw("E4"), ...$pulse("G4")).out()',
            },
            {
                name: 'outMono',
                signature:
                    'outMono(channel?: number, gain?: Poly<Signal>): this',
                description:
                    'Send all outputs to a single speaker channel as mono, summed together.',
                example: '$c(...$sine("C4"), ...$saw("E4")).outMono(0)',
            },
            {
                name: 'range',
                signature:
                    'range(outMin: Poly<Signal>, outMax: Poly<Signal>, inMin: Poly<Signal>, inMax: Poly<Signal>): Collection',
                description:
                    'Remap all outputs from input range to output range. Requires explicit input min/max.',
                example: '$c(...$sine("1hz"), ...$saw("2hz")).range(0, 1, -5, 5).out()',
            },
            {
                name: 'pipe',
                signature: 'pipe<T>(pipeFn: (self: this) => T): T',
                description:
                    'Pass this collection through a transform function and return the result. ' +
                    'Enables inline transforms and reusable signal-processing helpers.',
                example: '$c(...$sine("C4"), ...$saw("E4")).pipe(all => all.amplitude(0.5)).out()',
            },
            {
                name: 'pipe',
                signature:
                    'pipe<T>(pipeFn: (self: this, item: E) => T, array: E[]): Collection',
                description:
                    'Call pipeFn once for every element in the provided array, ' +
                    'collecting all results into a Collection.',
                example:
                    "$c(...$sine('C4'), ...$saw('E4')).pipe((col, cutoff) => $lpf(col, cutoff), ['200hz', '800hz', '3200hz']).out()",
            },
            {
                name: 'pipeMix',
                signature:
                    'pipeMix(pipeFn: (self: this) => ModuleOutput | Collection, options?: { mode?: "sum" | "average" | "max" | "min"; gain?: Poly<Signal> }): Collection',
                description:
                    'Pipe this collection through a transform, then mix the original and transformed signals together using a $mix module. ' +
                    'The callback receives this collection and returns a second signal; both are passed as inputs to $mix.',
                example: "$c(...$sine('C4'), ...$saw('E4')).pipeMix(s => $lpf(s, '1000hz')).out()",
            },
        ],
    },

    CollectionWithRange: {
        name: 'CollectionWithRange',
        description:
            'A collection of ModuleOutputWithRange instances. ' +
            'Created with the $r() helper function. Like Collection, but the range() method uses stored min/max values.',
        definition: 'interface extends Iterable<ModuleOutputWithRange> { ... }',
        examples: [
            '$r(...$sine("1hz"), ...$saw("2hz")).range(0, 5).out()     // Remap using stored ranges',
        ],
        seeAlso: ['Collection', 'ModuleOutputWithRange'],
        methods: [
            {
                name: 'range',
                signature:
                    'range(outMin: Poly<Signal>, outMax: Poly<Signal>): Collection',
                description:
                    'Remap all outputs from their native ranges to a new range. ' +
                    "Uses each output's stored minValue/maxValue.",
                example: '$r(...$sine("1hz"), ...$saw("2hz")).range(200, 2000).out()',
            },
        ],
    },

    Note: {
        name: 'Note',
        description:
            'A musical note string in scientific pitch notation. ' +
            'Consists of a note name (A-G or a-g), optional accidental (#/b), and optional octave number. ' +
            'If octave is omitted, defaults to octave 4.',
        definition: '`${NoteName}${Accidental}${Octave}`',
        examples: [
            '"C4"   // Middle C',
            '"A#3"  // A sharp in octave 3',
            '"Bb5"  // B flat in octave 5',
            '"G"    // G4 (octave 4 is default)',
            '"c#"    // C#4 (octave 4 is default)',
        ],
        seeAlso: ['Signal', 'HZ', 'MidiNote'],
    },

    HZ: {
        name: 'HZ',
        description:
            'A frequency string specifying a value in Hertz. ' +
            'Case-insensitive suffix "hz". Converted to 1V/oct voltage internally.',
        definition: '`${number}hz` | `${number}Hz`',
        examples: [
            '"440hz"   // A4 concert pitch',
            '"261.63Hz" // Middle C',
            '"1000hz"  // 1 kHz',
        ],
        seeAlso: ['Signal', 'Note'],
    },

    MidiNote: {
        name: 'MidiNote',
        description:
            'A MIDI note number string. MIDI note 60 is middle C (C4). ' +
            'Converted to 1V/oct voltage internally.',
        definition: '`${number}m`',
        examples: [
            '"60m"  // Middle C (C4)',
            '"69m"  // A4 (440 Hz)',
            '"36m"  // C2',
        ],
        seeAlso: ['Signal', 'Note'],
    },

    Scale: {
        name: 'Scale',
        description:
            'A scale pattern string for generating multiple pitches. ' +
            'Format: "{count}s({root}:{mode})" where count is the number of notes, ' +
            'root is the root note, and mode is the scale type.',
        definition: '`${number}s(${Note}:${Mode})`',
        examples: [
            '"4s(C:major)"     // 4 notes of C major scale',
            '"8s(A:minor)"     // 8 notes of A minor scale',
            '"3s(G:dorian)"    // 3 notes of G dorian mode',
            '"5s(E:pentatonic minor)"  // E minor pentatonic',
        ],
        seeAlso: ['Signal', 'Note'],
    },

    StereoOutOptions: {
        name: 'StereoOutOptions',
        description:
            'Options for stereo output routing via the out() method. ' +
            'Controls base channel, gain, panning, and stereo width.',
        definition:
            'interface { baseChannel?: number; gain?: Poly<Signal>; pan?: Poly<Signal>; width?: Signal }',
        examples: [
            "$sine('c').out({ baseChannel: 4 })      // Output of channels 4 and 5",
            "$sine('c').out({ gain: 2.5 })           // 50% gain",
            "$sine('c').out({ pan: -2.5 })           // Pan left",
            "$sine('c').out({ width: 5 })            // Full stereo spread",
            "$sine('c').out({ gain: $perc($pulse('8hz')), pan: $sine('1hz') })  // Modulated",
        ],
        seeAlso: ['ModuleOutput', 'Collection', 'Poly<Signal>'],
    },
};

export interface GlobalFunctionDoc {
    name: string;
    signature: string;
    description: string;
    params?: Array<{ name: string; type: string; description: string }>;
    returns?: string;
    examples: string[];
    group: string;
}

/**
 * Documentation for all DSL global functions and helpers.
 */
export const GLOBAL_DOCS: GlobalFunctionDoc[] = [
    // ---- Helpers ----
    {
        name: '$hz',
        signature: '$hz(frequency: number): number',
        description:
            'Convert a frequency in Hertz to a 1V/octave voltage value.',
        params: [
            {
                name: 'frequency',
                type: 'number',
                description: 'Frequency in Hz',
            },
        ],
        returns: 'Voltage value usable as a Signal',
        examples: ['$hz(440)    // A4', '$hz(261.63) // ~C4'],
        group: 'Helpers',
    },
    {
        name: '$note',
        signature: '$note(noteName: string): number',
        description: 'Convert a note name string to a 1V/octave voltage value.',
        params: [
            {
                name: 'noteName',
                type: 'string',
                description: 'Note name like "C4", "A#3", "Bb5"',
            },
        ],
        returns: 'Voltage value usable as a Signal',
        examples: ['$note("C4")  // Middle C', '$note("A4")  // 440 Hz'],
        group: 'Helpers',
    },
    {
        name: '$c',
        signature:
            '$c(...args: (ModuleOutput | Iterable<ModuleOutput>)[]): Collection',
        description:
            'Create a Collection from one or more ModuleOutputs. Collections support chainable DSP methods, indexing, and spreading.',
        examples: [
            '$c(...$sine("C4"), ...$saw("E4")).amplitude(0.5).out()',
            '$c($noise("white"), $noise("pink"))[0].out()  // index access',
        ],
        group: 'Helpers',
    },
    {
        name: '$r',
        signature:
            '$r(...args: (ModuleOutputWithRange | Iterable<ModuleOutputWithRange>)[]): CollectionWithRange',
        description:
            'Create a CollectionWithRange from ranged outputs. The range() method uses stored min/max values.',
        examples: [
            '$r(...$sine("1hz"), ...$saw("2hz")).range(0, 5).out()',
        ],
        group: 'Helpers',
    },
    {
        name: '$cartesian',
        signature:
            '$cartesian<A extends unknown[][]>(...arrays: A): ElementsOf<A>[]',
        description:
            'Compute the Cartesian product of the given arrays. Returns every possible combination of one element from each array.',
        examples: [
            "$cartesian(['C3', 'C4'], [0, 2.5]).map(\n  ([note, shape]) => $saw(note, { shape })\n).forEach(s => s.out())",
        ],
        group: 'Helpers',
    },
    // ---- Global Settings ----
    {
        name: '$setTempo',
        signature: '$setTempo(tempo: number): void',
        description: 'Set the global tempo for the root clock.',
        params: [
            { name: 'tempo', type: 'number', description: 'Tempo in BPM' },
        ],
        examples: ['$setTempo(120)  // 120 BPM', '$setTempo(140)  // 140 BPM'],
        group: 'Global Settings',
    },
    {
        name: '$setTimeSignature',
        signature:
            '$setTimeSignature(numerator: number, denominator: number): void',
        description:
            'Set the time signature for the root clock. Both values must be positive integers.',
        params: [
            {
                name: 'numerator',
                type: 'number',
                description: 'Beats per bar (e.g. 3, 4, 6, 7)',
            },
            {
                name: 'denominator',
                type: 'number',
                description:
                    'Beat value (e.g. 4 for quarter note, 8 for eighth note)',
            },
        ],
        examples: [
            '$setTimeSignature(4, 4)  // 4/4 (default)',
            '$setTimeSignature(3, 4)  // 3/4 waltz',
            '$setTimeSignature(7, 8)  // 7/8 asymmetric',
        ],
        group: 'Global Settings',
    },
    {
        name: '$setOutputGain',
        signature: '$setOutputGain(gain: Mono<Signal>): void',
        description:
            'Set the global output gain applied to the final mix. 2.5 is the default (50%); 5.0 is unity gain.',
        params: [
            {
                name: 'gain',
                type: 'Mono<Signal>',
                description: 'Gain level (2.5 default, 5.0 unity)',
            },
        ],
        examples: [
            '$setOutputGain(2.5) // 50% gain (default)',
            '$setOutputGain(5.0) // unity gain',
            "$setOutputGain($sine('1hz')) // modulated gain",
        ],
        group: 'Global Settings',
    },
    // ---- Controls -----
    {
        name: '$slider',
        signature:
            '$slider(label: string, value: number, min: number, max: number): ModuleOutput',
        description:
            'Create a UI slider bound to a signal module. The slider appears in the Control panel. Dragging it updates the audio engine and the source code value in real time.',
        params: [
            {
                name: 'label',
                type: 'string',
                description: 'Display label (must be a string literal)',
            },
            {
                name: 'value',
                type: 'number',
                description: 'Initial value (must be a numeric literal)',
            },
            { name: 'min', type: 'number', description: 'Minimum value' },
            { name: 'max', type: 'number', description: 'Maximum value' },
        ],
        returns: 'ModuleOutput carrying the current slider value',
        examples: [
            'const vol = $slider("Volume", 0.5, 0, 1);\n$sine(440).amplitude(vol).out();',
            'const cutoff = $slider("Cutoff", 1000, 100, 8000);\n$saw(440).pipe(s => $lpf(s, cutoff)).out();',
        ],
        group: 'Controls',
    },
    // ---- Advanced ----
    {
        name: '$deferred',
        signature: '$deferred(channels?: number): DeferredCollection',
        description:
            'Create placeholder signals that can be assigned later. Useful for feedback loops.',
        params: [
            {
                name: 'channels',
                type: 'number',
                description: 'Number of deferred outputs (1-16, default 1)',
            },
        ],
        returns: 'DeferredCollection',
        examples: [
            'const fb = $deferred()\nconst sig = $slew(fb[0], { rise: 0.01, fall: 0.01 })\nfb.set(sig)\nsig.out()',
        ],
        group: 'Advanced',
    },

    {
        name: '$bus',
        signature: '$bus(cb: (mixed: Collection) => unknown): Bus',
        description:
            'Create a send-return bus. Signals are routed to the bus via `.send(bus, gain)` on any ModuleOutput or Collection. ' +
            'The callback receives a Collection that is the mix of all sends, allowing effects or further routing to be applied.',
        params: [
            {
                name: 'cb',
                type: '(mixed: Collection) => unknown',
                description:
                    'Called at patch finalization with the mixed sends. Call .out() or return a signal.',
            },
        ],
        returns: 'Bus handle passed to .send()',
        examples: [
            'const fx = $bus((mixed) => mixed.gain(0.8).out())\n$saw("C3").send(fx, 0.6)\n$sine("E3").send(fx, 0.4)',
        ],
        group: 'Advanced',
    },
    {
        name: '$setEndOfChainCb',
        signature:
            '$setEndOfChainCb(cb: (mixed: Collection) => ModuleOutput | Collection | CollectionWithRange): void',
        description:
            'Set a custom processor applied to the final mix just before the global output gain. ' +
            'The callback receives the fully mixed Collection and should return a processed signal.',
        params: [
            {
                name: 'cb',
                type: '(mixed: Collection) => ModuleOutput | Collection | CollectionWithRange',
                description: 'Transform applied to the final mix',
            },
        ],
        examples: [
            `$setEndOfChainCb((mix) => $lpf(mix, '2000hz'));`,
            '$setEndOfChainCb((mix) => mix.scope());',
        ],
        group: 'Advanced',
    },
];

/**
 * Check if a string is a known DSL type name.
 */
export function isDslType(name: string): name is DslTypeName {
    return DSL_TYPE_NAMES.includes(name as DslTypeName);
}

/**
 * Get documentation for a DSL type by name.
 */
export function getTypeDoc(name: string): TypeDocumentation | undefined {
    if (isDslType(name)) {
        return TYPE_DOCS[name];
    }
    return undefined;
}
