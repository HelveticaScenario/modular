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

interface Array<T> {
  /**
   * Pipe this array through a transform function.
   *
   * Passes `this` to `pipeFn` and returns the result, enabling inline
   * functional transforms and method chaining on any array.
   *
   * @param pipeFn - A function that receives this array and returns a transformed value
   * @returns The return value of `pipeFn`
   *
   * @example
   * // Pipe an array of outputs
   * [osc1, osc2, osc3].pipe(all => $mix(all)).out()
   */
  pipe<U>(this: this, pipeFn: (self: this) => U): U;
}

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
 * Extracts the element types from a tuple of arrays.
 * Used as the return type of {@link $cartesian} to enable typed destructuring.
 * @example
 * type T = ElementsOf<[number[], string[]]>; // [number, string]
 */
type ElementsOf<T extends unknown[][]> = { [K in keyof T]: T[K] extends (infer E)[] ? E : never };

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


/**
 * A signal input that sums all channels to a single mono value.
 * Structurally identical to {@link Poly}, but signals that the module
 * combines all voices into one control signal rather than preserving polyphony.
 *
 * @example $clock(120)                    // Constant tempo
 * @example $stereoMix(osc, { width: lfo }) // Width summed to mono
 * @see {@link Poly} - for polyphonic signals that preserve per-voice data
 * @see {@link Signal} - for single-channel signals
 */
type Mono<T extends Signal = Signal> = OrArray<T> | Iterable<ModuleOutput>;

/**
 * A phase-warp table descriptor produced by the `$table.*` helpers.
 *
 * Passed to modules that accept a `Table`-typed param (e.g. the
 * `phase` config field on `$wavetable`) to reshape a raw phase signal
 * before it is used to read a wavetable.
 *
 * Create one with `$table.mirror`, `$table.bend`, `$table.sync`,
 * `$table.fold`, or `$table.pwm` — do not construct directly.
 */
type Table =
  | { readonly type: "mirror"; readonly amount: Signal }
  | { readonly type: "bend"; readonly amount: Signal }
  | { readonly type: "sync"; readonly ratio: Signal }
  | { readonly type: "fold"; readonly amount: Signal }
  | { readonly type: "pwm"; readonly width: Signal }
  | { readonly type: "identity" };

/**
 * A buffer output reference — returned by `$buffer()`, passed to readers
 * (like `$bufRead`, `$delayRead`) as their `buffer` param.
 */
type BufferOutputRef = {
  readonly type: "buffer_ref";
  readonly module: string;
  readonly port: string;
  readonly channels: number;
  readonly frameCount: number;
};

/**
 * A loaded WAV sample handle — returned by `$wavs()`, passed to `$sampler()` as the `wav` param.
 */
type WavHandle = {
  readonly type: 'wav_ref';
  readonly path: string;
  readonly channels: number;
  readonly sampleRate: number;
  readonly frameCount: number;
  readonly duration: number;
  readonly bitDepth: number;
  /** File modification time (epoch ms). Cache-key hint — changes when the WAV is edited on disk. */
  readonly mtime: number;
  readonly pitch?: number;
  readonly playback?: 'one-shot' | 'loop';
  readonly bpm?: number;
  readonly beats?: number;
  readonly timeSignature?: {
    readonly num: number;
    readonly den: number;
  };
  readonly loops: ReadonlyArray<{
    readonly type: 'forward' | 'pingpong' | 'backward';
    readonly start: number;
    readonly end: number;
  }>;
  readonly cuePoints: ReadonlyArray<{
    readonly position: number;
    readonly label: string;
  }>;
};

/**
 * Options for stereo output routing via the out() method.
 * @see {@link ModuleOutput.out}
 * @see {@link Collection.out}
 */
interface StereoOutOptions {
  /** Base output channel (0-15, default 0). Left plays on baseChannel, right on baseChannel+1 */
  baseChannel?: number;
  /** Output gain. If set, a $scaleAndShift module is added after the stereo mix */
  gain?: Poly<Signal>;
  /** Pan position (-5 = left, 0 = center, +5 = right). Default 0 */
  pan?: Poly<Signal>;
  /** Stereo width/spread (0 = no spread, 5 = full spread). Default 0 */
  width?: Mono<Signal>;
}

/**
 * A single output from a module, representing a mono signal connection.
 * 
 * ModuleOutputs are chainable - methods like amplitude(), shift(), and out() 
 * return the same output for fluent API usage.
 * 
 * @example
 * const osc = osc.sine("C4")
 * osc.amplitude(0.5).out()           // Chain methods
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
     * Scale the signal by a linear factor (5 = unity, 2.5 = half, 10 = 2x).
     * Creates a $scaleAndShift module internally.
     *
     * For perceptual (audio-taper) volume control, use {@link gain} instead.
     * @param factor - Scale factor as {@link Poly<Signal>}
     * @returns The scaled {@link Collection} for chaining
     * @example osc.amplitude(2.5)  // Half amplitude
     */
   amplitude(factor: Poly<Signal>): Collection;

   /** Alias for {@link amplitude} */
   amp(factor: Poly<Signal>): Collection;
  
  /**
   * Add a DC offset to the signal. Creates a $scaleAndShift module internally.
   * @param offset - Offset value as {@link Poly<Signal>}
   * @returns The shifted {@link Collection} for chaining
   * @example lfo.shift(2.5)  // Shift to 0-5V range
   */
  shift(offset: Poly<Signal>): Collection;

    /**
     * Scale the signal by a factor with a perceptual (audio taper) curve
     * (5 = unity, 0 = silence).
     *
     * For linear amplitude scaling, use {@link amplitude} instead.
     * @param level - Amplitude level as {@link Poly<Signal>}
     * @returns The scaled {@link Collection} for chaining
     * @example osc.gain(2.5)
     */
   gain(level: Poly<Signal>): Collection;

  /**
   * Apply a power curve to this signal. Creates a \$curve module internally.
   * @param factor - Exponent for the curve (default 3)
   * @returns The curved {@link Collection} for chaining
   * @example lfo.exp(2)  // Quadratic curve
   */
  exp(factor?: Poly<Signal>): Collection;
  
  /**
   * Add scope visualization for this output.
   * The scope appears as an overlay in the editor.
   * @param config - Scope configuration options
   * @param config.msPerFrame - Time window in milliseconds (default 500)
   * @param config.triggerThreshold - Trigger threshold in volts (optional)
   * @param config.triggerWaitToRender - Whether the scope should wait to render until the buffer fills (default true). Only applicable if triggerThreshold is set.
   * @param config.range - Voltage range for display as [min, max] tuple (default [-5, 5])
   */
  scope(config?: { msPerFrame?: number; triggerThreshold?: number; triggerWaitToRender?: boolean; range?: [number, number] }): this;
  
  /**
   * Send this output to speakers as stereo.
   * @param options - Stereo output options ({@link StereoOutOptions})
   * @example osc.out({ gain: 2.5, pan: -2 })
   */
  out(options?: StereoOutOptions): this;
  
  /**
   * Send this output to speakers as mono.
   * @param channel - Output channel (0-15, default 0)
    * @param gain - Output gain as {@link Poly<Signal>} (optional)
    * @example lfo.outMono(2, 0.3)
    */
   outMono(channel?: number, gain?: Poly<Signal>): this;

  /**
   * Pipe this output through a transform function.
   *
   * Passes `this` to `pipeFn` and returns the result, enabling inline
   * functional transforms and reusable signal-processing helpers.
   *
   * @param pipeFn - A function that receives this output and returns a transformed value
   * @returns The return value of `pipeFn`
   *
   * @example
   * // Inline transform
   * $sine('a').pipe(s => s.amplitude(0.5).shift(1))
   *
   * @example
   * // Reusable helper
   * const tremolo = (c) => c.amplitude($sine('10hz').range(4, 5))
   * $saw('c').pipe(tremolo).out()
   */
  pipe<T>(pipeFn: (self: this) => T): T;
  /**
   * Pipe this output through a transform for each element of an array.
   * Returns a {@link Collection} containing one output per element.
   *
   * @param pipeFn - A function that receives this output and one element from the array
   * @param array - An array whose elements are passed to `pipeFn` one by one
   * @returns A {@link Collection} with one item per element
   *
   * @example
   * // 6 outputs
   * $sine(['C4', 'E4', 'G4']).pipe(
   *   (s, cut) => $lph(s, cut),
   *   ['440hz', '880hz'],
   * ).out()
   */
  pipe<T extends ModuleOutput | Iterable<ModuleOutput>, E>(
    pipeFn: (self: this, item: E) => T,
    array: E[]
  ): Collection;

  /**
   * Pipe this output through a transform, then mix the original and transformed
   * signals together using a \$mix module.
   *
   * @param pipeFn - A function that receives this output and returns a signal to mix with the original
   * @param mix - Optional crossfade as {@link Poly<Signal>}. 0 for only original, 5 for only transformed. Default is 2.5 for equal mix.
   * @returns A Collection from the \$mix output
   *
   * @example
   * // Mix original with a filtered version
   * $saw('c4').pipeMix(s => $lpf(s, '1000hz')).out()
   *
   * @example
   * // Mix with custom balance
   * $saw('c4').pipeMix(s => $lpf(s, '1000hz'), 1.0).out()
   */
  pipeMix(pipeFn: (self: this) => ModuleOutput | Collection, mix?: Poly<Signal> ): Collection;

  /**
   * Remap this output from an explicit input range to a new output range.
   * Creates a $remap module internally.
   * @param outMin - New minimum as {@link Poly<Signal>}
   * @param outMax - New maximum as {@link Poly<Signal>}
   * @param inMin - Input minimum as {@link Poly<Signal>}
   * @param inMax - Input maximum as {@link Poly<Signal>}
   * @returns A {@link ModuleOutput} with the remapped signal
   * @example $sine('c4').range(0, 1, -5, 5)
   */
  range(outMin: Poly<Signal>, outMax: Poly<Signal>, inMin: Poly<Signal>, inMax: Poly<Signal>): ModuleOutput;

  /**
   * Register this output as a send to a bus, with optional gain.
   * @param bus - The {@link Bus} to send to
   * @param gain - Send level as {@link Poly<Signal>}
   * @returns This output for chaining
   */
  send(bus: Bus, gain?: Poly<Signal>): this;
}

/**
 * DeferredModuleOutput is a placeholder for a signal that will be assigned later.
 * Useful for feedback loops and forward references in the DSL.
 * Supports the same chainable methods as ModuleOutput (amplitude, shift, scope, out, outMono).
 */
interface DeferredModuleOutput extends ModuleOutput {
  /**
   * Set the actual signal this deferred output should resolve to.
   * @param signal - The signal to resolve to (number, string, or ModuleOutput)
   */
  set(signal: Signal): void;
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


class BaseCollection<T extends ModuleOutput> implements Iterable<T> {
  /** Number of outputs in the collection */
  readonly length: number;
  /** Index access to individual elements */
  readonly [index: number]: T;
  [Symbol.iterator](): Iterator<T>;

    /**
     * Scale all signals by a linear factor (5 = unity, 2.5 = half, 10 = 2x).
     *
     * For perceptual (audio-taper) volume control, use {@link gain} instead.
     * @param factor - Scale factor as {@link Poly<Signal>}
     * @see {@link ModuleOutput.amplitude}
     */
   amplitude(factor: Poly<Signal>): Collection;

   /** Alias for {@link amplitude} */
   amp(factor: Poly<Signal>): Collection;

