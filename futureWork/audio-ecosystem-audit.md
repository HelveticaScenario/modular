# Audio Ecosystem Audit — Portable Code, Algorithms & Ideas

Comprehensive audit of `~/dev/audio/` — 12+ projects spanning DSP engines, live-coding
environments, plugin frameworks, and hardware firmware. Organized by what's most directly
useful for Operator.

---

## Table of Contents

1. [Portability Matrix](#portability-matrix)
2. [Tier 1 — Drop-In Portable Code](#tier-1--drop-in-portable-code)
3. [Tier 2 — Portable With Adaptation](#tier-2--portable-with-adaptation)
4. [Tier 3 — Architectural Inspiration](#tier-3--architectural-inspiration)
5. [Tier 4 — Language & DSL Design Ideas](#tier-4--language--dsl-design-ideas)
6. [Strudel Branch Analysis](#strudel-branch-analysis)
7. [Bug Fixes & Lessons Learned](#bug-fixes--lessons-learned)
8. [Source Index](#source-index)

---

## Portability Matrix

| Code                            | Source                  | Lines | Deps     | Port Effort | Value |
| ------------------------------- | ----------------------- | ----- | -------- | ----------- | ----- |
| `sin_2pi_9()` polynomial sine   | VCV Fundamental VCO.cpp | 6     | None     | Trivial     | High  |
| `SoftclipADAA1`                 | VCV Fundamental VCF.cpp | 32    | None     | Trivial     | High  |
| `MinBlep<Z,O>` template         | VCV Fundamental VCO.cpp | ~120  | None     | Low         | High  |
| PolyBLEP/PolyBLAMP inlines      | stmlib polyblep.h       | 4 fn  | None     | Trivial     | High  |
| ZDF SVF / OnePole / DCBlocker   | stmlib filter.h         | ~200  | None     | Low         | High  |
| Delay line (int/linear/Hermite) | stmlib delay_line.h     | ~100  | None     | Low         | High  |
| TPT Ladder Filter w/ ADAA       | VCV Fundamental VCF.cpp | ~200  | ADAA     | Low         | High  |
| `OnePoleLowpass` (3 methods)    | VCV Fundamental VCO.cpp | ~40   | None     | Trivial     | Med   |
| `rcp_newton1` / `rsqrt_newton1` | VCV Fundamental VCF.cpp | 8     | None     | Trivial     | Med   |
| DBAP spatial panning core       | Csound dbap.c           | ~30   | None     | Low         | High  |
| Doppler shift modulator         | BespokeSynth            | 96    | None     | Trivial     | Med   |
| ErbeVerb FDN reverb             | audioAlgorithmTests     | ~400  | None     | Medium      | High  |
| MoogLadder ZDF                  | MetaModule              | ~150  | None     | Low         | High  |
| Karplus-Strong (6-tap parallel) | MetaModule              | ~200  | None     | Low         | High  |
| Wavefold (4 types + table)      | MetaModule              | ~150  | None     | Low         | High  |
| Pitch shifter (2-grain OLA)     | MetaModule              | ~200  | None     | Low         | Med   |
| LPG (Gillet Buchla-style)       | MetaModule              | ~100  | None     | Low         | High  |
| Clouds granular engine          | eurorack/clouds         | ~800  | stmlib   | Medium      | High  |
| Rings modal resonator           | eurorack/rings          | ~600  | stmlib   | Medium      | High  |
| Rings string (Karplus+)         | eurorack/rings          | ~400  | stmlib   | Medium      | High  |
| Warps cross-mod (6 algorithms)  | eurorack/warps          | ~500  | stmlib   | Medium      | High  |
| Elements bow-string interaction | eurorack/elements       | ~300  | stmlib   | Medium      | High  |
| MetaModule bypass routing       | MetaModule info/\*.hh   | —     | —        | Pattern     | Low   |
| Lock-free work-stealing queue   | SuperCollider supernova | ~500  | Boost    | High        | High  |
| Glicol audio DAG (petgraph)     | glicol rs/synth         | ~600  | petgraph | High        | Med   |

---

## Tier 1 — Drop-In Portable Code

These can be ported to Rust with minimal effort and zero external dependencies.

### 1.1 — `sin_2pi_9()` — 9th-Degree Polynomial Sine

**Source:** `vcv/Fundamental/src/VCO.cpp`

6 lines, THD -103.7 dB. Evaluates `sin(2π·x)` for `x ∈ [0,1)` using an odd-symmetry
9th-degree minimax polynomial. Folds input to `[0, 0.25]` via quadrant reflection, then
applies 5 Horner-form coefficients.

Replaces `std::sin()` in any oscillator hot path. Operator currently uses `f32::sin()` in
several places — this would be a direct drop-in anywhere phase-to-sine conversion happens.

### 1.2 — `SoftclipADAA1` — Antiderivative Anti-Aliasing

**Source:** `vcv/Fundamental/src/VCF.cpp`

32 lines. First-order ADAA for `f(x) = x / sqrt(x² + 1)`. The antiderivative
`F(x) = sqrt(x² + 1)` is trivial, making this far cheaper than `tanh(x)` ADAA (which
requires `log(cosh(x))`). Based on Parker/Zavalishin/Le Bihan 2016.

The key insight: `x / sqrt(x² + 1)` is a near-perfect `tanh` substitute that's cheaper to
compute AND has an elementary antiderivative. The ADAA wrapper handles the `x₀ ≈ x₁` case
with a Taylor expansion fallback.

Immediately useful for any nonlinear module — waveshapers, filter feedback, saturation.

### 1.3 — PolyBLEP / PolyBLAMP

**Source:** `vcv/AudibleInstruments/eurorack/stmlib/dsp/polyblep.h`

4 inline functions, zero dependencies:

- `ThisBlepSample(t)` / `NextBlepSample(t)` — Residual for value discontinuities (saw, square)
- `ThisIntegratedBlepSample(t)` / `NextIntegratedBlepSample(t)` — For slope discontinuities (triangle)

These are the canonical implementations used by virtually every Eurorack-derived VCO.
Operator may already have these — if so, compare against these reference versions.

### 1.4 — MinBLEP via Cepstral Domain

**Source:** `vcv/Fundamental/src/VCO.cpp`

Full implementation of Eli Brandt's 2001 method:

1. Windowed sinc → FFT → log magnitude → IFFT (cepstrum)
2. Zero negative-time cepstral coefficients → FFT → exp → IFFT
3. Integrate to get minBLEP, differentiate for minBLAMP
4. Store in transposed layout for SIMD-friendly accumulation

The `MinBlep<Z,O>` template precomputes both minBLEP and minBLAMP tables at compile time
with configurable zero-crossings `Z` and oversampling factor `O`. The `VCOProcessor` applies
them per-waveform: square gets minBLEP at phase=0 and PW crossing, saw at 0.5, triangle gets
minBLAMP at 0.25/0.75, hard sync uses value discontinuity correction, soft sync uses slope
correction.

### 1.5 — ZDF State Variable Filter

**Source:** `vcv/AudibleInstruments/eurorack/stmlib/dsp/filter.h`

~200 lines covering:

- `Svf` — Full ZDF SVF (LP/BP/HP simultaneously) with 4 levels of `tan()` approximation
  (exact, rational, 3rd-order, fast). Resonance from 0 to self-oscillation.
- `NaiveSvf` — Chamberlin topology (cheaper, less stable at high freq)
- `CrossoverSvf` — Linkwitz-Riley crossover via cascaded SVF
- `DCBlocker` — Single-pole highpass at ~20 Hz

The `tan()` approximation hierarchy is particularly valuable — lets you trade accuracy for
CPU in real-time, or pick the right tradeoff per module.

### 1.6 — Template Delay Line

**Source:** `vcv/AudibleInstruments/eurorack/stmlib/dsp/delay_line.h`

~100 lines. Template class with:

- Integer-sample read
- Linear interpolation read
- Hermite interpolation read (4-point, 3rd-order)
- Built-in allpass interpolation variant
- Circular buffer with power-of-2 masking

This is the delay primitive underlying Clouds, Rings, Elements, and every Mutable reverb.

### 1.7 — `rcp_newton1` / `rsqrt_newton1`

**Source:** `vcv/Fundamental/src/VCF.cpp`

8 lines total. Hardware-approximate reciprocal and reciprocal-sqrt with one Newton-Raphson
refinement step. Uses `_mm_rcp_ss` / `_mm_rsqrt_ss` intrinsics. In Rust, these map to
`std::arch::x86_64` intrinsics or `f32::recip()` with manual refinement.

### 1.8 — Doppler Shift Modulator

**Source:** `BespokeSynth/Source/DopplerShift.cpp` (96 lines)

Simple but novel: a modulator that outputs `(c + v_receiver) / (c - v_source)` where
velocities come from input modulators. Useful as a pitch/frequency modulation source for
physical modeling or spatial audio effects.

### 1.9 — `OnePoleLowpass` with 3 Cutoff Methods

**Source:** `vcv/Fundamental/src/VCO.cpp`

Three methods for computing the one-pole coefficient from frequency:

- `matchedZ(fc, fs)` — `exp(-2π·fc/fs)`, exact
- `bilinear(fc, fs)` — Pre-warped bilinear transform
- `approx(fc, fs)` — Rational approximation, cheapest

Includes frequency response analysis comments. Useful as a reference for choosing the right
one-pole method based on accuracy requirements.

---

## Tier 2 — Portable With Adaptation

Require some restructuring but the core algorithms are clean and well-isolated.

### 2.1 — TPT Ladder Filter with ADAA Softclip

**Source:** `vcv/Fundamental/src/VCF.cpp` (~200 lines of filter logic)

4-pole transistor-ladder topology using TPT (topology-preserving transform) integration.
Each stage applies `SoftclipADAA1` for nonlinear saturation — 5 ADAA instances total (4
stages + feedback). Uses `tan_1_2()` rational approximation for the integrator coefficient.

Highpass output derived from binomial alternating sum of stage outputs:
`hp = x - 4·s₁ + 6·s₂ - 4·s₃ + s₄`. This is a known technique but rarely seen implemented
this cleanly.

The combination of TPT + ADAA softclip is state-of-the-art for virtual analog ladder filters.

### 2.2 — DBAP — Distance-Based Amplitude Panning

**Source:** `csound/Opcodes/dbap.c` (~819 lines total, core solver ~30 lines)

Implementation of Lossius/Baltazar/de la Hogue 2009 ICMC paper. Works in 2D or 3D. Core
algorithm:

1. For each speaker, compute `distance = sqrt(Σ(source_pos - speaker_pos)²)`
2. Apply spatial blur: `d_eff = sqrt(d² + blur²)`
3. Gain = `weight / d_eff^rolloff`
4. Normalize gains to preserve energy

Additional features: per-speaker weight factors, spread parameter (distributes source
across nearby speakers), quickselect median for normalization reference, gain interpolation
between audio frames to prevent zipper noise.

This would be novel in the modular synth context — spatial panning beyond simple stereo.

### 2.3 — MoogLadder ZDF (Faust-Generated)

**Source:** `metamodule-core-modules/4ms/core/` (Faust → C++)

Zero-delay-feedback Moog ladder. Faust generates the topology-preserving implementation
automatically. The output is verbose but the algorithm is proven correct by the Faust
compiler's signal flow analysis.

### 2.4 — Karplus-Strong (6-Tap Parallel)

**Source:** `metamodule-core-modules/4ms/core/`

6 parallel delay lines with individual damping, mixed to stereo output. Each tap has its own
pitch, feedback, and lowpass coefficients. More like a resonator bank than a single string —
useful for metallic/bell/chime textures.

### 2.5 — Wavefolder (4 Types with Table Interpolation)

**Source:** `metamodule-core-modules/4ms/core/`

Four wavefold algorithms:

1. Triangle fold (classic Buchla/Serge style)
2. Sine fold
3. Soft saturation fold
4. Hard clip fold

Each uses wavetable lookup with linear interpolation for the transfer function. The table
approach means you can add arbitrary waveshaping curves by just providing a new table.

### 2.6 — LPG — Buchla-Style Low Pass Gate

**Source:** `metamodule-core-modules/4ms/core/` (based on Emilie Gillet's design)

Combined VCA + VCF that responds to control voltage with vactrol-like response curves.
Attempt to model the natural decay characteristics of a vactrol — fast attack, slow
logarithmic release with frequency-dependent behavior. The filter opens and amplifies
simultaneously, which is the defining characteristic of a true LPG vs. a VCF+VCA chain.

### 2.7 — ErbeVerb FDN Reverb

**Source:** `audioAlgorithmTests/erbe/src/main.rs` (~400 lines, user's own code)

4-channel FDN (Feedback Delay Network) reverb with:

- Hadamard feedback matrix (energy-preserving, maximally diffuse)
- Allpass diffusion chains per channel
- Granular modulation (4 grains with raised-cosine windows)
- Chebyshev saturation in the feedback path
- Auto-decay compressor to prevent runaway feedback

**Known bugs to fix if porting:**

- Stereo EQ filter state bleed (left/right share biquad state)
- No parameter smoothing (zipper noise on knob changes)
- Linear interpolation on delay reads — should be Hermite for reverb quality

### 2.8 — Pitch Shifter (2-Grain Overlap-Add)

**Source:** `metamodule-core-modules/4ms/core/`

Two grains with complementary Hann windows, each reading from a circular delay buffer at a
different rate. The rate difference between write and read pointers determines pitch shift.
Classic time-domain pitch shifting — simple, predictable, works well within ±1 octave.

---

## Tier 3 — Architectural Inspiration

Larger systems that offer design patterns and ideas rather than drop-in code.

### 3.1 — Clouds Granular Engine

**Source:** `vcv/AudibleInstruments/eurorack/clouds/dsp/`

64-voice granular processor with:

- Tiered quality levels (trading voice count for sample rate / bit depth)
- Sub-sample grain scheduling for precise timing
- Carmack fast inverse sqrt for gain normalization
- 4 playback modes: granular, spectral (phase vocoder), looping delay, oliverb reverb
- Window shapes: Hann, inverse Hann, triangle, reverse, with smooth morphing between them

**Portable ideas:**

- The grain scheduling system (sub-sample precision with interpolation) is applicable to
  any granular module
- The quality-tier concept (degrade gracefully under CPU pressure) could be exposed as a
  module parameter

### 3.2 — Rings Resonator — 64-Mode Modal Synthesis

**Source:** `vcv/AudibleInstruments/eurorack/rings/dsp/`

64 parallel bandpass resonators driven by an exciter signal. Each mode has independently
computed frequency, amplitude, and decay. Three resonator models:

1. **Modal** — Pure bandpass bank (struck/plucked metal, glass, wood)
2. **Sympathetic strings** — Modes tuned to harmonic series with coupling
3. **Modulated/inharmonic** — FM-like mode spacing for bell/gong textures

Key technique: `CosineOscillator` used to simulate pickup position — multiplying mode
amplitudes by `cos(n·π·position)` creates comb-filter-like spectral shaping that mimics
moving a pickup along a vibrating surface. Odd/even mode splitting to stereo gives natural
width.

### 3.3 — Rings String — Extended Karplus-Strong

**Source:** `vcv/AudibleInstruments/eurorack/rings/dsp/`

Goes well beyond basic KS:

- **Dual damping filters** — Separate brightness and high-frequency damping controls
- **Allpass dispersion** — Models stiffness (piano-like inharmonicity)
- **Curved bridge nonlinearity** — `x / (1 + |x|)` soft clip in the feedback loop, amount
  controlled by "position" parameter
- **RT60 decay targeting** — Compute feedback coefficient from desired decay time rather
  than arbitrary 0-1 range

### 3.4 — Warps Cross-Modulation (6 Algorithms)

**Source:** `vcv/AudibleInstruments/eurorack/warps/dsp/`

Six cross-modulation algorithms with smooth morphing between them:

1. Crossfade (dry blend)
2. Frequency-domain vocoder
3. Ring modulation (4-quadrant multiply)
4. Analog ring mod (Julian Parker diode model — `d(x) = x·exp(x) / (exp(x) + 1)`)
5. Bitwise XOR (digital ring mod)
6. Octave-shifted fold (wavefolder with carrier tracking)

Easter egg mode: Frequency shifter using quadrature oscillator + Hilbert transform.

The template morphing system is the interesting part — each algorithm pair has a crossfade
zone, and the `Modulator::Process()` function manages a state machine that smoothly
transitions between adjacent algorithms based on a single "algorithm" knob position.

### 3.5 — Elements — Bowed String Interaction

**Source:** `vcv/AudibleInstruments/eurorack/elements/dsp/`

McIntyre/Schumacher/Woodhouse physical model of bow-string interaction:

- `BowTable` implements the static friction curve `f(v) = v · exp(-a·v² + b)`
- Exciter section provides blow, bow, and strike models
- Resonator body uses modal synthesis (shared with Rings)

The bow model is the novel part — it's a well-studied physical model that produces realistic
bowed-string behavior including Helmholtz motion, and the implementation is surprisingly
compact.

### 3.6 — Plaits — 16-Engine Macro-Oscillator

**Source:** `vcv/AudibleInstruments/eurorack/plaits/dsp/`

Key design pattern: `HysteresisQuantizer` prevents engine-switching noise by requiring the
engine select knob to move past a threshold before changing. Each engine has identical I/O
interface (`Render(params, output, aux)`) despite wildly different algorithms.

Engines worth studying individually: virtual analog, waveshaping, FM, grain, string, modal,
bass drum, snare drum, hi-hat (each is a standalone DSP algorithm).

The built-in LPG (low-pass gate) applies to ALL engine outputs — it's in the voice wrapper,
not per-engine. This is a good architectural decision: shared post-processing that's
orthogonal to synthesis method.

### 3.7 — Glicol Audio Graph

**Source:** `glicol/rs/synth/src/graph.rs`, `context.rs`

`petgraph`-based audio DAG with:

- `const`-generic buffer sizes (compile-time-fixed block length)
- DFS post-order traversal for correct processing order
- "Reverb as sub-graph" pattern — complex effects are sub-DAGs, not monolithic nodes
- Hot-swappable nodes (replace a node's processor without rebuilding the graph)

The graph traversal strategy and the sub-graph pattern are both relevant to Operator's
`PatchGraph` system, though Operator already has a working graph implementation.

### 3.8 — SuperCollider Supernova — Lock-Free DSP Threading

**Source:** `supercollider/server/supernova/dsp_thread_queue/`

Lock-free work-stealing thread pool for parallel DSP:

- Groups of unit generators form dependency chains
- Independent chains processed in parallel across worker threads
- Atomic reference counting for completion signaling
- No mutexes in the audio path

This is relevant if Operator ever needs multi-threaded DSP processing. The current
single-audio-thread design is simpler and likely sufficient, but this is the reference
implementation for how to do it right.

### 3.9 — BespokeSynth Lock-Free Queue

**Source:** `BespokeSynth/Source/LockFreeQueue.h`

SPSC (single-producer, single-consumer) lock-free queue using the Herb Sutter "divider"
pattern. Simpler than Operator's `rtrb`-based command queue but same fundamental approach.
Worth comparing implementations.

### 3.10 — VCV Rack Parameter System

**Source:** VCV Rack SDK

Features beyond basic parameter storage:

- **Smoothing** — Exponential filter on parameter changes to prevent zipper noise
- **Snapping** — Quantize to integer/semitone/etc. values
- **Display scaling** — Separate internal range vs. displayed range with unit labels
- **Randomization ranges** — Per-parameter limits on random values

### 3.11 — MetaModule Bypass Routing

**Source:** `metamodule-core-modules/4ms/info/*_info.hh`

`constexpr std::array<BypassRoute, N>` in module Info structs declares input→output
passthrough when a module is bypassed. `SmartCoreProcessor::handle_bypass()` copies inputs to
paired outputs, zeros unlisted outputs. Supports fan-out (1 input → 2 outputs for stereo).

Pattern worth adopting — explicitly declaring bypass behavior per-module ensures predictable
signal flow when modules are disabled.

---

## Tier 4 — Language & DSL Design Ideas

### 4.1 — Glicol Parser — Minimal Audio DSL

**Source:** `glicol/rs/parser/src/glicol.pest`

PEG grammar that compiles text directly to audio graph:

```
// Glicol syntax
~a: sin 440 >> mul 0.5
~b: saw 220 >> lpf 1000 0.5
~out: mix ~a ~b
```

Key ideas:

- `>>` operator chains audio nodes (pipeline syntax)
- `~name:` creates named signal paths
- References (`~a`) enable DAG topology from linear text
- Entire grammar is ~100 lines of PEG

### 4.2 — Vult Transpiler — Stateful DSP Language

**Source:** `vult/src/`

Hand-written Pratt parser in OCaml for a DSP-specific language:

- `mem` keyword declares persistent state (survives between `process` calls)
- `val` keyword for sample-rate-dependent initialization
- Hindley-Milner type inference (no type annotations needed)
- Generates C, JavaScript, Lua, and Java from same source

The `mem` keyword concept is particularly relevant — it solves the "where do I put my filter
state?" problem that every DSP framework struggles with. In Operator's DSL, module state is
implicit in the Rust structs, but an explicit `mem` keyword in the JS DSL could enable
user-defined stateful processors.

### 4.3 — Strudel Pattern System — Patterns as Time Queries

**Source:** `strudel/packages/core/pattern.mjs`

Patterns are functions from time spans to events:

```javascript
// A pattern is: (Span) => [Event]
// where Span = {begin: Fraction, end: Fraction}
// and Event = {whole: Span, part: Span, value: any}
```

This query-based model (from Haskell's TidalCycles) enables:

- Lazy evaluation — only compute events in the requested time window
- Composability — `stack`, `cat`, `fastcat`, `seq`, `apply`/`layer` combine patterns
- Time manipulation — `fast`, `slow`, `early`, `late` transform the query span
- 8 alignment modes for combining patterns of different lengths

The Functor/Applicative/Monad hierarchy gives you `fmap` (transform values), `appLeft`/
`appRight`/`appBoth` (combine patterns), and `bind` (pattern-dependent patterns).

### 4.4 — Strudel Mini-Notation

**Source:** `strudel/packages/mini/mini.mjs`

PEG.js grammar for TidalCycles mini-notation:

```
"bd sd [hh hh] cp"     // sequence with subdivision
"bd*4"                  // repeat
"bd(3,8)"               // Euclidean rhythm
"<bd sd cp>"            // alternate per cycle
"bd? sd"                // random drop
"bd:2"                  // sample index
```

Compact, musician-friendly notation that could inspire Operator's DSL syntax for
pattern/sequencer modules.

---

## Strudel Branch Analysis

Analysis of unmerged branches on the Codeberg fork (`codeberg.org/uzu/strudel`).

### 5.1 — `soloshortcut` — Major Simplification Refactor

**199 files changed, -8135 / +1754 lines (net deletion of ~6400 lines)**

This is a significant cleanup. The key architectural changes:

**Transpiler consolidation:** The plugin-based transpiler architecture
(`registerTranspilerPlugin`, `getPlugins`) has been removed. Separate plugin files for mini,
kabelsalat, sample, and widgets are consolidated into a single `transpiler.mjs`. New `K()`
call handling extracts pattern placeholders from Kabelsalat DSP code and wraps Strudel
patterns with `S()` for interop.

**Block-based evaluation removed:** The entire block-based evaluation system (~200 lines)
is deleted from `repl.mjs`. No more `codeBlocks`, `blockPatterns`, `evaluateBlock()`,
`processLabeledBlock()`. Solo pattern handling (`S$:` prefix) is inlined directly into
`evaluate()`. This is a major simplification of the REPL's execution model.

**Helix editor mode dropped:** The `codemirror-helix` dependency is removed entirely.
Vim, Emacs, VSCode, and default CodeMirror keybindings remain.

**JSDoc tag cleanup:** Removed `superdough`, `wavetable`, `stepwise` tags from controls.
Consolidated to `fx`, `temporal`, `samples`, `music_theory`, `combiners`. This suggests a
documentation/discoverability restructuring.

**Package removals:** Entire `packages/edo/` package (EDO tuning system) deleted.
`packages/midi/input.mjs` and `packages/midi/util.mjs` deleted.

**Pattern.mjs additions:**

- `log2` promoted to a COMPOSER (previously standalone `register`)
- `seq` added as synonym for `fastcat`
- `apply` added as synonym for `layer`

**Takeaway:** This branch represents a philosophical shift toward simplicity — removing
plugin architecture overhead, dropping niche features (EDO, Helix mode, block evaluation),
and consolidating code. The transpiler rewrite to support Kabelsalat (a modular synth DSL)
alongside Strudel patterns is the key new capability being traded for all the removed
complexity.

### 5.2 — `sample_note` — Pitch Detection from Sample Filenames

**5 commits**

New `extractMidiNoteFromString()` in `sampler.mjs` — regex extracts note names from
filenames (e.g., `piano_C4.wav` → MIDI 60). Sample map processing now returns
`SampleMetaData` objects `{url, midi}` instead of plain URL strings.

This enables automatic chromatic mapping of sample packs — load a folder of `C4.wav`,
`D4.wav`, etc. and they auto-map to the correct pitches. Useful idea for any sampler module.

### 5.3 — `glossing/inputs` — External Audio Inputs

**2 commits**

Allows using microphone/line-in as a sound source in Strudel patterns. The external audio
input becomes a pattern value that can be processed through the same effect chain as any
other sound source.

### 5.4 — `glossing/phases-bugfix` — Phase Computation Fix

**1 commit — typo fix in phase computation.** Small but important correctness fix.

---

## Bug Fixes & Lessons Learned

### 6.1 — JPVerb Memory Bugs (sc3-plugins)

**Source:** `sc3-plugins/source/DEINDUGens/JPverbRaw.cpp`

Three bugs found and fixed in recent commits:

1. **Uninitialized pointers** — Delay line pointers used before allocation
2. **Inverted null check** — `if (mem)` instead of `if (!mem)`, meaning JPVerb always
   failed silently with audio-rate modulation inputs. This bug existed for years.
3. **Unsafe `goto end`** — Jumped over variable initialization

**Lesson:** Memory management bugs in DSP code can be silent — the output just sounds wrong
or produces silence, and users blame their patch rather than the module. Rust's ownership
system prevents all three of these bugs by construction.

### 6.2 — ErbeVerb Issues (audioAlgorithmTests)

**Source:** `audioAlgorithmTests/erbe/src/main.rs`

- **Stereo EQ state bleed** — Left and right channels share biquad filter state variables.
  Each channel needs its own `z1`/`z2` state.
- **No parameter smoothing** — Direct parameter application causes zipper noise.
  Should use exponential smoothing (see `OnePoleLowpass` from VCV above).
- **Linear interpolation on delay reads** — Produces audible artifacts in reverb tails.
  Hermite interpolation (see stmlib `delay_line.h`) is the standard for reverb quality.

### 6.3 — VBAP Dynamic Speaker Allocation (sc3-plugins)

Removed hardcoded speaker limits in VBAP implementation, replacing with dynamic allocation.
Relevant lesson: avoid hardcoded array sizes for configurable topology — use dynamic
allocation during init, fixed-size operation during process.

---

## Source Index

Quick reference for all mentioned source files.

### User's Projects

| File                                          | Description                  |
| --------------------------------------------- | ---------------------------- |
| `audio/audioAlgorithmTests/erbe/src/main.rs`  | ErbeVerb FDN reverb          |
| `audio/audioAlgorithmTests/delay/src/main.rs` | Stereo delay                 |
| `audio/audioAlgorithmTests/echo/src/main.rs`  | Echo with LFO modulation     |
| `audio/lily/src/types/plugins.ts`             | Plugin type system           |
| `audio/lily/claude-electron-plugins.md`       | Plugin architecture research |

### VCV Rack / Mutable Instruments

| File                                                            | Description                    |
| --------------------------------------------------------------- | ------------------------------ |
| `audio/vcv/Fundamental/src/VCO.cpp`                             | MinBLEP VCO (complete rewrite) |
| `audio/vcv/Fundamental/src/VCF.cpp`                             | TPT ladder with ADAA           |
| `audio/vcv/AudibleInstruments/eurorack/stmlib/dsp/filter.h`     | ZDF SVF, OnePole, DCBlocker    |
| `audio/vcv/AudibleInstruments/eurorack/stmlib/dsp/polyblep.h`   | PolyBLEP/PolyBLAMP             |
| `audio/vcv/AudibleInstruments/eurorack/stmlib/dsp/delay_line.h` | Template delay line            |
| `audio/vcv/AudibleInstruments/eurorack/clouds/dsp/`             | Granular engine                |
| `audio/vcv/AudibleInstruments/eurorack/rings/dsp/`              | Modal resonator + string       |
| `audio/vcv/AudibleInstruments/eurorack/warps/dsp/`              | Cross-modulation               |
| `audio/vcv/AudibleInstruments/eurorack/elements/dsp/`           | Bowed resonator                |
| `audio/vcv/AudibleInstruments/eurorack/plaits/dsp/`             | 16-engine macro-osc            |

### 4ms MetaModule

| File                                               | Description                              |
| -------------------------------------------------- | ---------------------------------------- |
| `audio/metamodule-core-modules/4ms/core/`          | MoogLadder, Karplus, Wavefold, LPG, etc. |
| `audio/metamodule-core-modules/4ms/info/*_info.hh` | Bypass route declarations                |

### BespokeSynth

| File                                             | Description          |
| ------------------------------------------------ | -------------------- |
| `audio/BespokeSynth/Source/DopplerShift.cpp`     | Doppler modulator    |
| `audio/BespokeSynth/Source/IControlVisualizer.h` | LCD render interface |
| `audio/BespokeSynth/Source/LockFreeQueue.h`      | SPSC queue           |

### Csound

| File                          | Description          |
| ----------------------------- | -------------------- |
| `audio/csound/Opcodes/dbap.c` | DBAP spatial panning |
| `audio/csound/Opcodes/dbap.h` | DBAP types           |

### Language / DSL

| File                                      | Description                                       |
| ----------------------------------------- | ------------------------------------------------- |
| `audio/glicol/rs/parser/src/glicol.pest`  | PEG audio DSL grammar                             |
| `audio/glicol/rs/synth/src/graph.rs`      | petgraph audio DAG                                |
| `audio/vult/src/`                         | Pratt parser, `mem` keyword, multi-target codegen |
| `audio/strudel/packages/core/pattern.mjs` | Pattern-as-query system                           |
| `audio/strudel/packages/mini/mini.mjs`    | Mini-notation PEG grammar                         |

### Other

| File                                                     | Description               |
| -------------------------------------------------------- | ------------------------- |
| `audio/supercollider/server/supernova/dsp_thread_queue/` | Lock-free work stealing   |
| `audio/sc3-plugins/source/DEINDUGens/JPverbRaw.cpp`      | JPVerb (memory bug fixes) |
