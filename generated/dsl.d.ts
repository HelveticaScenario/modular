declare global {

/** The **`console`** object provides access to the debugging console (e.g., the Web console in Firefox). */
/**
 * The **`console`** object provides access to the debugging console (e.g., the Web console in Firefox).
 *
 * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console)
 */
interface Console {
    /**
     * The **`console.assert()`** static method writes an error message to the console if the assertion is false.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/assert_static)
     */
    assert(condition?: boolean, ...data: any[]): void;
    /**
     * The **`console.clear()`** static method clears the console if possible.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/clear_static)
     */
    clear(): void;
    /**
     * The **`console.count()`** static method logs the number of times that this particular call to `count()` has been called.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/count_static)
     */
    count(label?: string): void;
    /**
     * The **`console.countReset()`** static method resets counter used with console/count_static.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/countReset_static)
     */
    countReset(label?: string): void;
    /**
     * The **`console.debug()`** static method outputs a message to the console at the 'debug' log level.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/debug_static)
     */
    debug(...data: any[]): void;
    /**
     * The **`console.dir()`** static method displays a list of the properties of the specified JavaScript object.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/dir_static)
     */
    dir(item?: any, options?: any): void;
    /**
     * The **`console.dirxml()`** static method displays an interactive tree of the descendant elements of the specified XML/HTML element.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/dirxml_static)
     */
    dirxml(...data: any[]): void;
    /**
     * The **`console.error()`** static method outputs a message to the console at the 'error' log level.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/error_static)
     */
    error(...data: any[]): void;
    /**
     * The **`console.group()`** static method creates a new inline group in the Web console log, causing any subsequent console messages to be indented by an additional level, until console/groupEnd_static is called.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/group_static)
     */
    group(...data: any[]): void;
    /**
     * The **`console.groupCollapsed()`** static method creates a new inline group in the console.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/groupCollapsed_static)
     */
    groupCollapsed(...data: any[]): void;
    /**
     * The **`console.groupEnd()`** static method exits the current inline group in the console.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/groupEnd_static)
     */
    groupEnd(): void;
    /**
     * The **`console.info()`** static method outputs a message to the console at the 'info' log level.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/info_static)
     */
    info(...data: any[]): void;
    /**
     * The **`console.log()`** static method outputs a message to the console.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/log_static)
     */
    log(...data: any[]): void;
    /**
     * The **`console.table()`** static method displays tabular data as a table.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/table_static)
     */
    table(tabularData?: any, properties?: string[]): void;
    /**
     * The **`console.time()`** static method starts a timer you can use to track how long an operation takes.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/time_static)
     */
    time(label?: string): void;
    /**
     * The **`console.timeEnd()`** static method stops a timer that was previously started by calling console/time_static.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/timeEnd_static)
     */
    timeEnd(label?: string): void;
    /**
     * The **`console.timeLog()`** static method logs the current value of a timer that was previously started by calling console/time_static.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/timeLog_static)
     */
    timeLog(label?: string, ...data: any[]): void;
    timeStamp(label?: string): void;
    /**
     * The **`console.trace()`** static method outputs a stack trace to the console.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/trace_static)
     */
    trace(...data: any[]): void;
    /**
     * The **`console.warn()`** static method outputs a warning message to the console at the 'warning' log level.
     *
     * [MDN Reference](https://developer.mozilla.org/docs/Web/API/console/warn_static)
     */
    warn(...data: any[]): void;
}

var console: Console;
type NoteNames = "a" | "A" | "b" | "B" | "c" | "C" | "d" | "D" | "e" | "E" | "f" | "F" | "g" | "G"
type Accidental = "" | "#" | "b"
type Note = `${NoteNames}${Accidental}${number | ''}`

type HZ = `${number}hz` | `${number}Hz`

type MidiNote = `${number}m`

type CaseVariants<T extends string> = 
  | Lowercase<T>
  | Uppercase<T>
  | Capitalize<T>;

type ModeString =
  // Ionian (Major)
  | `M ${string}`
  | "M"
  | `${string}${CaseVariants<"maj">}${string}`
  | `${string}${CaseVariants<"major">}${string}`
  | `${string}${CaseVariants<"ionian">}${string}`
  
  // Harmonic Minor
  | `${string}${CaseVariants<"har">} ${CaseVariants<"minor">}${string}`
  | `${string}${CaseVariants<"harmonic">}${CaseVariants<"minor">}${string}`
  | `${string}${CaseVariants<"harmonic">} ${CaseVariants<"minor">}${string}`
  
  // Melodic Minor
  | `${string}${CaseVariants<"mel">} ${CaseVariants<"minor">}${string}`
  | `${string}${CaseVariants<"melodic">}${CaseVariants<"minor">}${string}`
  | `${string}${CaseVariants<"melodic">} ${CaseVariants<"minor">}${string}`
  
  // Pentatonic Major
  | `${string}${CaseVariants<"pentatonic">} ${CaseVariants<"major">}${string}`
  | `${string}${CaseVariants<"pentatonic">} ${CaseVariants<"maj">}${string}`
  | `${string}${CaseVariants<"pent">} ${CaseVariants<"maj">}${string}`
  | `${string}${CaseVariants<"pent">} ${CaseVariants<"major">}${string}`
  
  // Pentatonic Minor
  | `${string}${CaseVariants<"pentatonic">} ${CaseVariants<"minor">}${string}`
  | `${string}${CaseVariants<"pentatonic">} ${CaseVariants<"min">}${string}`
  | `${string}${CaseVariants<"pent">} ${CaseVariants<"min">}${string}`
  | `${string}${CaseVariants<"pent">} ${CaseVariants<"minor">}${string}`
  
  // Blues
  | `${string}${CaseVariants<"blues">}${string}`
  
  // Chromatic
  | `${string}${CaseVariants<"chromatic">}${string}`
  
  // Whole Tone
  | `${string}${CaseVariants<"whole">} ${CaseVariants<"tone">}${string}`
  | `${string}${CaseVariants<"whole">}${CaseVariants<"tone">}${string}`
  
  // Aeolian (Minor)
  | `m ${string}`
  | "m"
  | `${string}${CaseVariants<"min">}${string}`
  | `${string}${CaseVariants<"minor">}${string}`
  | `${string}${CaseVariants<"aeolian">}${string}`
  
  // Dorian (start of string)
  | `${CaseVariants<"dorian">}${string}`
  
  // Locrian (start of string)
  | `${CaseVariants<"locrian">}${string}`
  
  // Mixolydian (start of string)
  | `${CaseVariants<"mixolydian">}${string}`
  
  // Phrygian (start of string)
  | `${CaseVariants<"phrygian">}${string}`
  
  // Lydian (start of string)
  | `${CaseVariants<"lydian">}${string}`;

/**
 * A scale pattern string for generating multiple pitches.
 * Format: "{count}s({root}:{mode})"
 * @example "4s(C:major)" - 4 notes of C major scale
 * @example "8s(A:minor)" - 8 notes of A minor scale
 * @see {@link Signal}
 * @see {@link Note}
 */
type Scale = `${number}s(${Note}:${ModeString})`

type OrArray<T> = T | T[];

/**
 * A single-channel audio signal value. The fundamental type for all audio connections.
 * 
 * Signals follow the 1V/octave convention where 0V = C4 (~261.63 Hz).
 * 
 * Can be one of:
 * - A **number** (constant voltage)
 * - A **{@link Note}** string like `"C4"` or `"A#3"`
 * - A **{@link HZ}** string like `"440hz"`
 * - A **{@link MidiNote}** string like `"60m"`
 * - A **{@link Scale}** pattern like `"4s(C:major)"`
 * - A **{@link ModuleOutput}** from another module
 * 
 * @example sine("C4")        // Note string
 * @example sine(440)         // Number
 * @example sine("440hz")     // Hz string
 * @example sine(lfo.out)     // ModuleOutput
 * @see {@link Poly<Signal>} - for multi-channel signals
 * @see {@link ModuleOutput} - for module connections
 */
type Signal = number | Note | HZ | MidiNote | Scale | ModuleOutput;

/**
 * A potentially multi-channel signal for polyphonic patches.
 * 
 * Can be:
 * - A single {@link Signal}
 * - An array of {@link Signal}s (creates multiple voices)
 * - An iterable of {@link ModuleOutput}s
 * 
 * @example filter.lpf(["C3", "E3", "G3"]) // 3-voice chord
 * @example osc.saw([...seq.pitch])        // Spread sequencer outputs
 * @see {@link Signal} - for single-channel signals
 * @see {@link Collection} - for grouping outputs
 */
type Poly<T extends Signal = Signal> = OrArray<T> | Iterable<ModuleOutput>;


type Mono<T extends Signal = Signal> = OrArray<T> | Iterable<ModuleOutput>;

/**
 * Options for stereo output routing via the out() method.
 * @see {@link ModuleOutput.out}
 * @see {@link Collection.out}
 */
interface StereoOutOptions {
  /** Output gain. If set, a util.scaleAndShift module is added after the stereo mix */
  gain?: Poly<Signal>;
  /** Pan position (-5 = left, 0 = center, +5 = right). Default 0 */
  pan?: Poly<Signal>;
  /** Stereo width/spread (0 = no spread, 5 = full spread). Default 0 */
  width?: Signal;
}

/**
 * A single output from a module, representing a mono signal connection.
 * 
 * ModuleOutputs are chainable - methods like gain(), shift(), and out() 
 * return the same output for fluent API usage.
 * 
 * @example
 * const osc = osc.sine("C4")
 * osc.gain(0.5).out()           // Chain methods
 * osc.scope().out()             // Add visualization
 * filter.lpf(osc, { q: 4 })     // Use as input
 * 
 * @see {@link ModuleOutputWithRange} - for outputs with known value ranges
 * @see {@link Collection} - for grouping multiple outputs
 * @see {@link Signal} - ModuleOutput is a valid Signal
 */
interface ModuleOutput {
  /** The unique identifier of the module this output belongs to */
  readonly moduleId: string;
  /** The name of the output port */
  readonly portName: string;
  /** The channel index for polyphonic outputs */
  readonly channel: number;
  
  /**
   * Scale the signal by a factor. Creates a util.scaleAndShift module internally.
   * @param factor - Scale factor as {@link Poly<Signal>}
   * @returns The scaled {@link ModuleOutput} for chaining
   * @example osc.gain(0.5)  // Half amplitude
   */
  gain(factor: Poly<Signal>): ModuleOutput;
  
  /**
   * Add a DC offset to the signal. Creates a util.scaleAndShift module internally.
   * @param offset - Offset value as {@link Poly<Signal>}
   * @returns The shifted {@link ModuleOutput} for chaining
   * @example lfo.shift(2.5)  // Shift to 0-5V range
   */
  shift(offset: Poly<Signal>): ModuleOutput;
  
  /**
   * Add scope visualization for this output.
   * The scope appears as an overlay in the editor.
   * @param config - Scope configuration options
   * @param config.msPerFrame - Time window in milliseconds (default 500)
   * @param config.triggerThreshold - Trigger threshold in volts (optional)
   * @param config.range - Voltage range for display as [min, max] tuple (default [-5, 5])
   * @example osc.scope({ msPerFrame: 100, range: [-10, 10] }).out()
   */
  scope(config?: { msPerFrame?: number; triggerThreshold?: number; range?: [number, number] }): this;
  
  /**
   * Send this output to speakers as stereo.
   * @param baseChannel - Base output channel (0-15, default 0). Left plays on baseChannel, right on baseChannel+1
   * @param options - Stereo output options ({@link StereoOutOptions})
   * @example osc.out(0, { gain: 0.5, pan: -2 })
   */
  out(baseChannel?: number, options?: StereoOutOptions): this;
  
  /**
   * Send this output to speakers as mono.
   * @param channel - Output channel (0-15, default 0)
   * @param gain - Output gain as {@link Poly<Signal>} (optional)
   * @example lfo.outMono(2, 0.3)
   */
  outMono(channel?: number, gain?: Poly<Signal>): this;
}

/**
 * A {@link ModuleOutput} that knows its output value range (minValue, maxValue).
 * 
 * Typically returned by LFOs, envelopes, and other modulation sources.
 * The range() method uses the stored min/max for automatic scaling.
 * 
 * @example
 * const lfo = lfo.sine(2)              // Outputs -5 to +5
 * lfo.range(200, 2000)                 // Remap to 200-2000
 * env.adsr({ attack: 0.1 }).range(0, 1)
 * 
 * @see {@link ModuleOutput} - base interface
 * @see {@link CollectionWithRange} - for collections of ranged outputs
 */
interface ModuleOutputWithRange extends ModuleOutput {
  /** The minimum value this output produces */
  readonly minValue: number;
  /** The maximum value this output produces */
  readonly maxValue: number;
  
  /**
   * Remap the output from its native range to a new range.
   * Uses the stored minValue/maxValue automatically.
   * @param outMin - New minimum as {@link Poly<Signal>}
   * @param outMax - New maximum as {@link Poly<Signal>}
   * @returns A {@link ModuleOutput} with the remapped signal
   * @example lfo.range(note("C3"), note("C5"))
   */
  range(outMin: Poly<Signal>, outMax: Poly<Signal>): ModuleOutput;
}

/**
 * A collection of {@link ModuleOutput} instances with chainable DSP methods.
 * 
 * Created with the $() helper function. Supports iteration, indexing, and spreading.
 * Methods operate on all outputs in the collection.
 * 
 * @example
 * $(osc1, osc2, osc3).gain(0.5).out()  // Apply gain to all
 * for (const v of voices) { ... }      // Iterate
 * [...voices]                          // Spread to array
 * voices[0]                            // Index access
 * 
 * @see {@link CollectionWithRange} - for ranged outputs
 * @see {@link ModuleOutput} - individual outputs
 * @see {@link $} - helper to create Collection
 */
interface Collection extends Iterable<ModuleOutput> {
  /** Number of outputs in the collection */
  readonly length: number;
  /** Index access to individual {@link ModuleOutput}s */
  readonly [index: number]: ModuleOutput;
  [Symbol.iterator](): Iterator<ModuleOutput>;
  
  /**
   * Scale all signals by a factor.
   * @param factor - Scale factor as {@link Poly<Signal>}
   * @see {@link ModuleOutput.gain}
   */
  gain(factor: Poly<Signal>): Collection;
  
  /**
   * Add DC offset to all signals.
   * @param offset - Offset as {@link Poly<Signal>}
   * @see {@link ModuleOutput.shift}
   */
  shift(offset: Poly<Signal>): Collection;
  
  /**
   * Add scope visualization for the first output in the collection.
   * @param config - Scope configuration options
   * @param config.msPerFrame - Time window in milliseconds (default 500)
   * @param config.triggerThreshold - Trigger threshold in volts (optional)
   * @param config.range - Voltage range for display as [min, max] tuple (default [-5, 5])
   */
  scope(config?: { msPerFrame?: number; triggerThreshold?: number; range?: [number, number] }): this;
  
  /**
   * Send all outputs to speakers as stereo, summed together.
   * @param baseChannel - Base output channel (0-14, default 0)
   * @param options - Stereo output options ({@link StereoOutOptions})
   */
  out(baseChannel?: number, options?: StereoOutOptions): this;
  
  /**
   * Send all outputs to speakers as mono, summed together.
   * @param channel - Output channel (0-15, default 0)
   * @param gain - Output gain as {@link Poly<Signal>} (optional)
   */
  outMono(channel?: number, gain?: Poly<Signal>): this;
  
  /**
   * Remap all outputs from input range to output range.
   * Requires explicit input min/max values.
   * @param inMin - Input minimum as {@link Poly<Signal>}
   * @param inMax - Input maximum as {@link Poly<Signal>}
   * @param outMin - Output minimum as {@link Poly<Signal>}
   * @param outMax - Output maximum as {@link Poly<Signal>}
   * @see {@link CollectionWithRange.range} - for automatic input range
   */
  range(inMin: Poly<Signal>, inMax: Poly<Signal>, outMin: Poly<Signal>, outMax: Poly<Signal>): Collection;
}

/**
 * A collection of {@link ModuleOutputWithRange} instances.
 * 
 * Created with the $r() helper function. Like {@link Collection}, but the 
 * range() method uses stored min/max values from each output.
 * 
 * @example
 * $r(lfo1, lfo2).range(0, 5).out()     // Remap using stored ranges
 * $r(...seq.gates).range(0, 1)        // Spread and remap gates
 * 
 * @see {@link Collection} - for outputs without known ranges
 * @see {@link ModuleOutputWithRange} - individual ranged outputs
 * @see {@link $r} - helper to create CollectionWithRange
 */
interface CollectionWithRange extends Iterable<ModuleOutputWithRange> {
  /** Number of outputs in the collection */
  readonly length: number;
  /** Index access to individual {@link ModuleOutputWithRange}s */
  readonly [index: number]: ModuleOutputWithRange;
  [Symbol.iterator](): Iterator<ModuleOutputWithRange>;
  
  /**
   * Scale all signals by a factor.
   * @param factor - Scale factor as {@link Poly<Signal>}
   */
  gain(factor: Poly<Signal>): Collection;
  
  /**
   * Add DC offset to all signals.
   * @param offset - Offset as {@link Poly<Signal>}
   */
  shift(offset: Poly<Signal>): Collection;
  
  /**
   * Add scope visualization for the first output in the collection.
   * @param config - Scope configuration options
   * @param config.msPerFrame - Time window in milliseconds (default 500)
   * @param config.triggerThreshold - Trigger threshold in volts (optional)
   * @param config.range - Voltage range for display as [min, max] tuple (default [-5, 5])
   */
  scope(config?: { msPerFrame?: number; triggerThreshold?: number; range?: [number, number] }): this;
  
  /**
   * Send all outputs to speakers as stereo, summed together.
   * @param baseChannel - Base output channel (0-14, default 0)
   * @param options - Stereo output options ({@link StereoOutOptions})
   */
  out(baseChannel?: number, options?: StereoOutOptions): this;
  
  /**
   * Send all outputs to speakers as mono, summed together.
   * @param channel - Output channel (0-15, default 0)
   * @param gain - Output gain as {@link Poly<Signal>} (optional)
   */
  outMono(channel?: number, gain?: Poly<Signal>): this;
  
  /**
   * Remap all outputs from their native ranges to a new range.
   * Uses each output's stored minValue/maxValue.
   * @param outMin - Output minimum as {@link Poly<Signal>}
   * @param outMax - Output maximum as {@link Poly<Signal>}
   * @see {@link Collection.range} - for explicit input range
   */
  range(outMin: Poly<Signal>, outMax: Poly<Signal>): Collection;
}

// Helper functions exposed by the DSL runtime

/**
 * Convert a frequency in Hertz to a voltage value (1V/octave).
 * @param frequency - Frequency in Hz
 * @returns Voltage value for use as a {@link Signal}
 * @example hz(440)  // A4
 * @example hz(261.63)  // ~C4
 */
function hz(frequency: number): number;

/**
 * Convert a note name string to a voltage value (1V/octave).
 * @param noteName - Note name like "C4", "A#3", "Bb5"
 * @returns Voltage value for use as a {@link Signal}
 * @example note("C4")  // Middle C
 * @example note("A4")  // 440 Hz
 */
function note(noteName: string): number;

/**
 * Convert beats per minute to a frequency in Hz.
 * Useful for setting clock tempo.
 * @param beatsPerMinute - Tempo in BPM
 * @returns Frequency in Hz
 * @example bpm(120)  // 2 Hz
 * @see {@link setTempo}
 */
function bpm(beatsPerMinute: number): number;

/**
 * Create a {@link Collection} from {@link ModuleOutput} instances.
 * 
 * Collections support chainable DSP methods, iteration, indexing, and spreading.
 * @param args - One or more {@link ModuleOutput}s to group
 * @returns A {@link Collection} of the outputs
 * @example $c(osc1, osc2).gain(0.5).out()
 * @example $c(osc1, osc2, osc3)[0]  // Index access
 * @example [...$c(osc1, osc2)]      // Spread to array
 * @see {@link $r} - for ranged outputs
 */
function $c(...args: ModuleOutput[]): Collection;

/**
 * Create a {@link CollectionWithRange} from {@link ModuleOutputWithRange} instances.
 * 
 * Like $() but the range() method uses stored min/max values.
 * @param args - One or more {@link ModuleOutputWithRange}s to group
 * @returns A {@link CollectionWithRange} of the outputs
 * @example $r(lfo1, lfo2).range(0, 5)  // Uses stored ranges
 * @example $r(...seq.gates).range(0, 1)
 * @see {@link $} - for outputs without known ranges
 */
function $r(...args: ModuleOutputWithRange[]): CollectionWithRange;

/**
 * Set the global tempo for the root clock.
 * @param tempo - Tempo as a Signal (use bpm() helper, e.g., setTempo(bpm(140)))
 * @example setTempo(bpm(120)) // 120 beats per minute
 * @example setTempo(hz(2)) // 2 Hz = 120 BPM
 * @example setTempo(lfo.sine) // modulate tempo from LFO
 */
function setTempo(tempo: Signal): void;

/**
 * Set the global output gain applied to the final mix.
 * @param gain - Gain as a Signal (2.5 is default, 5.0 is unity gain)
 * @example setOutputGain(2.5) // 50% gain (default)
 * @example setOutputGain(5.0) // unity gain
 * @example setOutputGain(env.out) // modulate gain from envelope
 */
function setOutputGain(gain: Signal): void;

/**
 * DeferredModuleOutput is a placeholder for a signal that will be assigned later.
 * Useful for feedback loops and forward references in the DSL.
 * Supports the same chainable methods as ModuleOutput (gain, shift, scope, out, outMono).
 */
interface DeferredModuleOutput extends ModuleOutput {
  /**
   * Set the actual signal this deferred output should resolve to.
   * @param signal - The signal to resolve to (number, string, or ModuleOutput)
   */
  set(signal: Signal): void;
  /**
   * Scale the resolved output by a factor.
   * The transform is stored and applied during resolution.
   */
  gain(factor: Poly<Signal>): Collection;
  /**
   * Shift the resolved output by an offset.
   * The transform is stored and applied during resolution.
   */
  shift(offset: Poly<Signal>): Collection;
  /**
   * Add scope visualization for the resolved output.
   * The side effect is stored and executed during resolution.
   */
  scope(config?: { msPerFrame?: number; triggerThreshold?: number; range?: [number, number] }): this;
  /**
   * Send the resolved output to speakers as stereo.
   * The side effect is stored and executed during resolution.
   */
  out(baseChannel?: number, options?: StereoOutOptions): this;
  /**
   * Send the resolved output to speakers as mono.
   * The side effect is stored and executed during resolution.
   */
  outMono(channel?: number, gain?: Poly<Signal>): this;
}

/**
 * DeferredCollection is a collection of DeferredModuleOutput instances.
 * Provides a .set() method to assign signals to all contained deferred outputs.
 */
interface DeferredCollection extends Iterable<DeferredModuleOutput> {
  readonly length: number;
  readonly [index: number]: DeferredModuleOutput;
  [Symbol.iterator](): Iterator<DeferredModuleOutput>;
  /**
   * Set the signals for all deferred outputs in this collection.
   * @param polySignal - A Poly<Signal> (single signal, array, or iterable) to distribute across outputs
   */
  set(polySignal: Poly<Signal>): void;
  /**
   * Scale all resolved outputs by a factor.
   */
  gain(factor: Poly<Signal>): Collection;
  /**
   * Shift all resolved outputs by an offset.
   */
  shift(offset: Poly<Signal>): Collection;
  /**
   * Add scope visualization for the first resolved output.
   */
  scope(config?: { msPerFrame?: number; triggerThreshold?: number; range?: [number, number] }): this;
  /**
   * Send all resolved outputs to speakers as stereo.
   */
  out(baseChannel?: number, options?: StereoOutOptions): this;
  /**
   * Send all resolved outputs to speakers as mono.
   */
  outMono(channel?: number, gain?: Poly<Signal>): this;
}

/**
 * Create a DeferredCollection with placeholder signals that can be assigned later.
 * Useful for feedback loops and forward references.
 * @param channels - Number of deferred outputs (1-16, default 1)
 * @example
 * const feedback = deferred();
 * const delayed = delay(osc.out, feedback[0]);
 * feedback.set(delayed);
 */
function deferred(channels?: number): DeferredCollection;

/**
 * Create a slider control that binds a UI slider to a signal module.
 *
 * The slider appears in the Control panel and allows real-time parameter adjustment.
 * Dragging the slider updates both the audio engine and the source code value.
 *
 * @param label - Display label for the slider (must be a string literal)
 * @param value - Initial value (must be a numeric literal)
 * @param min - Minimum slider value
 * @param max - Maximum slider value
 * @returns A ModuleOutput carrying the slider's current value as a signal
 *
 * @example
 * const vol = slider("Volume", 0.5, 0, 1);
 * sine(440).gain(vol).out();
 *
 * @example
 * const freq = slider("Frequency", 440, 20, 20000);
 * sine(freq).out();
 */
function slider(label: string, value: number, min: number, max: number): ModuleOutput;


namespace $ {
/**
 * a polyphonic signal passthrough
 * @param source - signal input (polyphonic)
 * @param config - Configuration object
 */
export function signal(source: Poly<Signal>, config?: { id?: string }): Collection;

/**
 * Mix multiple polyphonic signals together
 * @param inputs - Polyphonic inputs to mix together
 * @param config - Configuration object
 *   - gain - Output gain/attenuation (polyphonic - can extend output channels)
 *   - mode - Mixing mode (applied per-channel across all inputs)
 */
export function mix(inputs: Poly<Signal>[], config?: { gain?: Poly<Signal>; mode?: "sum" | "average" | "max" | "min"; id?: string }): Collection;

/**
 * Mix polyphonic signal to stereo
 * @param input - Polyphonic input signal to mix down to stereo
 * @param config - Configuration object
 *   - pan - Pan position for each channel (-5 = left, 0 = center, +5 = right).
 *   - width - Stereo width (0 = no spread, 5 = full spread across voices).
 */
export function stereoMix(input: Poly<Signal>, config?: { pan?: Poly<Signal>; width?: Mono<Signal>; id?: string }): Collection;

/**
 * Output type for clock module.
 * Extends Collection (default output: playhead)
 */
export interface ClockOutputs extends Collection {
  /** trigger output every bar */
  readonly barTrigger: ModuleOutputWithRange;
  /** ramp from 0 to 5V every bar */
  readonly ramp: ModuleOutputWithRange;
  /** trigger output at 48 PPQ */
  readonly ppqTrigger: ModuleOutputWithRange;
}

/**
 * A tempo clock with multiple outputs
 * @param tempo - tempo in v/oct (tempo)
 * @param config - Configuration object
 */
export function clock(tempo?: Mono<Signal>, config?: { id?: string }): ClockOutputs;

/**
 * Wavefolder effect adapted from 4ms Ensemble Oscillator
 * @param input - input signal to fold (bipolar, typically -5 to 5)
 * @param amount - fold amount (0-5, where 0 = bypass, 5 = maximum folding)
 * @param config - Configuration object
 *   - freq - frequency in v/oct (optional, enables anti-aliasing when connected)
 */
export function fold(input: Poly<Signal>, amount?: Poly<Signal>, config?: { freq?: Poly<Signal>; id?: string }): CollectionWithRange;

/**
 * Chebyshev waveshaper adapted from 4ms Ensemble Oscillator
 * @param input - input signal to shape (bipolar, typically -5 to 5)
 * @param amount - harmonic order amount (0-5, where 0 = fundamental only, 5 = 16th harmonic)
 * @param config - Configuration object
 *   - freq - frequency in v/oct (optional, enables anti-aliasing when connected)
 */
export function cheby(input: Poly<Signal>, amount?: Poly<Signal>, config?: { freq?: Poly<Signal>; id?: string }): CollectionWithRange;

/**
 * Triangle segment morpher adapted from 4ms Ensemble Oscillator
 * @param input - input signal to shape (bipolar, typically -5 to 5)
 * @param amount - segment shape amount (0-5, morphs between 8 shapes)
 * @param config - Configuration object
 *   - freq - frequency in v/oct (optional, enables anti-aliasing when connected)
 */
export function segment(input: Poly<Signal>, amount?: Poly<Signal>, config?: { freq?: Poly<Signal>; id?: string }): CollectionWithRange;

/**
 * A sine wave oscillator
 * @param freq - frequency in v/oct
 * @param config - Configuration object
 */
export function sine(freq: Poly<Signal>, config?: { id?: string }): CollectionWithRange;

/**
 * Sawtooth/Triangle/Ramp oscillator
 * @param freq - frequency in v/oct
 * @param config - Configuration object
 *   - shape - waveform shape: 0=saw, 2.5=triangle, 5=ramp
 */
export function saw(freq: Poly<Signal>, config?: { shape?: Poly<Signal>; id?: string }): CollectionWithRange;

/**
 * Pulse/Square oscillator with PWM
 * @param freq - frequency in v/oct
 * @param config - Configuration object
 *   - pwm - pulse width modulation input
 *   - width - pulse width (0-5, 2.5 is square)
 */
export function pulse(freq: Poly<Signal>, config?: { pwm?: Poly<Signal>; width?: Poly<Signal>; id?: string }): CollectionWithRange;

/**
 * A phase-driven sine wave oscillator
 * @param phase - phase input (0-1, will be wrapped)
 * @param config - Configuration object
 */
export function dSine(phase: Poly<Signal>, config?: { id?: string }): CollectionWithRange;

/**
 * A phase-driven sawtooth/triangle/ramp oscillator
 * @param phase - phase input (0-1, will be wrapped)
 * @param config - Configuration object
 *   - shape - waveform shape: 0=saw, 2.5=triangle, 5=ramp
 */
export function dSaw(phase: Poly<Signal>, config?: { shape?: Poly<Signal>; id?: string }): CollectionWithRange;

/**
 * A phase-driven pulse/square oscillator with PWM
 * @param phase - phase input (0-1, will be wrapped)
 * @param config - Configuration object
 *   - pwm - pulse width modulation input
 *   - width - pulse width (0-5, 2.5 is square)
 */
export function dPulse(phase: Poly<Signal>, config?: { pwm?: Poly<Signal>; width?: Poly<Signal>; id?: string }): CollectionWithRange;

/**
 * Noise generator with selectable color
 * @param color - color of the noise: white, pink, brown
 * @param config - Configuration object
 */
export function noise(color: "white" | "pink" | "brown", config?: { id?: string }): ModuleOutputWithRange;

/**
 * Output type for macro module.
 * Extends Collection (default output: output)
 */
export interface MacroOutputs extends Collection {
  /** auxiliary output - varies per engine */
  readonly aux: Collection;
}

/**
 * Mutable Instruments Plaits - Full macro-oscillator with 24 engines, LPG, and modulation
 * @param freq - Pitch input in V/Oct (0V = C4)
 * @param engine - Synthesis engine selection
 * @param config - Configuration object
 *   - fm - FM input (-5V to +5V) - frequency modulation
 *   - fmAmt - FM CV attenuverter (-5 to 5) - scales frequency modulation
 *   - harmonics - Harmonics parameter (0-5V) - function varies per engine
 *   - level - Level/dynamics input (0-5V) - controls VCA/LPG
 *   - lpgColor - LPG color (0-5V) - lowpass gate filter response (low = mellow, high = bright)
 *   - lpgDecay - LPG decay (0-5V) - lowpass gate envelope decay time
 *   - morph - Morph parameter (0-5V) - function varies per engine
 *   - morphAmt - Morph CV attenuverter (-5 to 5) - scales morph modulation
 *   - timbre - Timbre parameter (0-5V) - function varies per engine
 *   - timbreAmt - Timbre CV attenuverter (-5 to 5) - scales timbre modulation
 *   - trigger - Trigger input - gates/triggers the internal envelope
 */
export function macro(freq: Poly<Signal>, engine: "vaVcf" | "phaseDistortion" | "sixOpA" | "sixOpB" | "sixOpC" | "waveTerrain" | "stringMachine" | "chiptune" | "virtualAnalog" | "waveshaping" | "twoOpFm" | "granularFormant" | "additive" | "wavetable" | "chords" | "speech" | "swarm" | "filteredNoise" | "particleNoise" | "inharmonicString" | "modalResonator" | "bassDrum" | "snareDrum" | "hiHat", config?: { fm?: Poly<Signal>; fmAmt?: Poly<Signal>; harmonics?: Poly<Signal>; level?: Poly<Signal>; lpgColor?: Poly<Signal>; lpgDecay?: Poly<Signal>; morph?: Poly<Signal>; morphAmt?: Poly<Signal>; timbre?: Poly<Signal>; timbreAmt?: Poly<Signal>; trigger?: Poly<Signal>; id?: string }): MacroOutputs;

/**
 * 12dB/octave lowpass filter with resonance
 * @param input - signal input
 * @param cutoff - cutoff frequency in v/oct
 * @param resonance - filter resonance (0-5)
 * @param config - Configuration object
 */
export function lpf(input: Poly<Signal>, cutoff: Poly<Signal>, resonance?: Poly<Signal>, config?: { id?: string }): Collection;

/**
 * 12dB/octave highpass filter with resonance
 * @param input - signal input
 * @param cutoff - cutoff frequency in v/oct
 * @param resonance - filter resonance (0-5)
 * @param config - Configuration object
 */
export function hpf(input: Poly<Signal>, cutoff: Poly<Signal>, resonance?: Poly<Signal>, config?: { id?: string }): Collection;

/**
 * 12dB/octave bandpass filter
 * @param input - signal input
 * @param center - center frequency in v/oct
 * @param resonance - filter Q (bandwidth control, 0-5)
 * @param config - Configuration object
 */
export function bpf(input: Poly<Signal>, center: Poly<Signal>, resonance?: Poly<Signal>, config?: { id?: string }): Collection;

/**
 * XOR bit-crush phase-distortion adapted from 4ms Ensemble Oscillator
 * @param input - input phase (0 to 1)
 * @param amount - crush amount (0-5, where 0 = clean, 5 = maximum XOR distortion)
 * @param config - Configuration object
 */
export function crush(input: Poly<Signal>, amount?: Poly<Signal>, config?: { id?: string }): CollectionWithRange;

/**
 * FM feedback phase-distortion adapted from 4ms Ensemble Oscillator
 * @param input - input phase (0 to 1)
 * @param amount - feedback amount (0-5, where 0 = no feedback, 5 = maximum feedback FM)
 * @param config - Configuration object
 *   - freq - frequency in v/oct (optional, enables anti-aliasing when connected)
 */
export function feedback(input: Poly<Signal>, amount?: Poly<Signal>, config?: { freq?: Poly<Signal>; id?: string }): CollectionWithRange;

/**
 * Pulsar synthesis phase-distortion adapted from 4ms Ensemble Oscillator
 * @param input - input phase (0 to 1)
 * @param amount - compression amount (0-5, where 0 = no compression, 5 = 64x compression)
 * @param config - Configuration object
 *   - freq - frequency in v/oct (optional, enables anti-aliasing when connected)
 */
export function pulsar(input: Poly<Signal>, amount?: Poly<Signal>, config?: { freq?: Poly<Signal>; id?: string }): CollectionWithRange;

/**
 * Phase ramp generator (0 to 1)
 * @param freq - frequency in v/oct
 * @param config - Configuration object
 */
export function ramp(freq: Poly<Signal>, config?: { id?: string }): CollectionWithRange;

/**
 * ADSR envelope generator
 * @param gate - gate input (expects >0V for on)
 * @param config - Configuration object
 *   - attack - attack time in seconds
 *   - decay - decay time in seconds
 *   - release - release time in seconds
 *   - sustain - sustain level in volts (0-5)
 */
export function adsr(gate: Poly<Signal>, config?: { attack?: Poly<Signal>; decay?: Poly<Signal>; release?: Poly<Signal>; sustain?: Poly<Signal>; id?: string }): CollectionWithRange;

/**
 * Divides an incoming clock signal by a specified integer value
 * @param input
 * @param division
 * @param config - Configuration object
 */
export function clockDivider(input: Mono<Signal>, division: number, config?: { id?: string }): ModuleOutput;

/**
 * Lag Processor (Slew Limiter)
 * @param input
 * @param config - Configuration object
 *   - fall - fall time in seconds (default 0.01s)
 *   - rise - rise time in seconds (default 0.01s)
 */
export function slew(input: Poly<Signal>, config?: { fall?: Poly<Signal>; rise?: Poly<Signal>; id?: string }): Collection;

/**
 * Rising Edge Detector
 * @param input
 * @param config - Configuration object
 */
export function rising(input: Poly<Signal>, config?: { id?: string }): Collection;

/**
 * Falling Edge Detector
 * @param input
 * @param config - Configuration object
 */
export function falling(input: Poly<Signal>, config?: { id?: string }): Collection;

/**
 * Math expression evaluator
 * @param expression
 * @param config - Configuration object
 */
export function math(expression: string, config?: { x?: Mono<Signal>; y?: Mono<Signal>; z?: Mono<Signal>; id?: string }): ModuleOutput;

/**
 * remap a signal from one range to another
 * @param input - signal input to remap
 * @param inMin - minimum of input range
 * @param inMax - maximum of input range
 * @param outMin - minimum of output range
 * @param outMax - maximum of output range
 * @param config - Configuration object
 */
export function remap(input: Poly<Signal>, inMin?: Poly<Signal>, inMax?: Poly<Signal>, outMin?: Poly<Signal>, outMax?: Poly<Signal>, config?: { id?: string }): Collection;

/**
 * Sample and Hold
 * @param input
 * @param trigger
 * @param config - Configuration object
 */
export function sah(input: Poly<Signal>, trigger: Poly<Signal>, config?: { id?: string }): Collection;

/**
 * Track and Hold
 * @param input
 * @param gate
 * @param config - Configuration object
 */
export function tah(input: Poly<Signal>, gate: Poly<Signal>, config?: { id?: string }): Collection;

/**
 * Percussion envelope with exponential decay
 * @param trigger - trigger input (rising edge triggers envelope)
 * @param config - Configuration object
 *   - decay - decay time in seconds
 */
export function perc(trigger: Poly<Signal>, config?: { decay?: Poly<Signal>; id?: string }): CollectionWithRange;

/**
 * Output type for quantizer module.
 * Extends Collection (default output: output)
 */
export interface QuantizerOutputs extends Collection {
  /** trigger pulse on note change */
  readonly trig: CollectionWithRange;
}

/**
 * Quantizes V/Oct input to scale degrees
 * @param input - Input V/Oct signal to quantize
 * @param offset - Offset added to input before quantization (in V/Oct)
 * @param scale - Scale specification: "chromatic", "C(major)", "D(0 2 4 5 7 9 11)"
 * @param config - Configuration object
 */
export function quantizer(input: Poly<Signal>, offset?: Poly<Signal>, scale?: string, config?: { id?: string }): QuantizerOutputs;

/**
 * attenuate, invert, offset
 * @param input - signal input
 * @param scale - scale factor
 * @param shift - shift amount
 * @param config - Configuration object
 */
export function scaleAndShift(input: Poly<Signal>, scale?: Poly<Signal>, shift?: Poly<Signal>, config?: { id?: string }): Collection;

/**
 * Output type for cycle module.
 * Extends Collection (default output: cv)
 */
export interface CycleOutputs extends Collection {
  /** gate output */
  readonly gate: CollectionWithRange;
  /** trigger output */
  readonly trig: CollectionWithRange;
}

/**
 * A strudel/tidalcycles style sequencer
 * @param pattern - Strudel/tidalcycles style pattern string
 * @param config - Configuration object
 *   - channels - Number of polyphonic voices (1-16)
 *   - playhead - 2 channel control signal, sums the first 2 channels
 */
export function cycle(pattern: string, config?: { channels?: number; playhead?: Poly<Signal>; id?: string }): CycleOutputs;

/**
 * A sequencer track
 * @param keyframes - Keyframes as (polysignal, time) tuples. Must be sorted by time.
 * @param config - Configuration object
 *   - playhead - Playhead input - sums channels 0 and 1 for position
 */
export function track(keyframes: [Poly<Signal>, number][], config?: { interpolationType?: "linear" | "step" | "sineIn" | "sineOut" | "sineInOut" | "quadIn" | "quadOut" | "quadInOut" | "cubicIn" | "cubicOut" | "cubicInOut" | "quartIn" | "quartOut" | "quartInOut" | "quintIn" | "quintOut" | "quintInOut" | "expoIn" | "expoOut" | "expoInOut" | "circIn" | "circOut" | "circInOut" | "bounceIn" | "bounceOut" | "bounceInOut"; playhead?: Poly<Signal>; id?: string }): Collection;

/**
 * Output type for iCycle module.
 * Extends Collection (default output: cv)
 */
export interface ICycleOutputs extends Collection {
  /** gate output */
  readonly gate: CollectionWithRange;
  /** trigger output */
  readonly trig: CollectionWithRange;
}

/**
 * A scale-degree sequencer with interval and add patterns
 * @param intervalPattern - Primary interval/degree pattern
 * @param scale - Scale for quantizing degrees to pitches (supports optional octave, e.g. "C3(major)")
 * @param config - Configuration object
 *   - addPattern - Offset pattern added to interval_pattern
 *   - channels - Number of polyphonic voices (1-16)
 *   - playhead - 2 channel control signal, sums the first 2 channels
 */
export function iCycle(intervalPattern: string, scale: string, config?: { addPattern?: string; channels?: number; playhead?: Poly<Signal>; id?: string }): ICycleOutputs;

/**
 * Output type for midiCV module.
 * Extends Collection (default output: pitch)
 */
export interface MidiCVOutputs extends Collection {
  /** gate output (0V or 5V) */
  readonly gate: CollectionWithRange;
  /** velocity (0-5V) */
  readonly velocity: CollectionWithRange;
  /** channel pressure / aftertouch (0-5V) */
  readonly aftertouch: CollectionWithRange;
  /** retrigger pulse (5V for 1ms on new note) */
  readonly retrigger: CollectionWithRange;
  /** pitch wheel (-5V to +5V, unscaled) */
  readonly pitchWheel: CollectionWithRange;
  /** mod wheel (0-5V) */
  readonly modWheel: CollectionWithRange;
}

/**
 * MIDI to CV converter with polyphonic voice allocation
 * @param config - Configuration object
 *   - channel - MIDI channel filter (1-16, None = omni/all channels)
 *   - channels - Number of polyphonic voices (1-16)
 *   - device - MIDI device name to receive from (None = all devices)
 *   - monoMode - Monophonic note priority (when channels = 1)
 *   - pitchBendRange - Pitch bend range in semitones (0 = disabled, default 2)
 *   - polyMode - Polyphonic voice allocation mode
 */
export function midiCV(config?: { channel?: number; channels?: number; device?: string; monoMode?: "last" | "first" | "lowest" | "highest"; pitchBendRange?: number; polyMode?: "rotate" | "reuse" | "reset" | "mpe"; id?: string }): MidiCVOutputs;

/**
 * MIDI CC to CV converter
 * @param config - Configuration object
 *   - cc - CC number to monitor (0-127 for 7-bit, 0-31 for 14-bit mode)
 *   - channel - MIDI channel filter (1-16, None = omni/all channels)
 *   - device - MIDI device name to receive from (None = all devices)
 *   - highResolution - Enable 14-bit high-resolution CC mode (CC 0-31 MSB + CC 32-63 LSB)
 *   - smoothingMs - Smoothing time in milliseconds (0 = instant)
 */
export function midiCC(config?: { cc?: number; channel?: number; device?: string; highResolution?: boolean; smoothingMs?: number; id?: string }): ModuleOutputWithRange;
}

/** Default clock module running at 120 BPM. */
export const rootClock: $.ClockOutputs;

/** Input signals. */
export const input: Readonly<Collection>;
 
}

 export {};