  /**
   * Add DC offset to all signals.
   * @param offset - Offset as {@link Poly<Signal>}
   * @see {@link ModuleOutput.shift}
   */
  shift(offset: Poly<Signal>): Collection;

    /**
     * Scale all signals by a factor with a perceptual (audio taper) curve
     * (5 = unity, 0 = silence).
     *
     * For linear amplitude scaling, use {@link amplitude} instead.
     * @param level - Amplitude level as {@link Poly<Signal>}
     * @see {@link ModuleOutput.gain}
     */
  gain(level: Poly<Signal>): Collection;

  /**
   * Apply a power curve to all signals. Creates a \$curve module internally.
   * @param factor - Exponent for the curve (default 3)
   * @see {@link ModuleOutput.exp}
   */
  exp(factor?: Poly<Signal>): Collection;

  /**
   * Add scope visualization for the first output in the collection.
   * @param config - Scope configuration options
   * @param config.msPerFrame - Time window in milliseconds (default 500)
   * @param config.triggerThreshold - Trigger threshold in volts (optional)
   * @param config.triggerWaitToRender - Whether the scope should wait to render until the buffer fills (default true). Only applicable if triggerThreshold is set.
   * @param config.range - Voltage range for display as [min, max] tuple (default [-5, 5])
   */
  scope(config?: { msPerFrame?: number; triggerThreshold?: number; triggerWaitToRender?: boolean; range?: [number, number] }): this;

  /**
   * Send all outputs to speakers as stereo, summed together.
   * @param options - Stereo output options ({@link StereoOutOptions})
   */
  out(options?: StereoOutOptions): this;

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
  range(outMin: Poly<Signal>, outMax: Poly<Signal>, inMin: Poly<Signal>, inMax: Poly<Signal>): Collection;

  /**
   * Pipe this collection through a transform function.
   *
   * Passes `this` to `pipeFn` and returns the result, enabling inline
   * functional transforms and reusable signal-processing helpers.
   *
   * @param pipeFn - A function that receives this collection and returns a transformed value
   * @returns The return value of `pipeFn`
   *
   * @example
   * // Inline transform on a collection
   * $(osc1, osc2).pipe(all => all.amplitude(2.5)).out()
   *
   * @example
   * // Reusable helper applied to a collection
   * const tremolo = (c) => c.amplitude($sine('10hz').range(4, 5))
   * $r($saw('220hz'), $saw('221hz')).pipe(tremolo).out()
   */
  pipe<T>(pipeFn: (self: this) => T): T;
  /**
   * Pipe this collection through a transform for each element of an array.
   * Returns a {@link Collection} containing one output per element.
   *
   * @param pipeFn - A function that receives this collection and one element from the array
   * @param array - An array whose elements are passed to `pipeFn` one by one
   * @returns A {@link Collection} with one item per element
   *
   * @example
   * // Apply each filter cutoff to the whole collection
   * $c(osc1, osc2).pipe(
   *   (col, cutoff) => $lpf(col, cutoff),
   *   ['200hz', '800hz', '3200hz'],
   * ).out()
   */
  pipe<T extends ModuleOutput | Iterable<ModuleOutput>, E>(
    pipeFn: (self: this, item: E) => T,
    array: E[]
  ): Collection;

  /**
   * Pipe this collection through a transform, then mix the original and transformed
   * signals together using a \$mix module.
   *
   * @param pipeFn - A function that receives this collection and returns a signal to mix with the original
   * @param mix - Optional crossfade as {@link Poly<Signal>}. 0 for only original, 5 for only transformed. Default is 2.5 for equal mix.
   * @returns A Collection from the \$mix output
   *
   * @example
   * // Mix collection with a filtered version
   * $c(osc1, osc2).pipeMix(s => $lpf(s, '1000hz')).out()
   *
   * @example
   * // Mix with different balance
   * $c(osc1, osc2).pipeMix(s => $lpf(s, '1000hz'), 1).out()
   */
  pipeMix(pipeFn: (self: this) => ModuleOutput | Collection, mix?: Poly<Signal> ): Collection;

  /**
   * Register all outputs in this collection as a send to a bus, with optional gain.
   * @param bus - The {@link Bus} to send to
   * @param gain - Send level as {@link Poly<Signal>}
   * @returns This collection for chaining
   */
  send(bus: Bus, gain?: Poly<Signal>): this;
}

/**
 * A collection of {@link ModuleOutput} instances with chainable DSP methods.
 * 
 * Created with the $() helper function. Supports iteration, indexing, and spreading.
 * Methods operate on all outputs in the collection.
 * 
 * @example
 * $(osc1, osc2, osc3).amplitude(0.5).out()  // Apply amplitude to all
 * for (const v of voices) { ... }      // Iterate
 * [...voices]                          // Spread to array
 * voices[0]                            // Index access
 * 
 * @see {@link CollectionWithRange} - for ranged outputs
 * @see {@link ModuleOutput} - individual outputs
 * @see {@link $} - helper to create Collection
 */
class Collection extends BaseCollection<ModuleOutput> {
  constructor(...outputs: ModuleOutput[]);
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
class CollectionWithRange extends BaseCollection<ModuleOutputWithRange> {
  constructor(...outputs: ModuleOutputWithRange[]);

  /**
   * Remap all outputs from their native ranges to a new range.
   * Uses each output's stored minValue/maxValue.
   * @param outMin - Output minimum as {@link Poly<Signal>}
   * @param outMax - Output maximum as {@link Poly<Signal>}
   * @see {@link Collection.range} - for explicit input range
   */
  override range(outMin: Poly<Signal>, outMax: Poly<Signal>): Collection;
}

/**
 * DeferredCollection is a collection of DeferredModuleOutput instances.
 * Provides a .set() method to assign signals to all contained deferred outputs.
 */
class DeferredCollection extends BaseCollection<DeferredModuleOutput> {
  constructor(...outputs: DeferredModuleOutput[]);

