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
    'PolySignal',
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
            'sine("C4")           // Note string - converted to 1V/oct',
            'sine(440)            // Number - constant voltage',
            'sine("440hz")        // Hz string - converted to voltage',
            'sine("60m")          // MIDI note 60 (middle C)',
            'sine(lfo.out)        // ModuleOutput from another module',
            'sine("4s(C:major)")  // Scale pattern',
        ],
        seeAlso: [
            'PolySignal',
            'ModuleOutput',
            'Note',
            'HZ',
            'MidiNote',
            'Scale',
        ],
    },

    PolySignal: {
        name: 'PolySignal',
        description:
            'A potentially multi-channel signal. Can be an array of Signals for polyphonic patches, ' +
            'or an iterable of ModuleOutputs. When used as input to a module, arrays are expanded to create multiple voices.',
        definition: 'Signal | Signal[] | Iterable<ModuleOutput>',
        examples: [
            'filter.lpf(["C3", "E3", "G3"], { cutoff: 1000 })  // 3-voice chord',
            'osc.saw([...seq.pitch])                           // Spread sequencer outputs',
            'mix.add(osc1.out, osc2.out, osc3.out)             // Multiple ModuleOutputs',
        ],
        seeAlso: ['Signal', 'ModuleOutput', 'Collection'],
    },

    ModuleOutput: {
        name: 'ModuleOutput',
        description:
            'A single output from a module, representing a mono signal connection. ' +
            'ModuleOutputs are chainable - methods like gain(), shift(), and out() return the same output for fluent API usage. ' +
            'Every module factory returns either a ModuleOutput or a Collection of outputs.',
        definition:
            'interface { moduleId: string; portName: string; channel: number; ... }',
        examples: [
            'const osc = osc.sine("C4")',
            'osc.gain(0.5).out()           // Chain methods',
            'osc.scope().out()             // Add visualization',
            'filter.lpf(osc, { q: 4 })     // Use as input to another module',
        ],
        seeAlso: ['ModuleOutputWithRange', 'Collection', 'Signal'],
        methods: [
            {
                name: 'gain',
                signature: 'gain(factor: PolySignal): ModuleOutput',
                description:
                    'Scale the signal by a factor. Creates a util.scaleAndShift module internally.',
                example: 'osc.gain(0.5)  // Half amplitude',
            },
            {
                name: 'shift',
                signature: 'shift(offset: PolySignal): ModuleOutput',
                description:
                    'Add a DC offset to the signal. Creates a util.scaleAndShift module internally.',
                example: 'lfo.shift(2.5)  // Shift LFO to 0-5V range',
            },
            {
                name: 'scope',
                signature:
                    'scope(config?: { msPerFrame?: number; triggerThreshold?: number; scale?: number }): this',
                description:
                    'Add an oscilloscope visualization for this output. The scope appears as an overlay in the editor.',
                example: 'osc.scope({ msPerFrame: 100, scale: 5 }).out()',
            },
            {
                name: 'out',
                signature:
                    'out(baseChannel?: number, options?: StereoOutOptions): this',
                description:
                    'Send this output to the speakers as stereo audio. Left plays on baseChannel, right on baseChannel+1.',
                example: 'osc.out(0, { gain: 0.5, pan: -2 })',
            },
            {
                name: 'outMono',
                signature: 'outMono(channel?: number, gain?: PolySignal): this',
                description:
                    'Send this output to a single speaker channel as mono audio.',
                example: 'lfo.outMono(2, 0.3)',
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
            'const lfo = lfo.sine(2)              // LFO outputs -5 to +5',
            'lfo.range(200, 2000)                 // Remap to 200-2000 for filter cutoff',
            'env.adsr({ attack: 0.1 }).range(0, 1)  // Envelope 0-1 range',
        ],
        seeAlso: ['ModuleOutput', 'CollectionWithRange'],
        methods: [
            {
                name: 'range',
                signature:
                    'range(outMin: PolySignal, outMax: PolySignal): ModuleOutput',
                description:
                    'Remap the output from its native range (minValue, maxValue) to a new range (outMin, outMax). ' +
                    'Unlike Collection.range(), this uses the stored min/max values automatically.',
                example:
                    'lfo.range(note("C3"), note("C5"))  // Remap LFO to pitch range',
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
            '$(osc1, osc2, osc3).gain(0.5).out()  // Apply gain to all, send to output',
            'const voices = $(osc1, osc2, osc3)',
            'for (const v of voices) { ... }      // Iterate over outputs',
            '[...voices]                          // Spread to array',
            'voices[0]                            // Index access',
        ],
        seeAlso: ['CollectionWithRange', 'ModuleOutput', 'PolySignal'],
        methods: [
            {
                name: 'gain',
                signature: 'gain(factor: PolySignal): Collection',
                description: 'Scale all signals in the collection by a factor.',
                example: '$(osc1, osc2).gain(0.5)',
            },
            {
                name: 'shift',
                signature: 'shift(offset: PolySignal): Collection',
                description:
                    'Add a DC offset to all signals in the collection.',
                example: '$(lfo1, lfo2).shift(2.5)',
            },
            {
                name: 'scope',
                signature:
                    'scope(config?: { msPerFrame?: number; triggerThreshold?: number; scale?: number }): this',
                description:
                    'Add scope visualization for the first output in the collection.',
                example: '$(osc1, osc2).scope().out()',
            },
            {
                name: 'out',
                signature:
                    'out(baseChannel?: number, options?: StereoOutOptions): this',
                description:
                    'Send all outputs to speakers as stereo, summed together.',
                example: '$(osc1, osc2, osc3).out()',
            },
            {
                name: 'outMono',
                signature: 'outMono(channel?: number, gain?: PolySignal): this',
                description:
                    'Send all outputs to a single speaker channel as mono, summed together.',
                example: '$(osc1, osc2).outMono(0, 0.3)',
            },
            {
                name: 'range',
                signature:
                    'range(inMin: PolySignal, inMax: PolySignal, outMin: PolySignal, outMax: PolySignal): Collection',
                description:
                    'Remap all outputs from input range to output range. Requires explicit input min/max.',
                example: '$(lfo1, lfo2).range(-5, 5, 0, 1)',
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
            '$r(lfo1, lfo2).range(0, 5).out()     // Remap using stored ranges',
            '$r(...seq.gates).range(0, 1)        // Spread and remap gates',
        ],
        seeAlso: ['Collection', 'ModuleOutputWithRange'],
        methods: [
            {
                name: 'range',
                signature:
                    'range(outMin: PolySignal, outMax: PolySignal): Collection',
                description:
                    'Remap all outputs from their native ranges to a new range. ' +
                    "Uses each output's stored minValue/maxValue.",
                example: '$r(lfo1, lfo2).range(200, 2000)',
            },
        ],
    },

    Note: {
        name: 'Note',
        description:
            'A musical note string in scientific pitch notation. ' +
            'Consists of a note name (A-G), optional accidental (#/b), and optional octave number. ' +
            'If octave is omitted, defaults to octave 4.',
        definition: '`${NoteName}${Accidental}${Octave}`',
        examples: [
            '"C4"   // Middle C',
            '"A#3"  // A sharp in octave 3',
            '"Bb5"  // B flat in octave 5',
            '"G"    // G4 (octave 4 is default)',
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
            'Controls gain, panning, and stereo width.',
        definition:
            'interface { gain?: PolySignal; pan?: PolySignal; width?: Signal }',
        examples: [
            'osc.out(0, { gain: 0.5 })           // 50% gain',
            'osc.out(0, { pan: -2.5 })           // Pan left',
            'osc.out(0, { width: 5 })            // Full stereo spread',
            'osc.out(0, { gain: env.out, pan: lfo.out })  // Modulated',
        ],
        seeAlso: ['ModuleOutput', 'Collection', 'PolySignal'],
    },
};

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