  /**
   * Set the signals for all deferred outputs in this collection.
   * @param polySignal - A Poly<Signal> (single signal, array, or iterable) to distribute across outputs
   */
  set(polySignal: Poly<Signal>): void;
}


// Helper functions exposed by the DSL runtime

/**
 * Convert a frequency in Hertz to a voltage value (1V/octave).
 * @param frequency - Frequency in Hz
 * @returns Voltage value for use as a {@link Signal}
 * @example $hz(440)  // A4
 * @example $hz(261.63)  // ~C4
 */
function $hz(frequency: number): number;

/**
 * Convert a note name string to a voltage value (1V/octave).
 * @param noteName - Note name like "C4", "A#3", "Bb5"
 * @returns Voltage value for use as a {@link Signal}
 * @example $note("C4")  // Middle C
 * @example $note("A4")  // 440 Hz
 */
function $note(noteName: string): number;

/**
 * Create a {@link Collection} from {@link ModuleOutput} instances.
 * 
 * Collections support chainable DSP methods, iteration, indexing, and spreading.
 * @param args - One or more {@link ModuleOutput}s to group
 * @returns A {@link Collection} of the outputs
 * @example $c(osc1, osc2).amplitude(0.5).out()
 * @example $c(osc1, osc2, osc3)[0]  // Index access
 * @example [...$c(osc1, osc2)]      // Spread to array
 * @see {@link $r} - for ranged outputs
 */
function $c(...args: (ModuleOutput | Iterable<ModuleOutput>)[]): Collection;

/**
 * Create a {@link CollectionWithRange} from {@link ModuleOutputWithRange} instances.
 * 
 * Like $() but the range() method uses stored min/max values.
 * @param args - One or more {@link ModuleOutputWithRange}s to group
 * @returns A {@link CollectionWithRange} of the outputs
 * @example $r(lfo1, lfo2).range(0, 5)  // Uses stored ranges
 * @example $r(...seq.gates).range(0, 1)
 * @see {@link $c} - for outputs without known ranges
 */
function $r(...args: (ModuleOutputWithRange | Iterable<ModuleOutputWithRange>)[]): CollectionWithRange;

/**
 * Set the global tempo for the root clock.
 * @param tempo - Tempo in BPM
 * @example $setTempo(120)  // 120 beats per minute
 * @example $setTempo(140)  // 140 beats per minute
 */
function $setTempo(tempo: number): void;

/**
 * Set the global output gain applied to the final mix.
 * @param gain - Gain as a Mono<Signal> (2.5 is default, 5.0 is unity)
 * @example $setOutputGain(2.5) // 50% gain (default)
 * @example $setOutputGain(5.0) // unity
 * @example $setOutputGain(env.out) // modulate gain from envelope
 */
function $setOutputGain(gain: Mono<Signal>): void;

/**
 * Set the time signature for the root clock.
 * Both values must be positive integers.
 * @param numerator - Beats per bar (e.g. 3, 4, 6, 7)
 * @param denominator - Beat value (e.g. 4 for quarter note, 8 for eighth note)
 * @example $setTimeSignature(4, 4)  // 4/4 time (default)
 * @example $setTimeSignature(3, 4)  // 3/4 waltz time
 * @example $setTimeSignature(6, 8)  // 6/8 compound time
 * @example $setTimeSignature(7, 8)  // 7/8 asymmetric time
 * @example $setTimeSignature(5, 4)  // 5/4 time
 */
function $setTimeSignature(numerator: number, denominator: number): void;

/**
 * Create a DeferredCollection with placeholder signals that can be assigned later.
 * Useful for feedback loops and forward references.
 * @param channels - Number of deferred outputs (1-16, default 1)
 * @example
 * const feedback = $deferred();
 * const delayed = $delay(osc.out, feedback[0]);
 * feedback.set(delayed);
 */
function $deferred(channels?: number): DeferredCollection;

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
 * const vol = $slider("Volume", 0.5, 0, 1);
 * $sine(440).amplitude(vol).out();
 */
function $slider(label: string, value: number, min: number, max: number): ModuleOutput;

/**
 * A send-return bus. Create one with {@link $bus}, then call `.send(bus, gain)` on
 * any {@link ModuleOutput} or {@link Collection} to route signals through it.
 * The bus callback receives a mixed {@link Collection} of all sends.
 */
class Bus {
  /** @internal */
  private constructor();
}

/**
 * Create a send-return bus.
 *
 * The callback receives a {@link Collection} that is the mix of all signals
 * sent to this bus via `.send(bus, gain)`. Use it to add effects or route the
 * mixed signal to an output.
 *
 * @param cb - Called during patch finalization with the mixed sends.
 *             The return value of this function is discarded, it's up to the cb to
 *             call `.out()` or `.outMono()` to actually hear anything.
 * @returns A {@link Bus} handle passed to `.send()`
 *
 * @example
 * const reverb = \$bus((mixed) => \$reverb(mixed).out());
 * \$saw('a').send(reverb, 0.6);
 * \$sine('a2').send(reverb, 0.4);
 */
function $bus(cb: (mixed: Collection) => unknown): Bus;

/**
 * Set a custom end-of-chain processor applied to the final mix before output gain.
 *
 * The callback receives the fully mixed {@link Collection} and should return a
 * processed signal. It is called once during patch finalization.
 *
 * @param cb - Transform applied to the final mix
 *
 * @example
 * $setEndOfChainCb((mix) => $lpf(mix, '2000hz'));
 */
function $setEndOfChainCb(cb: (mixed: Collection) => ModuleOutput | Collection | CollectionWithRange): void;

/**
 * Compute the Cartesian product of the given arrays.
 *
 * Returns every possible combination of one element from each array,
 * as a typed tuple array. Pairs well with the array overload of `.pipe()`
 * to fan a signal across multiple parameter dimensions.
 *
 * @param arrays - Zero or more arrays to combine
 * @returns Array of typed tuples, one per combination
 *
 * @example
 * // Fan an oscillator across every combination of frequency and waveform
 * $cartesian([220, 440, 880], ['sine', 'saw']).pipe(
 *   (osc, [freq, shape]) => $oscillator({ freq, shape }).out(),
 * ).out();
 *
 * @example $cartesian([1, 2], ['a', 'b'])
 * // → [[1,'a'], [1,'b'], [2,'a'], [2,'b']]
 */
function $cartesian<A extends unknown[][]>(...arrays: A): ElementsOf<A>[];

/**
 * Phase-warp table descriptors for modules that accept a {@link Table}
 * (e.g. the `phase` config field on `$wavetable`).
 *
 * Each helper returns a {@link Table} whose inner signal-valued field
 * accepts a constant, a module output, or any other {@link Signal}.
 *
 * @example
 * // Symmetric mirror warp driven by an LFO
 * $wavetable(\$wavs().mywavs.mytable, 'c4', {
 *   phase: $table.mirror($lfo.sine('1hz')),
 * }).out();
 */
declare const $table: {
    /** Reflect the phase around its midpoint by `amount` (0..1). */
    mirror(amount: Signal): Table;
    /** Bend the phase curve by `amount` (0..1 = linear..extreme). */
    bend(amount: Signal): Table;
    /** Hard-sync: restart the phase every `ratio` of a cycle. */
    sync(ratio: Signal): Table;
    /** Fold the phase back on itself by `amount`. */
    fold(amount: Signal): Table;
    /** Pulse-width modulation warp with duty cycle `width` (0..1). */
    pwm(width: Signal): Table;
};


/**
 * Utility module for routing, naming, and exposing signals in a patch.
 * @param source - Input signal to forward.
 * @param config - Configuration object
 */
export function $signal(source: Poly<Signal>, config?: { id?: string }): Collection;

/**
 * Mix module for combining multiple signals into a single mix bus.
 * 
 * Use this when you want to blend several multichannel modulation/audio sources.
 * It mixes channel `n` across all inputs into output channel `n`, rather than
 * folding all channels into a single mono channel.
 * @param inputs - Input signals to mix channel-by-channel.
 * @param config - Configuration object
 *   - gain - Final output level (perceptual curve, exponent 3).
 *   - mode - How inputs are combined.
 *   -     - `"sum"` — Sum all inputs.
 *   -     - `"average"` — Average all inputs.
 *   -     - `"max"` — Keep the strongest input.
 *   -     - `"min"` — Keep the weakest non-zero input.
 */
export function $mix(inputs: Poly<Signal>[], config?: { gain?: Poly<Signal>; mode?: "sum" | "average" | "max" | "min"; id?: string }): Collection;

/**
 * Pan and spread a signal into stereo.
 * @param input - Input signal to place in the stereo field.
 * @param config - Configuration object
 *   - pan - Pan position per channel (-5 = left, 0 = center, +5 = right).
 *   - width - Stereo spread across channels (0 = no spread, 5 = widest spread).
 */
export function $stereoMix(input: Poly<Signal>, config?: { pan?: Poly<Signal>; width?: Mono<Signal>; id?: string }): Collection;

/**
 * EXPERIMENTAL
 * 
 * Single-band feed-forward compressor with peak envelope follower.
 * 
 * Applies feed-forward compression in the log domain with configurable
 * threshold, ratio, attack/release ballistics, and makeup gain. Input and
 * output gain staging allow driving signal into the compressor and trimming
 * the output level independently. A dry/wet mix control enables parallel
 * compression.
 * 
 * **Signal flow:** input → input gain → compressor → output gain → dry/wet mix → output
 * 
 * - **threshold** — compression threshold in volts (0–5, default 2.5).
 * - **ratio** — compression ratio (1–20, default 4.0).
 * - **attack** / **release** — envelope follower time constants in seconds.
 * - **makeup** — post-compression makeup gain (linear multiplier, 0–5).
 * - **inputGain** — gain before the compressor (-5V = -24dB, 0V = unity,
 *   5V = +24dB). Raising input gain drives more signal into the compressor.
 * - **outputGain** — gain after compression (-5V = -24dB, 0V = unity,
 *   5V = +24dB). Trims the final output level.
 * - **mix** — dry/wet blend (0 = fully dry, 5 = fully wet, default 5.0).
 *   The dry signal is the original input before any gain staging.
 * 
 * ```js
 * // simple bus compressor
 * $comp(input, { threshold: 2.5, ratio: 4, attack: 0.01, release: 0.1 })
 * ```
 * 
 * ```js
 * // multiband compression using $xover + $comp
 * let bands = $xover(input, { lowMidFreq: '200hz', midHighFreq: '2000hz' })
 * let low  = $comp(bands.low,  { threshold: 2.5, ratio: 4 })
 * let mid  = $comp(bands.mid,  { threshold: 3,   ratio: 3 })
 * let high = $comp(bands.high, { threshold: 2,   ratio: 6 })
 * $mix(low, mid, high).out()
 * ```
 * @param input - audio input signal
 * @param config - Configuration object
 *   - attack - attack time in seconds (default 0.01)
 *   - inputGain - input gain control (-5V = -24dB, 0V = unity, 5V = +24dB) — drives signal into the compressor
 *   - makeup - makeup gain multiplier (0-5, default 1.0)
 *   - mix - dry/wet blend (0 = fully dry, 5 = fully wet, default 5.0)
 *   - outputGain - output gain control (-5V = -24dB, 0V = unity, 5V = +24dB) — trims level after compression
 *   - ratio - compression ratio (1-20, default 4.0)
 *   - release - release time in seconds (default 0.1)
 *   - threshold - compression threshold (0-5V, default 2.5)
 */
export function $comp(input: Poly<Signal>, config?: { attack?: Poly<Signal>; inputGain?: Poly<Signal>; makeup?: Poly<Signal>; mix?: Poly<Signal>; outputGain?: Poly<Signal>; ratio?: Poly<Signal>; release?: Poly<Signal>; threshold?: Poly<Signal>; id?: string }): Collection;

/**
 * Output type for $xover module.
 * Extends Collection (default output: output)
 */
export interface XoverOutputs extends Collection {
  /** low band output */
  readonly low: Collection;
  /** mid band output */
  readonly mid: Collection;
  /** high band output */
  readonly high: Collection;
}

/**
 * EXPERIMENTAL
 * 
 * Three-band crossover / band splitter.
 * 
 * Splits an input signal into three frequency bands (low, mid, high).
 * The default `sample` output passes the input through unchanged,
 * so the module is a no-op unless you explicitly tap the
 * `.low`, `.mid`, or `.high` outputs.
 * 
 * Two crossover frequencies define the band boundaries:
 * - **lowMidFreq** — boundary between the low and mid bands (V/Oct, default ~200 Hz).
 * - **midHighFreq** — boundary between the mid and high bands (V/Oct, default ~2000 Hz).
 * 
 * ```js
 * // Split into 3 bands and process each independently
 * let bands = $xover(input, { lowMidFreq: '200hz', midHighFreq: '2000hz' })
 * let low  = $comp(bands.low,  { threshold: 2.5, ratio: 4 })
 * let mid  = $comp(bands.mid,  { threshold: 3,   ratio: 3 })
 * let high = $comp(bands.high, { threshold: 2,   ratio: 6 })
 * $mix([low, mid, high]).out()
 * ```
 * @param input - audio input signal
 * @param config - Configuration object
 *   - lowMidFreq - crossover frequency between low and mid bands (V/Oct, 0V = C4)
 *   - midHighFreq - crossover frequency between mid and high bands (V/Oct, 0V = C4)
 */
export function $xover(input: Poly<Signal>, config?: { lowMidFreq?: Poly<Signal>; midHighFreq?: Poly<Signal>; id?: string }): XoverOutputs;

/**
 * Wavefolder that reflects the signal back when it exceeds a threshold,
 * producing dense, harmonically rich tones. Higher amounts create more
 * complex, metallic timbres.
 * @param input - input signal to fold (bipolar, typically -5 to 5)
 * @param amount - fold amount (0-5, where 0 = bypass, 5 = maximum folding)
 * @param config - Configuration object
 *   - freq - pitch of the source signal in V/Oct (optional, reduces aliasing at high frequencies)
 */
export function $fold(input: Poly<Signal>, amount: Poly<Signal>, config?: { freq?: Poly<Signal>; id?: string }): CollectionWithRange;

/**
 * Harmonic waveshaping effect that adds controlled overtone content.
 * 
 * At low amounts the signal passes through cleanly; turning it up
 * progressively emphasizes higher harmonics (2nd, 3rd, … up to 16th),
 * thickening and brightening the tone.
 * @param input - input signal to shape (bipolar, typically -5 to 5)
 * @param amount - harmonic richness (0–5). At 0 the signal is clean; at 5 the highest harmonic content dominates
 * @param config - Configuration object
 *   - freq - pitch of the source signal in V/Oct (optional, reduces aliasing at high frequencies)
 */
export function $cheby(input: Poly<Signal>, amount: Poly<Signal>, config?: { freq?: Poly<Signal>; id?: string }): CollectionWithRange;

/**
 * Stereo plate reverb based on the Dattorro algorithm.
 * 
 * Implements Jon Dattorro's plate reverberator with input diffusion,
 * a cross-coupled stereo tank, and multi-tap output. Even input
 * channels are summed to the left input, odd channels to the right.
 * Output is always 100% wet.
 * 
 * ```js
 * $dattorro($saw('c3'), { decay: 3, damping: 1, size: 2 }).out()
 * ```
 * @param input - audio input (even channels → left, odd channels → right)
 * @param config - Configuration object
 *   - damping - high-frequency damping in the reverb tank (-5 to 5, default 0)
 *   - decay - reverb decay time (-5 to 5, default 0)
 *   - diffusion - input diffusion amount (0 to 5, default 3.5)
 *   - modulation - external tank modulation signal (-5 to 5, default 0, not clamped)
 *   - predelay - predelay time in seconds (0 to 0.5, default 0)
 *   - size - room size — scales all delay line lengths (-5 to 5, default 0)
 */
export function $dattorro(input: Poly<Signal>, config?: { damping?: Mono<Signal>; decay?: Mono<Signal>; diffusion?: Mono<Signal>; modulation?: Mono<Signal>; predelay?: Mono<Signal>; size?: Mono<Signal>; id?: string }): Collection;

/**
 * Stereo plate reverb with a dense Glicol-inspired feedback network.
 * 
 * Uses a longer feedback path with distributed damping and more allpass
 * stages than the standard Dattorro algorithm, producing a thicker,
 * warmer reverb tail. Always 100% wet — use `.send()` or `$mix` for
 * dry/wet blending.
 * 
 * ```js
 * $plate($saw('c3')).out()
 * $plate($saw('c3'), { decay: 2, bandwidth: 3 }).out()
 * $plate($saw('c3'), { modulation: $sine('0.1hz') }).out()
 * ```
 * @param input - audio input (even channels → left, odd channels → right)
 * @param config - Configuration object
 *   - bandwidth - input bandwidth — controls high-frequency content entering the tank.
 *   - damping - tank damping — higher values absorb more high frequencies per recirculation.
 *   - decay - feedback decay — controls how long the reverb tail sustains.
 *   - modulation - external tank modulation signal.
 */
export function $plate(input: Poly<Signal>, config?: { bandwidth?: Mono<Signal>; damping?: Mono<Signal>; decay?: Mono<Signal>; modulation?: Mono<Signal>; id?: string }): Collection;

/**
 * Waveshaper that morphs through 8 distinct tonal shapes as you sweep the
 * amount control. Low settings pass the signal cleanly; mid settings compress
 * and square it off; high settings introduce stepped, ripple-like overtone
 * patterns. Works best on simple waveforms like sines or triangles.
 * @param input - input signal to shape (bipolar, typically -5 to 5)
 * @param amount - segment shape amount (0-5, morphs between 8 shapes)
 * @param config - Configuration object
 *   - freq - pitch of the source signal in V/Oct (optional, reduces aliasing at high frequencies)
 */
export function $segment(input: Poly<Signal>, amount: Poly<Signal>, config?: { freq?: Poly<Signal>; id?: string }): CollectionWithRange;

/**
 * A sine wave oscillator.
 * 
 * ## Example
 * 
 * ```js
 * $sine('c4').out()
 * ```
 * @param freq - pitch in V/Oct (0V = C4)
 * @param config - Configuration object
 *   - fm - FM input signal (pre-scaled by user)
 *   - fmMode - FM mode: throughZero (default), lin, or exp
 *   -     - `"throughZero"` — Through-zero FM: frequency can go negative (phase runs backward)
 *   -     - `"lin"` — Linear FM: like through-zero but frequency clamped to >= 0
 *   -     - `"exp"` — Exponential FM: modulator added to pitch in V/Oct space
 */
export function $sine(freq: Poly<Signal>, config?: { fm?: Poly<Signal>; fmMode?: "throughZero" | "lin" | "exp"; id?: string }): CollectionWithRange;

/**
 * A variable-symmetry triangle oscillator that morphs between saw, triangle, and ramp.
 * 
 * The `shape` parameter shifts the peak position of a triangle wave,
 * smoothly morphing between waveforms by adjusting attack/release time:
 * - **0** — Saw (all rise, instant drop)
 * - **2.5** — Triangle (symmetric)
 * - **5** — Ramp (instant rise, all fall)
 * 
 * The `freq` input follows the **V/Oct** standard (0V = C4).
 * Output range is **±5V**.
 * 
 * ## Example
 * 
 * ```js
 * $saw('a3', { shape: 2.5 }).out() // triangle wave
 * ```
 * @param freq - pitch in V/Oct (0V = C4)
 * @param config - Configuration object
 *   - fm - FM input signal (pre-scaled by user)
 *   - fmMode - FM mode: throughZero (default), lin, or exp
 *   -     - `"throughZero"` — Through-zero FM: frequency can go negative (phase runs backward)
 *   -     - `"lin"` — Linear FM: like through-zero but frequency clamped to >= 0
 *   -     - `"exp"` — Exponential FM: modulator added to pitch in V/Oct space
 *   - shape - waveform shape: 0=saw, 2.5=triangle, 5=ramp
 */
export function $saw(freq: Poly<Signal>, config?: { fm?: Poly<Signal>; fmMode?: "throughZero" | "lin" | "exp"; shape?: Poly<Signal>; id?: string }): CollectionWithRange;

/**
 * Pulse/square wave oscillator with pulse width modulation.
 * 
 * The `freq` input follows the **V/Oct** standard (0V = C4).
 * The `width` parameter sets the duty cycle: 0 = narrow pulse,
 * 2.5 = square wave, 5 = inverted narrow pulse.
 * `pwm` is added to `width` for modulation.
 * 
 * Output range is **±5V**.
 * 
 * ## Example
 * 
 * ```js
 * $pulse('c3', { width: 2.5 }).out()
 * ```
 * @param freq - pitch in V/Oct (0V = C4)
 * @param config - Configuration object
 *   - fm - FM input signal (pre-scaled by user)
 *   - fmMode - FM mode: throughZero (default), lin, or exp
 *   -     - `"throughZero"` — Through-zero FM: frequency can go negative (phase runs backward)
 *   -     - `"lin"` — Linear FM: like through-zero but frequency clamped to >= 0
 *   -     - `"exp"` — Exponential FM: modulator added to pitch in V/Oct space
 *   - pwm - pulse width modulation CV — added to the width parameter
 *   - width - pulse width (0-5, 2.5 is square)
 */
export function $pulse(freq: Poly<Signal>, config?: { fm?: Poly<Signal>; fmMode?: "throughZero" | "lin" | "exp"; pwm?: Poly<Signal>; width?: Poly<Signal>; id?: string }): CollectionWithRange;

/**
 * Phase-driven sine wave oscillator.
 * 
 * Instead of a frequency input, this oscillator is driven by an external
 * phasor signal (0–1). Connect a `ramp` or other phase source to `phase`
 * and use phase-distortion modules between them for complex timbres.
 * 
 * Output range is **±5V**.
 * @param phase - phasor input (0–1, wraps at boundaries)
 * @param config - Configuration object
 */
export function $pSine(phase: Poly<Signal>, config?: { id?: string }): CollectionWithRange;

/**
 * Phase-driven variable-symmetry triangle oscillator.
 * 
 * Instead of a frequency input, this oscillator is driven by an external
 * phasor signal (0–1). Connect a `ramp` or other phase source to `phase`
 * and use phase-distortion modules between them for complex timbres.
 * 
 * The `shape` parameter shifts the peak position of a triangle wave,
 * smoothly morphing between waveforms by adjusting attack/release time:
 * - **0** — Saw (all rise, instant drop)
 * - **2.5** — Triangle (symmetric)
 * - **5** — Ramp (instant rise, all fall)
 * 
 * Output range is **±5V**.
 * @param phase - phasor input (0–1, wraps at boundaries)
 * @param config - Configuration object
 *   - shape - waveform shape: 0=saw, 2.5=triangle, 5=ramp
 */
export function $pSaw(phase: Poly<Signal>, config?: { shape?: Poly<Signal>; id?: string }): CollectionWithRange;

/**
 * Phase-driven pulse/square oscillator with pulse width modulation.
 * 
 * Instead of a frequency input, this oscillator is driven by an external
 * phasor signal (0–1). Connect a `ramp` or other phase source to `phase`
 * and use phase-distortion modules between them for complex timbres.
 * 
 * The `width` parameter sets the duty cycle: 0 = narrow pulse,
 * 2.5 = square wave, 5 = inverted narrow pulse.
 * `pwm` is added to `width` for modulation.
 * 
 * Output range is **±5V**.
 * @param phase - phasor input (0–1, wraps at boundaries)
 * @param config - Configuration object
 *   - pwm - pulse width modulation CV — added to the width parameter
 *   - width - pulse width (0-5, 2.5 is square)
 */
export function $pPulse(phase: Poly<Signal>, config?: { pwm?: Poly<Signal>; width?: Poly<Signal>; id?: string }): CollectionWithRange;

/**
 * Noise generator with selectable color.
 * 
 * Generates random noise in one of three spectral colors:
 * - **White**: equal energy across all frequencies (bright, hissy)
 * - **Pink**: equal energy per octave (warm, balanced — good for "ocean" textures)
 * - **Brown**: steep low-frequency emphasis (deep, rumbling)
 * 
 * Output range is **±5V**.
 * 
 * ## Example
 * 
 * ```js
 * $noise("pink").out()
 * ```
 * @param color - color of the noise: white, pink, brown
 *   - `"white"` — equal energy across all frequencies
 *   - `"pink"` — rolled-off highs (−3 dB/octave), natural-sounding
 *   - `"brown"` — deep rumble (−6 dB/octave)
 * @param config - Configuration object
 */
export function $noise(color?: "white" | "pink" | "brown", config?: { id?: string }): ModuleOutputWithRange;

/**
 * Output type for $macro module.
 * Extends CollectionWithRange (default output: output)
 */
export interface MacroOutputs extends CollectionWithRange {
  /** auxiliary output — varies per engine */
  readonly aux: CollectionWithRange;
}

/**
 * Full-featured Plaits macro-oscillator with all 24 engines, LPG, and modulation routing.
 * 
 * For detailed engine descriptions and parameter behavior, see the
 * [Mutable Instruments Plaits documentation](https://pichenettes.github.io/mutable-instruments-documentation/modules/plaits/).
 * 
 * Engines (selected via `engine` param):
 * - Virtual analog VCF (classic subtractive)
 * - Phase distortion
 * - Six-op FM (3 banks)
 * - Wave terrain
 * - String machine
 * - Chiptune
 * - Virtual analog (dual oscillator)
 * - Waveshaping
 * - Two-operator FM
 * - Granular formant
 * - Harmonic/additive
 * - Wavetable
 * - Chords
 * - Vowel/speech synthesis
 * - Swarm
 * - Filtered noise
 * - Particle noise
 * - Inharmonic strings
 * - Modal resonator
 * - Analog bass drum
 * - Analog snare drum
 * - Analog hi-hat
 * @param freq - Pitch input in V/Oct (0V = C4)
 * @param engine - Synthesis engine selection
 *   - `"vaVcf"` — Virtual analog oscillator with VCF - classic subtractive synthesis
 *   - `"phaseDistortion"` — Phase distortion synthesis
 *   - `"sixOpA"` — Six-operator FM synthesis (bank A)
 *   - `"sixOpB"` — Six-operator FM synthesis (bank B)
 *   - `"sixOpC"` — Six-operator FM synthesis (bank C)
 *   - `"waveTerrain"` — Wave terrain synthesis
 *   - `"stringMachine"` — String machine emulation
 *   - `"chiptune"` — Chiptune waveforms with arpeggiator
 *   - `"virtualAnalog"` — Virtual analog dual oscillator
 *   - `"waveshaping"` — Waveshaping oscillator
 *   - `"twoOpFm"` — Two-operator FM synthesis
 *   - `"granularFormant"` — Granular formant oscillator
 *   - `"additive"` — Harmonic/additive oscillator
 *   - `"wavetable"` — Wavetable oscillator
 *   - `"chords"` — Chord generator
 *   - `"speech"` — Vowel and speech synthesis
 *   - `"swarm"` — Swarm oscillator
 *   - `"filteredNoise"` — Filtered noise
 *   - `"particleNoise"` — Particle noise
 *   - `"inharmonicString"` — Inharmonic string modeling
 *   - `"modalResonator"` — Modal resonator
 *   - `"bassDrum"` — Analog bass drum
 *   - `"snareDrum"` — Analog snare drum
 *   - `"hiHat"` — Analog hi-hat
 * @param config - Configuration object
 *   - fm - FM input (-5V to +5V) - frequency modulation
 *   - fmAmt - FM CV attenuverter (-5 to 5) - scales frequency modulation
 *   - harmonics - Harmonics parameter (-5V to +5V, bipolar, default 0V) - function varies per engine
 *   - level - Level/dynamics input (0-5V) - controls VCA/LPG
 *   - lpgColor - LPG color (0-5V) - lowpass gate filter response (low = mellow, high = bright)
 *   - lpgDecay - LPG decay (0-5V) - lowpass gate envelope decay time
 *   - morph - Morph parameter (-5V to +5V, bipolar, default 0V) - function varies per engine
 *   - morphAmt - Morph CV attenuverter (-5 to 5) - scales morph modulation
 *   - timbre - Timbre parameter (-5V to +5V, bipolar, default 0V) - function varies per engine
 *   - timbreAmt - Timbre CV attenuverter (-5 to 5) - scales timbre modulation
 *   - trigger - Trigger input - gates/triggers the internal envelope
 */
export function $macro(freq: Poly<Signal>, engine: "vaVcf" | "phaseDistortion" | "sixOpA" | "sixOpB" | "sixOpC" | "waveTerrain" | "stringMachine" | "chiptune" | "virtualAnalog" | "waveshaping" | "twoOpFm" | "granularFormant" | "additive" | "wavetable" | "chords" | "speech" | "swarm" | "filteredNoise" | "particleNoise" | "inharmonicString" | "modalResonator" | "bassDrum" | "snareDrum" | "hiHat", config?: { fm?: Poly<Signal>; fmAmt?: Poly<Signal>; harmonics?: Poly<Signal>; level?: Poly<Signal>; lpgColor?: Poly<Signal>; lpgDecay?: Poly<Signal>; morph?: Poly<Signal>; morphAmt?: Poly<Signal>; timbre?: Poly<Signal>; timbreAmt?: Poly<Signal>; trigger?: Poly<Signal>; id?: string }): MacroOutputs;

/**
 * Supersaw oscillator with multiple detuned sawtooth voices and PolyBLEP anti-aliasing.
 * 
 * Generates a classic supersaw sound by stacking multiple sawtooth oscillators
 * with symmetric detuning. Each input channel is processed by all voices,
 * creating a rich, full sound.
 * 
 * - **freq** — pitch in V/Oct (0V = C4)
 * - **voices** — number of detuned saw voices (1–16, default 5)
 * - **detune** — detune spread in semitones (default 0.18)
 * 
 * Output range is **±5V** with gain compensation for input channel count.
 * 
 * ## Example
 * 
 * ```js
 * $supersaw('c3').out()
 * $supersaw('c3', { voices: 7, detune: 0.3 }).out()
 * ```
 * @param freq - pitch in V/Oct (0V = C4)
 * @param config - Configuration object
 *   - detune - detune spread in semitones (default 0.18)
 *   - fm - FM input signal (pre-scaled by user)
 *   - fmMode - FM mode: throughZero (default), lin, or exp
 *   -     - `"throughZero"` — Through-zero FM: frequency can go negative (phase runs backward)
 *   -     - `"lin"` — Linear FM: like through-zero but frequency clamped to >= 0
 *   -     - `"exp"` — Exponential FM: modulator added to pitch in V/Oct space
 *   - voices - number of supersaw voices (1–16)
 */
export function $supersaw(freq: Poly<Signal>, config?: { detune?: Poly<Signal>; fm?: Poly<Signal>; fmMode?: "throughZero" | "lin" | "exp"; voices?: number; id?: string }): CollectionWithRange;

/**
 * A band-limited wavetable oscillator.
 * 
 * Reads a pre-built mipmap pyramid (FFT-filtered frame copies) and
 * selects a level appropriate for the playback frequency to suppress
 * aliasing. Frame position can be swept across multi-frame tables for
 * classic wavetable timbral sweeps, and an optional phase-warp `Table`
 * reshapes the read phase before sampling.
 * 
 * ## Example
 * 
 * ```js
 * $wavetable($wavs().tables.pad, 'c4').out()
 * $wavetable(wav, 'c2', { position: lfo }).out()
 * ```
 * @param wav - Loaded WAV reference containing the wavetable data.
 * @param pitch - Pitch in V/Oct (0V = C4).
 * @param position - Frame position as a signal. 0V maps to the first frame, 5V maps to
 * @param config - Configuration object
 *   - phase - Optional phase-warp table applied before sampling.
 */
export function $wavetable(wav: { channels: number; mtime?: number; path: string; type: string }, pitch: Poly<Signal>, position?: Poly<Signal>, config?: { phase?: Table; id?: string }): CollectionWithRange;

/**
 * Lowpass filter that attenuates frequencies above the cutoff point.
 * 
 * Use it to tame bright timbres, create bass-heavy sounds, or build classic
 * subtractive synth patches. Sweeping the cutoff with an envelope or LFO
 * produces the familiar filter-sweep effect.
 * 
 * - **cutoff** — set in V/Oct (0 V = C4). Accepts modulation for filter sweeps.
 * - **resonance** — boosts frequencies near the cutoff (0–5). High values
 *   produce a ringing peak; very high values cause self-oscillation.
 * 
 * ```js
 * // subtractive bass: saw through a lowpass with envelope on cutoff
 * let env = $adsr($pPulse($clock[0]), { attack: 0.01, decay: 0.3, sustain: 1, release: 0.4 })
 * $lpf($saw('c2'), env.range('200hz', '2000hz'))
 * ```
 * @param input - signal input
 * @param cutoff - cutoff frequency in V/Oct (0V = C4)
 * @param resonance - filter resonance (0-5)
 * @param config - Configuration object
 */
export function $lpf(input: Poly<Signal>, cutoff: Poly<Signal>, resonance?: Poly<Signal>, config?: { id?: string }): Collection;

/**
 * Highpass filter that attenuates frequencies below the cutoff point.
 * 
 * Use it to remove low-end rumble, thin out a sound, or create rising
 * filter effects. Pairs well with lowpass filters for isolating a
 * frequency band.
 * 
 * - **cutoff** — set in V/Oct (0 V = C4). Accepts modulation for filter sweeps.
 * - **resonance** — boosts frequencies near the cutoff (0–5). High values
 *   produce a ringing peak.
 * 
 * ```js
 * // remove low end from a noise source
 * $hpf($noise("white"), 'a3', 1)
 * ```
 * @param input - signal input
 * @param cutoff - cutoff frequency in V/Oct (0V = C4)
 * @param resonance - filter resonance (0-5)
 * @param config - Configuration object
 */
export function $hpf(input: Poly<Signal>, cutoff: Poly<Signal>, resonance?: Poly<Signal>, config?: { id?: string }): Collection;

/**
 * Bandpass filter that passes frequencies near the center frequency and
 * attenuates everything else.
 * 
 * Use it to isolate a frequency region, create vowel-like tones, or
 * build resonant "wah" effects by sweeping the center frequency.
 * 
 * - **center** — center frequency in V/Oct (0 V = C4).
 * - **resonance** — controls bandwidth (0–5). Higher values narrow the
 *   passband for a more pronounced, ringing sound.
 * 
 * ```js
 * // resonant bandpass sweep on noise
 * $bpf($noise("white"), $sine('0.5hz').range('440hz', '1200hz'), 3)
 * ```
 * @param input - signal input
 * @param center - center frequency in V/Oct (0V = C4)
 * @param resonance - filter resonance — controls bandwidth (0–5)
 * @param config - Configuration object
 */
export function $bpf(input: Poly<Signal>, center: Poly<Signal>, resonance?: Poly<Signal>, config?: { id?: string }): Collection;

/**
 * Output type for $jup6f module.
 * Extends CollectionWithRange (default output: output)
 */
export interface Jup6fOutputs extends CollectionWithRange {
  /** 2-pole (12dB/oct) lowpass */
  readonly lp12: CollectionWithRange;
  /** bandpass */
  readonly bp: CollectionWithRange;
  /** highpass */
  readonly hp: CollectionWithRange;
}

/**
 * Jupiter-6–style multimode ladder filter.
 * 
 * Models the IR3109 4-pole OTA cascade found in the Roland Jupiter-6.
 * Each stage applies `tanh` saturation for the warm, harmonically rich
 * character of the original analog circuit. Resonance drives the feedback
 * path and self-oscillates at high values.
 * 
 * The default output is a 24 dB/oct lowpass. Additional taps provide
 * 12 dB/oct lowpass, bandpass, and highpass responses derived from the
 * same ladder core.
 * 
 * - **cutoff** — set in V/Oct (0 V = C4). Accepts modulation for filter sweeps.
 * - **resonance** — feedback amount (0–5). Above ~4 the filter self-oscillates,
 *   producing a clean sine at the cutoff frequency.
 * 
 * ```js
 * // classic Jupiter pad: saw through the ladder with slow envelope
 * let env = $adsr($pPulse($clock[0]), { attack: 0.4, decay: 0.6, sustain: 0.3, release: 1.0 })
 * $jup6f($saw('c2'), env.range('200hz', '4000hz'), 2.5)
 * ```
 * @param input - signal input
 * @param cutoff - cutoff frequency in V/Oct (0V = C4)
 * @param resonance - filter resonance (0-5). High values produce self-oscillation.
 * @param config - Configuration object
 */
export function $jup6f(input: Poly<Signal>, cutoff: Poly<Signal>, resonance?: Poly<Signal>, config?: { id?: string }): Jup6fOutputs;

/**
 * Phase effect: digital bit-crush distortion.
 * 
 * Transforms a 0–1 phase signal by quantizing it into coarse steps,
 * creating glitchy, staircase-like phase patterns. Feed the output into
 * a phase oscillator (`$pSine`, `$pSaw`, `$pPulse`) to hear the result
 * as gritty digital artifacts — fractured harmonics at low settings,
 * aggressive bit-reduction at high settings.
 * 
 * # Example
 * 
 * ```js
 * // Bit-crush a ramp phase and convert to audio with $pSine
 * $pSine($crush($ramp('c3'), 2)).out()
 * ```
 * @param input - input phase (0 to 1)
 * @param amount - crush amount (0-5, where 0 = clean, 5 = maximum distortion)
 * @param config - Configuration object
 */
export function $crush(input: Poly<Signal>, amount?: Poly<Signal>, config?: { id?: string }): CollectionWithRange;

/**
 * Phase effect: FM feedback distortion.
 * 
 * Transforms a 0–1 phase signal by feeding the output back into itself,
 * progressively adding harmonic complexity and chaotic motion. Feed the
 * result into a phase oscillator (`$pSine`, `$pSaw`, `$pPulse`) to hear
 * the effect. At low amounts the timbre gains subtle overtones; at high
 * amounts it becomes chaotic and noisy.
 * 
 * # Example
 * 
 * ```js
 * // Apply feedback distortion to a ramp phase and convert to audio
 * $pSine($feedback($ramp('c3'), 3)).out()
 * ```
 * @param input - input phase (0 to 1)
 * @param amount - feedback amount (0-5, where 0 = no feedback, 5 = maximum feedback FM)
 * @param config - Configuration object
 *   - freq - pitch in V/Oct (optional, reduces aliasing at high frequencies)
 */
export function $feedback(input: Poly<Signal>, amount?: Poly<Signal>, config?: { freq?: Poly<Signal>; id?: string }): CollectionWithRange;

/**
 * Phase effect: pulsar synthesis distortion.
 * 
 * Transforms a 0–1 phase signal by compressing the active portion of each
 * cycle into a narrower window, leaving the rest silent. Feed the output
 * into a phase oscillator (`$pSine`, `$pSaw`, `$pPulse`) to hear pulsed
 * waveforms — at higher amounts the pulse becomes extremely narrow,
 * producing bright, impulse-like timbres useful for excitation signals
 * and metallic tones.
 * 
 * # Example
 * 
 * ```js
 * // Compress the phase with pulsar and convert to audio
 * $pSine($pulsar($ramp('c3'), 3)).out()
 * ```
 * @param input - input phase (0 to 1)
 * @param amount - compression amount (0-5, where 0 = no compression, 5 = maximum compression)
 * @param config - Configuration object
 *   - freq - pitch in V/Oct (optional, reduces aliasing at high frequencies)
 */
export function $pulsar(input: Poly<Signal>, amount?: Poly<Signal>, config?: { freq?: Poly<Signal>; id?: string }): CollectionWithRange;

/**
 * Phase ramp generator.
 * 
 * Produces a rising sawtooth phase signal from 0 to 1 at the given frequency.
 * This is the fundamental building block for phase-based synthesis:
 * feed its output into phase-distortion modules (crush, feedback, pulsar)
 * and then into a waveshaper (e.g. `$pSine`) to produce audio.
 * @param freq - pitch in V/Oct (0V = C4)
 * @param config - Configuration object
 */
export function $ramp(freq: Poly<Signal>, config?: { id?: string }): CollectionWithRange;

/**
 * An Attack-Decay-Sustain-Release envelope generator.
 * 
 * Generates a control voltage envelope driven by a **gate** input.
 * When the gate goes high (>1V) the envelope enters the attack phase;
 * when the gate goes low it enters release.
 * 
 * - **attack** / **decay** / **release** — time in seconds
 * - **sustain** — level in volts (0–5V)
 * 
 * Output range is **0–5V**.
 * 
 * ## Example
 * 
 * ```js
 * const env = $adsr($pPulse($clock[0]), { attack: 0.01, decay: 0.2, sustain: 3, release: 0.5 })
 * $sine('c4').amplitude(env).out()
 * ```
 * @param gate - gate input — rising edge starts the envelope, falling edge triggers release
 * @param config - Configuration object
 *   - attack - attack time in seconds
 *   - decay - decay time in seconds
 *   - release - release time in seconds
 *   - sustain - sustain level in volts (0-5)
 */
export function $adsr(gate: Poly<Signal>, config?: { attack?: Poly<Signal>; decay?: Poly<Signal>; release?: Poly<Signal>; sustain?: Poly<Signal>; id?: string }): CollectionWithRange;

/**
 * Reads a sample frame from a buffer and outputs its channels as a poly signal.
 * @param buffer
 * @param frame - read position (0 to 5V scales to 0 to buffer length)
 * @param config - Configuration object
 */
export function $bufRead(buffer: BufferOutputRef, frame: Mono<Signal>, config?: { id?: string }): Collection;

/**
 * Constrains a signal between a minimum and maximum value.
 * 
 * Bounds are independently optional — omit **min** or **max** to leave
 * that side unclamped.
 * 
 * ```js
 * // clamp a sine into the 0–5 V range
 * $clamp($sine('440hz'), 0, 5)
 * 
 * // one-sided: floor at 0 V, no ceiling
 * $clamp(signal, { min: 0 })
 * ```
 * @param input - signal to clamp
 * @param config - Configuration object
 *   - max - upper bound — if omitted the signal is unclamped above
 *   - min - lower bound — if omitted the signal is unclamped below
 */
export function $clamp(input: Poly<Signal>, config?: { max?: Poly<Signal>; min?: Poly<Signal>; id?: string }): Collection;

/**
 * Divides an incoming clock signal so it fires less often.
 * 
 * Feed it a clock and set **division** to an integer — the output will
 * tick once every *n* input ticks. Useful for creating slower rhythmic
 * subdivisions from a master clock.
 * 
 * ```js
 * // Pulses every other bar of the root clock:
 * $clockDivider($clock.barTrigger, 2)
 * ```
 * @param input - clock signal to divide
 * @param division - division factor (e.g. 2 = output fires every other tick)
 * @param config - Configuration object
 *   - reset - trigger to reset the counter to 0
 */
export function $clockDivider(input: Poly<Signal>, division: number, config?: { reset?: Poly<Signal>; id?: string }): Collection;

/**
 * Applies a power curve to a signal, normalised at ±5 V.
 * 
 * Formula: `sign(x) × 5 × (|x| / 5) ^ exp`
 * 
 * - **exp = 1** — linear pass-through
 * - **exp > 1** — pushes midrange toward zero (audio taper)
 * - **0 < exp < 1** — pushes midrange toward ±5 V
 * - **exp = 0** — step function (any nonzero → ±5 V)
 * 
 * ```js
 * $curve(lfo, 2)       // quadratic curve
 * $curve(signal, 3)    // cubic curve (audio taper)
 * ```
 * @param input - signal to apply curve to
 * @param exp - exponent for the power curve (0 = step, 1 = linear, >1 = audio taper)
 * @param config - Configuration object
 */
export function $curve(input: Poly<Signal>, exp: Poly<Signal>, config?: { id?: string }): Collection;

/**
 * Reads a signal from a buffer at a specified delay time relative to the write position.
 * @param buffer
 * @param time - Delay time in seconds (e.g. 0.5 for 500ms)
 * @param config - Configuration object
 */
export function $delayRead(buffer: BufferOutputRef, time: Poly<Signal>, config?: { id?: string }): Collection;

/**
 * Slew limiter that smooths abrupt voltage changes.
 * 
 * Separate **rise** and **fall** times control how quickly the output can
 * increase or decrease. Times are specified in **seconds per volt** — the
 * time the output takes to slew by 1 V. For example, a rise time of `0.1`
 * means the output climbs 1 V in 0.1 s; a 5 V gate signal would therefore
 * take 0.5 s to reach full height.
 * 
 * Use `$slew` to add portamento to pitch signals, smooth noisy control
 * voltages, or create envelope-like shapes from gate signals.
 * 
 * ```js
 * // portamento: glide between notes (0.1 s per volt of pitch change)
 * $sine($slew(sequencer.pitch, { rise: 0.1, fall: 0.1 }))
 * ```
 * @param input - signal input
 * @param config - Configuration object
 *   - fall - fall rate — seconds to slew 1 volt downward (default 0.01)
 *   - rise - rise rate — seconds to slew 1 volt upward (default 0.01)
 */
export function $slew(input: Poly<Signal>, config?: { fall?: Poly<Signal>; rise?: Poly<Signal>; id?: string }): Collection;

/**
 * Detects rising edges in a signal and emits a short pulse.
 * 
 * Outputs 5 V for a single sample whenever the input increases.
 * Useful for converting ramps, envelopes, or continuous signals into
 * gate/trigger events.
 * 
 * ```js
 * // trigger a percussion envelope on every rising edge of a slow oscillator
 * $perc($rising($sine('4hz')))
 * ```
 * @param input - signal to detect rising edges in
 * @param config - Configuration object
 */
export function $rising(input: Poly<Signal>, config?: { id?: string }): Collection;

/**
 * Detects falling edges in a signal and emits a short pulse.
 * 
 * Outputs 5 V for a single sample whenever the input decreases.
 * Useful for triggering events on the "off" transition of a gate or
 * on the downward slope of an LFO.
 * 
 * ```js
 * // trigger on every falling edge of a gate
 * $perc($falling(gate))
 * ```
 * @param input - signal to detect falling edges in
 * @param config - Configuration object
 */
export function $falling(input: Poly<Signal>, config?: { id?: string }): Collection;

/**
 * Evaluates a math expression every sample, giving you arbitrary control
 * voltage transformations.
 * 
 * Write an expression string using `x`, `y`, `z` as input variables.
 * The built-in variable `t` (time in seconds) is also available.
 * 
 * **Functions:** `sin`, `cos`, `tan`, `asin`, `acos`, `atan`,
 * `sinh`, `cosh`, `tanh`, `asinh`, `acosh`, `atanh`,
 * `log(base?, val)`, `abs`, `sign`, `int`, `ceil`, `floor`,
 * `round(modulus?, val)`, `min(val, ...)`, `max(val, ...)`,
 * `e()`, `pi()`, `vToHz(volts)`, `hzToV(hz)`
 * 
 * **Operators** (highest to lowest precedence):
 * `^`, `%`, `/`, `*`, `-`, `+`,
 * `== != < <= >= >`,
 * `&& and`, `|| or`
 * 
 * ```js
 * // crossfade between two oscillators
 * $math("x * sin(t) + y * cos(t)", { x: $saw('c3'), y: $pulse('c3') })
 * ```
 * @param expression - math expression to evaluate (e.g. "x * 2 + sin(t)")
 * @param config - Configuration object
 *   - x - first input variable, referenced as `x` in the expression
 *   - y - second input variable, referenced as `y` in the expression
 *   - z - third input variable, referenced as `z` in the expression
 */
export function $math(expression: string, config?: { x?: Mono<Signal>; y?: Mono<Signal>; z?: Mono<Signal>; id?: string }): ModuleOutput;

/**
 * Linearly rescales a signal from one voltage range to another.
 * 
 * Maps **input** from \[inMin, inMax\] to \[outMin, outMax\]. Useful for
 * converting between different voltage standards or reshaping control
 * signals.
 * 
 * ```js
 * // convert a 0–5 V envelope to a -5–5 V bipolar signal
 * $remap(env, -5, 5, 0, 5)
 * 
 * // convert a -5–5 V signal to 0–1 V
 * $remap(signal, 0, 1, -5, 5)
 * ```
 * @param input - signal input to remap
 * @param outMin - minimum of output range
 * @param outMax - maximum of output range
 * @param inMin - minimum of input range
 * @param inMax - maximum of input range
 * @param config - Configuration object
 */
export function $remap(input: Poly<Signal>, outMin: Poly<Signal>, outMax: Poly<Signal>, inMin: Poly<Signal>, inMax: Poly<Signal>, config?: { id?: string }): Collection;

/**
 * Captures and holds a voltage on each trigger.
 * 
 * When **trigger** receives a rising edge, the current value of **input**
 * is sampled and held at the output until the next trigger. Classic
 * use: sample random noise to generate stepped random melodies.
 * 
 * ```js
 * // stepped random melody
 * $sine(
 *  $quantizer(
 *    $sah($noise('white').range(0, 1), $pulse('2hz')),
 *    0,
 *    'c(maj)',
 *  ),
 * )
 * ```
 * @param input - signal to sample
 * @param trigger - rising edge captures the current input value
 * @param config - Configuration object
 */
export function $sah(input: Poly<Signal>, trigger: Poly<Signal>, config?: { id?: string }): Collection;

/**
 * Follows the input while the gate is low, and holds the value when the
 * gate goes high.
 * 
 * The complement of Sample and Hold: the output continuously tracks
 * **input** until **gate** rises, then freezes until the gate falls again.
 * 
 * ```js
 * // hold a slow sine value while the gate is high
 * $tah($sine('2hz'), gate)
 * ```
 * @param input - signal to track
 * @param gate - while gate is low the output follows the input; when gate goes high the last value is held
 * @param config - Configuration object
 */
export function $tah(input: Poly<Signal>, gate: Poly<Signal>, config?: { id?: string }): Collection;

/**
 * Simple envelope for percussive sounds.
 * 
 * A rising edge on **trigger** starts the envelope at 5 V, which then
 * decays exponentially to 0 V over the **decay** time. Perfect for
 * hi-hats, kicks, and other transient sounds.
 * 
 * Output range is **0–5 V**.
 * 
 * ```js
 * // short percussive hit
 * $noise("white").mul($perc($clock.gate, { decay: 0.1 }))
 * ```
 * @param trigger - trigger input (rising edge triggers envelope)
 * @param config - Configuration object
 *   - decay - decay time in seconds
 */
export function $perc(trigger: Poly<Signal>, config?: { decay?: Poly<Signal>; id?: string }): CollectionWithRange;

/**
 * Output type for $quantizer module.
 * Extends Collection (default output: output)
 */
export interface QuantizerOutputs extends Collection {
  /** trigger pulse on note change */
  readonly trig: CollectionWithRange;
}

/**
 * Snaps a V/Oct signal to the nearest note in a given scale.
 * 
 * Feed any continuous pitch signal into **input** and choose a **scale** —
 * the output locks to the closest scale degree. A **trig** pulse fires
 * whenever the quantized note changes, useful for re-triggering envelopes.
 * 
 * Scale format examples:
 * - `"chromatic"` — all 12 semitones
 * - `"C(major)"` — C major scale
 * - `"C#(minor)"` — C# minor scale
 * - `"D(0 2 4 5 7 9 11)"` — custom intervals from root
 * 
 * ```js
 * // quantize a random signal to C major
 * $sine($quantizer($sine(".1hz").range(0,3), "C(major)"))
 * ```
 * @param input - Input V/Oct signal to quantize
 * @param scale - Scale specification: "chromatic", "C(major)", "D(0 2 4 5 7 9 11)"
 * @param config - Configuration object
 *   - offset - Offset added to input before quantization (in V/Oct)
 */
export function $quantizer(input: Poly<Signal>, scale?: string, config?: { offset?: Poly<Signal>; id?: string }): QuantizerOutputs;

/**
 * Scales and offsets a signal — the classic attenuverter + DC offset.
 * 
 * - **scale** — gain factor (0–10 V; 5 V = unity, 0 V = silence,
 *   values above 5 V amplify, negative values invert).
 * - **shift** — DC offset added after scaling (in volts).
 * 
 * ```js
 * // invert a slow sine and shift it into 0–5 V range
 * $scaleAndShift($sine('1hz'), -5, 2.5)
 * ```
 * @param input - signal to scale and shift
 * @param scale - scale factor (0–10V range; 5V = unity gain, 0V = silence, -5V = inverted, 10V = 2x)
 * @param shift - DC offset added to the scaled signal (in volts)
 * @param config - Configuration object
 */
export function $scaleAndShift(input: Poly<Signal>, scale?: Poly<Signal>, shift?: Poly<Signal>, config?: { id?: string }): Collection;

/**
 * Produces a multi-channel output by linearly interpolating between a
 * minimum and maximum value, with an optional bias to skew the
 * distribution.
 * 
 * Each output channel gets an evenly spaced value between **min** and
 * **max**. The **bias** control reshapes the distribution: positive
 * values push channels toward **max**, negative values toward **min**.
 * 
 * | count | behaviour |
 * |-------|-----------|
 * | 1     | single value between min and max, positioned by bias |
 * | 2     | channel 0 = min, channel 1 = max (with bias = 0) |
 * | N     | N evenly spaced values from min to max (with bias = 0) |
 * 
 * ```js
 * // spread 4 oscillators across a frequency range
 * $sine($spread(0, 5, 4))
 * 
 * // 8 channels biased toward the minimum
 * $spread(0, 5, 8, { bias: -3 })
 * ```
 * @param min - lower bound of the spread range
 * @param max - upper bound of the spread range
 * @param count - number of output channels (1–16)
 * @param config - Configuration object
 *   - bias - distribution bias (-5 to 5): positive biases toward max, negative toward min
 */
export function $spread(min: Mono<Signal>, max: Mono<Signal>, count: number, config?: { bias?: Mono<Signal>; id?: string }): Collection;

/**
 * Expands each input channel into multiple detuned copies for unison effects.
 * 
 * Takes a signal (typically V/Oct pitch) and multiplies channels by the
 * unison count, applying symmetric detuning controlled by the spread parameter.
 * 
 * - **count** — number of detuned copies per input channel (1–16)
 * - **spread** — detune amount with exponential curve (0–10V → 0–1 octave V/Oct)
 * 
 * Output channels = `input_channels × count`, clamped to 16.
 * 
 * ## Example
 * 
 * ```js
 * // 7-voice unison saw with moderate spread
 * $saw($unison('c4', 7, 5)).out()
 * 
 * // With modulated spread
 * $saw($unison($midiCV().pitch, 5, $sine('0.2hz'))).out()
 * ```
 * @param input - input signal to expand (typically V/Oct pitch)
 * @param count - number of unison voices per input channel (1–16)
 * @param spread - detune spread amount (0–10V, exponential: 0V = none, 10V = 1 octave)
 * @param config - Configuration object
 */
export function $unison(input: Poly<Signal>, count?: number, spread?: Poly<Signal>, config?: { id?: string }): Collection;

/**
 * Folds a signal into a range by wrapping values that exceed the boundaries
 * back from the opposite side — like a phase accumulator.
 * 
 * Both **min** and **max** accept polyphonic signals. If **max** < **min**
 * the bounds are swapped automatically.
 * 
 * ```js
 * // wrap a ramp into 0–5 V
 * $wrap(ramp, 0, 5)
 * ```
 * @param input - signal to wrap
 * @param min - lower bound of the wrap range
 * @param max - upper bound of the wrap range
 * @param config - Configuration object
 */
export function $wrap(input: Poly<Signal>, min: Poly<Signal>, max: Poly<Signal>, config?: { id?: string }): Collection;

/**
 * Output type for $cycle module.
 * Extends Collection (default output: cv)
 */
export interface CycleOutputs extends Collection {
  /** high (5 V) while a note is active, low (0 V) otherwise */
  readonly gate: CollectionWithRange;
  /** short pulse (5 V) at the start of each note */
  readonly trig: CollectionWithRange;
}

/**
 * Pattern sequencer using mini-notation strings.
 * 
 * Write rhythmic and melodic patterns using a compact text syntax ported
 * from TidalCycles/Strudel. The pattern loops each **cycle** and supports
 * polyphony — overlapping notes are automatically allocated to separate
 * output channels.
 * 
 * ## Cycles
 * 
 * A **cycle** is one full traversal of the pattern. The playhead position
 * determines timing: its integer part selects the current cycle number and
 * the fractional part selects the position within that cycle. Space-separated
 * values divide the cycle into equal time slots.
 * 
 * ## Values
 * 
 * | Syntax | Meaning | Example |
 * |--------|---------|---------|
 * | Note name | Pitch (octave defaults to 3) | `'c4'`, `'a#3'`, `'db5'` |
 * | Bare number | MIDI note number | `60`, `72` |
 * | `Xhz` | Frequency | `'440hz'` |
 * | `Xv` | Explicit voltage | `'0v'`, `'1v'`, `'-0.5v'` |
 * | `~` | Rest (gate low, no change in CV) | `'c4 ~ e4 ~'` |
 * 
 * Bare numbers are MIDI note numbers (A0 = MIDI 33 = 0 V).
 * 
 * ## Grouping
 * 
 * - **`[a b c]`** — fast subsequence: subdivides the parent time slot so all
 *   elements play within it.
 * - **`<a b c>`** — slow / alternating: plays one element per cycle,
 *   advancing each time the pattern loops.
 * 
 * ```js
 * $cycle("c4 [d4 e4]")   // c4 for half the cycle, d4 & e4 share the other half
 * $cycle("<c4 g4> e4")   // cycle 1: c4 e4, cycle 2: g4 e4, …
 * ```
 * 
 * ## Stacks
 * 
 * **`a b, c d`** — comma-separated patterns play **simultaneously** (layered).
 * Each sub-pattern has its own independent timing.
 * 
 * ```js
 * $cycle("c4 e4, g4 b4")   // two patterns layered on top of each other
 * $cycle("c4 d4 e4, g3")   // three-note melody over a pedal tone
 * ```
 * 
 * ## Random choice
 * 
 * **`a|b|c`** — randomly selects one option each time the slot is reached.
 * 
 * ```js
 * $cycle("c4|d4|e4 g4")  // first slot is a random pick each cycle
 * ```
 * 
 * ## Nesting
 * 
 * Grouping, stacks, and random choice nest arbitrarily:
 * 
 * ```js
 * $cycle("<c4 [d4 e4]> [f4|g4 a4]")  // slow + fast + random combined
 * $cycle("[c4 e4, g4] a4")            // stack inside a fast subsequence
 * ```
 * 
 * ## Per-element modifiers
 * 
 * Modifiers attach directly to an element (no spaces). Multiple modifiers
 * can be chained in any order.
 * 
 * | Modifier | Syntax | Meaning |
 * |----------|--------|---------|
 * | Weight | `@n` | Relative duration within a sequence (default 1). `c4@2 e4` gives c4 twice the time. |
 * | Speed up | `*n` | Repeat/subdivide `n` times within the slot. `c4*3` plays c4 three times. |
 * | Slow down | `/n` | Stretch over `n` cycles. `c4/2` plays every other cycle. |
 * | Replicate | `!n` | Duplicate the element `n` times (default 2). `c4!3` is equivalent to `c4 c4 c4`. |
 * | Degrade | `?` or `?n` | Randomly drop the element. `c4?` drops ~50 % of the time; `c4?0.8` drops 80 %. |
 * | Euclidean | `(k,n)` or `(k,n,offset)` | Distribute `k` pulses over `n` steps using the Bjorklund algorithm. Optional `offset` rotates the pattern. |
 * 
 * ```js
 * $cycle("c4*2 e4 g4")        // c4 plays twice in its slot
 * $cycle("c4@3 e4 g4")        // c4 gets 3/5 of the cycle, e4 and g4 get 1/5 each
 * $cycle("c4? e4 g4")         // c4 randomly drops out ~50 % of the time
 * $cycle("c4(3,8) e4")        // Euclidean: 3 hits spread over 8 steps
 * $cycle("[c4 d4 e4 f4](3,8)") // Euclidean applied to a subpattern
 * ```
 * 
 * Modifier operands can also be subpatterns: `c4*[2 3]` alternates between
 * doubling and tripling each slot.
 * 
 * ## Outputs
 * 
 * - **cv** — V/Oct pitch (C4 = 0 V).
 * - **gate** — 5 V while a note is active, 0 V otherwise.
 * - **trig** — single-sample 5 V pulse at each note onset.
 * @param pattern - pattern string in mini-notation
 * @param config - Configuration object
 *   - channels - Number of polyphonic voices (1-16)
 *   - playhead - playhead position (driven by the global clock)
 */
export function $cycle(pattern: string, config?: { channels?: number; playhead?: Mono<Signal>; id?: string }): CycleOutputs;

/**
 * Automation track that interpolates between keyframed values.
 * 
 * Place keyframes at positions within a cycle (0–1) and the track
 * smoothly interpolates between them at the current playhead position according to the chosen
 * **interpolationType** (linear, ease-in, ease-out, etc.).
 * 
 * ```js
 * // automate filter cutoff over one cycle
 * $lpf(osc, $track([['c2', 0], ['c5', 0.5], ['c3', 1]]))
 * ```
 * @param keyframes - keyframe values and their positions (0–1)
 * @param config - Configuration object
 *   - interpolationType - interpolation curve between keyframes
 *   - playhead - playhead position (wraps from 0 to 1)
 */
export function $track(keyframes: [Poly<Signal>, number][], config?: { interpolationType?: "linear" | "step" | "sineIn" | "sineOut" | "sineInOut" | "quadIn" | "quadOut" | "quadInOut" | "cubicIn" | "cubicOut" | "cubicInOut" | "quartIn" | "quartOut" | "quartInOut" | "quintIn" | "quintOut" | "quintInOut" | "expoIn" | "expoOut" | "expoInOut" | "circIn" | "circOut" | "circInOut" | "bounceIn" | "bounceOut" | "bounceInOut"; playhead?: Mono<Signal>; id?: string }): Collection;

/**
 * Output type for $iCycle module.
 * Extends Collection (default output: cv)
 */
export interface ICycleOutputs extends Collection {
  /** high (5 V) while a note is active, low (0 V) otherwise */
  readonly gate: CollectionWithRange;
  /** short pulse (5 V) at the start of each note */
  readonly trig: CollectionWithRange;
}

/**
 * Scale-degree sequencer using a compact text syntax ported
 * from TidalCycles/Strudel.
 * 
 * Works with **scale degree numbers** instead of note names. One or more
 * **patterns** are combined by recursively folding the patterns into each other.
 * This is adapted from the default way that patterns are combined in Strudel:
 * 2 patterns are aligned in a cycle and the events of the second pattern are applied to the first.
 * Here this happens recursively (where n pattern is applied to n-1), adding
 * the values of those patterns' events together. The result is a single combined
 * pattern of scale degrees that can be sampled at the current playhead position to produce output CV/gate/trig.
 * Scale degrees outside the configured **scale** are automatically wrapped into the appropriate octave.
 * 
 * ## Cycles
 * 
 * A **cycle** is one full traversal of a pattern. The playhead position
 * determines timing: its integer part selects the current cycle number and
 * the fractional part selects the position within that cycle.
 * All patterns share the same cycle clock.
 * 
 * ## Scale degrees
 * 
 * Values are **0-indexed** degrees of the chosen scale. `0` is the root,
 * `1` is the second scale tone, `2` the third, and so on. Negative values
 * move downward; values beyond the scale length wrap into higher/lower
 * octaves automatically.
 * 
 * ## Mini-notation
 * 
 * | Syntax | Meaning | Example |
 * |--------|---------|---------|
 * | Bare number | Scale degree (0-indexed) | `0`, `2`, `4` |
 * | `~` | Rest (gate low, no change in pitch) | `'0 ~ 2 ~'` |
 * | `[a b c]` | Fast subsequence — subdivides parent time slot | `'[0 2 4]'` |
 * | `<a b c>` | Slow / alternating — one element per cycle | `'<0 4 7>'` |
 * | `a\|b\|c` | Random choice each time the slot is reached | `'0\|2\|4'` |
 * | `a, b` | Stack — comma-separated patterns play simultaneously | `'0 2, 4 7'` |
 * 
 * Grouping, stacks, and random choice nest arbitrarily.
 * 
 * ## Per-element modifiers
 * 
 * Modifiers attach directly to an element (no spaces). Multiple modifiers
 * can be chained in any order.
 * 
 * | Modifier | Syntax | Meaning |
 * |----------|--------|---------|
 * | Weight | `@n` | Relative duration within a sequence (default 1). `0@2 2` gives `0` twice the time. |
 * | Speed up | `*n` | Repeat/subdivide `n` times within the slot. `0*3` plays degree 0 three times. |
 * | Slow down | `/n` | Stretch over `n` cycles. `0/2` plays every other cycle. |
 * | Replicate | `!n` | Duplicate the element `n` times (default 2). `0!3` is equivalent to `0 0 0`. |
 * | Degrade | `?` or `?n` | Randomly drop the element. `0?` drops ~50 % of the time; `0?0.8` drops 80 %. |
 * | Euclidean | `(k,n)` or `(k,n,offset)` | Distribute `k` pulses over `n` steps (Bjorklund algorithm). |
 * 
 * Modifier operands can also be subpatterns: `0*[2 3]` alternates between
 * doubling and tripling each slot.
 * 
 * ## Polyphony
 * 
 * The first pattern's structure is preserved. When subsequent patterns
 * contain stacks (simultaneous events), one combined
 * event is created per left×right pair, all sharing the first pattern's timing. This
 * can create polyphonic output.
 * 
 * ```js
 * // first pattern: one note per slot
 * // second pattern: two simultaneous offsets → two voices per slot
 * $iCycle(["0 2 4", "0,4"], "c4(major)")
 * ```
 * 
 * ```js
 * // slow alternation in second pattern shifts the chord each cycle
 * $iCycle(["0,2,4", "<0 3>"], "c4(major)")
 * ```
 * 
 * ## Outputs
 * 
 * - **cv** — V/Oct pitch quantized to the scale (C4 = 0 V).
 * - **gate** — 5 V while a note is active, 0 V otherwise.
 * - **trig** — single-sample 5 V pulse at each note onset.
 * @param patterns - patterns to combine (left-fold with appLeft addition); accepts a single
 * @param scale - scale for quantizing degrees to pitches (supports optional octave, e.g. "c3(major)")
 * @param config - Configuration object
 *   - channels - number of polyphonic voices (1–16)
 *   - playhead - playhead position
 */
export function $iCycle(patterns: string | string[], scale: string, config?: { channels?: number; playhead?: Mono<Signal>; id?: string }): ICycleOutputs;

/**
 * Step sequencer
 * @param steps - Steps of the sequence
 * @param next - Next step trigger
 * @param config - Configuration object
 *   - reset - Reset trigger
 */
export function $step(steps: Poly<Signal>[], next: Mono<Signal>, config?: { reset?: Mono<Signal>; id?: string }): Collection;

/**
 * Output type for $midiCV module.
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
  /** pitch wheel position (-5V to +5V) */
  readonly pitchWheel: CollectionWithRange;
  /** mod wheel (0-5V) */
  readonly modWheel: CollectionWithRange;
}

/**
 * Converts MIDI note input into control voltages for driving synthesizer modules.
 * Supports polyphonic voice allocation with up to 16 voices.
 * 
 * ## Outputs
 * 
 * | Output | Signal | Range |
 * |---|---|---|
 * | `pitch` | V/Oct pitch (0V = C4), includes pitch bend | — |
 * | `gate` | High while a note is held | 0–5V |
 * | `velocity` | Note-on velocity | 0–5V |
 * | `aftertouch` | Channel or polyphonic aftertouch | 0–5V |
 * | `retrigger` | 5V pulse for 1ms on each new note | 0–5V |
 * | `pitchWheel` | Pitch bend wheel position | -5V–+5V |
 * | `modWheel` | Mod wheel (CC 1) | 0–5V |
 * 
 * ## Polyphonic modes (`polyMode`)
 * 
 * - `"rotate"` — cycles through voices round-robin
 * - `"reuse"` — reuses the voice that last played the same note
 * - `"reset"` — always starts allocation from voice 1
 * - `"mpe"` — one voice per MIDI channel (for MPE controllers)
 * 
 * ## Monophonic modes (`monoMode`, when `channels: 1`)
 * 
 * - `"last"` — most recently pressed note wins
 * - `"first"` — first pressed note wins
 * - `"lowest"` — lowest held note wins
 * - `"highest"` — highest held note wins
 * 
 * ## Example
 * 
 * ```js
 * // 4-voice polyphonic MIDI synth
 * const midi = $midiCV({ channels: 4 });
 * $sine(midi.pitch).amplitude($adsr(midi.gate,{attack: 0.01, decay: 0.1, sustain: 0.8, release: 0.5})).out();
 * 
 * // Mono lead with pitch bend
 * const lead = $midiCV({ channels: 1, pitchBendRange: 12 });
 * $saw(lead.pitch).amplitude($adsr(lead.gate,{attack: 0.01, decay: 0.1, sustain: 0.8, release: 0.5})).out();
 * ```
 * @param config - Configuration object
 *   - channel - MIDI channel to listen on (1–16, leave unset for omni/all channels)
 *   - channels - Number of polyphonic voices (1-16)
 *   - device - MIDI device name to receive from (leave unset to receive from all devices)
 *   - monoMode - Monophonic note priority (used when only one voice is active)
 *   -     - `"last"` — Last note pressed wins
 *   -     - `"first"` — First note pressed wins (ignores new notes)
 *   -     - `"lowest"` — Lowest pitch note wins
 *   -     - `"highest"` — Highest pitch note wins
 *   - pitchBendRange - Pitch bend range in semitones (0 = disabled, default 2)
 *   - polyMode - Polyphonic voice allocation mode
 *   -     - `"rotate"` — Round-robin through available voices
 *   -     - `"reuse"` — Reuse voice playing same note before rotating
 *   -     - `"reset"` — Always start from the first voice
 *   -     - `"mpe"` — MPE mode: MIDI channel maps directly to output channel
 */
export function $midiCV(config?: { channel?: number; channels?: number; device?: string; monoMode?: "last" | "first" | "lowest" | "highest"; pitchBendRange?: number; polyMode?: "rotate" | "reuse" | "reset" | "mpe"; id?: string }): MidiCVOutputs;

/**
 * Converts a MIDI continuous controller (CC) message into a smooth control voltage (0–5V).
 * 
 * Use the `cc` parameter to select which CC number to listen to (0–127).
 * Optional `smoothingMs` applies a slew filter to reduce stepping artifacts.
 * Enable `highResolution` for 14-bit CC precision (pairs CC 0–31 with 32–63).
 * 
 * ## Example
 * 
 * ```js
 * // Use mod wheel (CC 1) to sweep a filter cutoff
 * const mod = $midiCC({ cc: 1 });
 * $lpf($saw('c3'), mod.range('c2', 'c6')).out();
 * 
 * // High-resolution breath controller with smoothing
 * const breath = $midiCC({ cc: 2, highResolution: true, smoothingMs: 10 });
 * $sine('c4').amplitude(breath).out();
 * ```
 * @param config - Configuration object
 *   - cc - CC number to monitor (0-127 for 7-bit, 0-31 for 14-bit mode)
 *   - channel - MIDI channel to listen on (1–16, leave unset for omni/all channels)
 *   - device - MIDI device name to receive from (leave unset to receive from all devices)
 *   - highResolution - Enable 14-bit high-resolution CC mode (CC 0-31 MSB + CC 32-63 LSB)
 *   - smoothingMs - Smoothing time in milliseconds (0 = instant)
 */
export function $midiCC(config: { cc: number; channel?: number; device?: string; highResolution?: boolean; smoothingMs?: number; id?: string }): ModuleOutputWithRange;

/**
 * One-shot sample player. Plays a loaded WAV file from the beginning on each
 * gate rising edge. Speed control allows pitch-shifting and reverse playback.
 * 
 * ```js
 * $sampler($wavs().kick, $pulse('4hz'))
 * $sampler($wavs().tables.pad, $clock.beat, { speed: 0.5 })
 * ```
 * @param wav
 * @param gate - Gate input — rising edge starts playback from the beginning.
 * @param config - Configuration object
 *   - speed - Playback speed. 1.0 = normal, 2.0 = double speed, negative = reverse.
 */
export function $sampler(wav: { channels: number; mtime?: number; path: string; type: string }, gate: Mono<Signal>, config?: { speed?: Mono<Signal>; id?: string }): Collection;

/**
 * Output type for _clock module.
 * Extends Collection (default output: playhead)
 */
export interface _clockOutputs extends Collection {
  /** 5V trigger at the start of each bar */
  readonly barTrigger: ModuleOutputWithRange;
  /** 5V trigger at the start of each beat */
  readonly beatTrigger: ModuleOutputWithRange;
  /** 0..5V ramp that resets every bar */
  readonly ramp: ModuleOutputWithRange;
  /** 5V trigger at 48 pulses per quarter note */
  readonly ppqTrigger: ModuleOutputWithRange;
  /** Current beat within the bar (0-indexed) */
  readonly beatInBar: ModuleOutput;
}

/** Global clock module running at 120 BPM by default. */
export const $clock: _clockOutputs;

/** Input signals. */
export const $input: Readonly<Collection>;

/** Create a buffer module that captures an input signal into a circular audio buffer. */
export function $buffer(input: ModuleOutput | Collection | number, lengthSeconds: number, config?: { id?: string }): BufferOutputRef;


/** Load WAV samples from the wavs/ folder. */
export function $wavs(): Record<string, never>;

}

export {};
