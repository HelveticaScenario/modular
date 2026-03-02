# $curve Module + .gain() / .exp() Design

## Summary

A new `$curve` utility module that applies a power curve to a signal, plus two DSL convenience methods: `.gain(level)` for perceptual amplitude control, and `.exp(factor)` for general signal shaping.

## Problem

`.amp()` applies linear gain via `$scaleAndShift`. Linear amplitude doesn't match human loudness perception — halving the voltage only drops ~6dB, but perceptual "half as loud" is ~10dB. We need a way to control amplitude along a perceptual curve.

## $curve Module

### Parameters

| Param   | Type       | Default  | Range | Description              |
| ------- | ---------- | -------- | ----- | ------------------------ |
| `input` | PolySignal | required | —     | Signal to curve          |
| `exp`   | PolySignal | required | min 0 | Exponent for power curve |

### Formula

```
sign(x) * 5 * (|x| / 5) ^ exp
```

- Sign-preserving: works with bipolar signals
- Normalizes at ±5V: output = input when |input| = 5V, regardless of exponent
- At 0V: output = 0V regardless of exponent
- exp=1: linear pass-through
- exp>1: pushes midrange values toward 0 (audio taper)
- 0<exp<1: pushes midrange values toward ±5V
- exp=0: step function (any nonzero input maps to ±5V)

### Module Declaration

```rust
#[module(name = "$curve", args(input, exp))]
```

Both args are positional and required. Uses `PolySignal` throughout.

### DSL Usage

```js
$curve(lfo, 2); // square the curve of an LFO
$curve(env, 0.5); // sqrt curve on an envelope
$curve(signal, 3); // cubic curve
```

## .gain(level) DSL Method

Chains `$curve` → `$scaleAndShift` with a fixed exponent of 3, providing perceptual amplitude control.

### Implementation

```
curvedLevel = $curve(level, 3)
output = $scaleAndShift(signal, curvedLevel)
```

Effective gain = `(level / 5) ^ 3`:

| level | .amp() (linear) | .gain() (exp=3) |
| ----- | --------------- | --------------- |
| 5V    | 0 dB (unity)    | 0 dB (unity)    |
| 4.5V  | -0.9 dB         | -1.4 dB         |
| 3.75V | -2.5 dB         | -3.7 dB         |
| 2.5V  | -6.0 dB         | -9.0 dB         |
| 1.25V | -12.0 dB        | -18.1 dB        |
| 0.5V  | -20.0 dB        | -30.0 dB        |
| 0V    | silence         | silence         |

Midpoint (2.5V) at -9dB is close to the perceptual "half as loud" mark (~10dB). Matches standard audio-taper potentiometer response. ~30dB usable dynamic range.

### DSL Usage

```js
osc.gain(2.5)                // perceptual half volume
osc.gain($adsr(gate, ...))   // envelope-controlled perceptual volume
$c(osc1, osc2).gain(3)       // gain on a collection
```

## .exp(factor?) DSL Method

Wraps the signal with `$curve(signal, factor)`. Default factor is 3.

### DSL Usage

```js
lfo.exp(); // cubic curve (default exp=3)
lfo.exp(2); // quadratic curve
env.exp(0.5); // sqrt curve (expands upper range)
```

## Files to Modify

- **New:** `crates/modular_core/src/dsp/utilities/curve.rs`
- **Modify:** `crates/modular_core/src/dsp/utilities/mod.rs` — register $curve
- **Modify:** `src/main/dsl/GraphBuilder.ts` — add .gain() and .exp() to ModuleOutput and BaseCollection
- **Modify:** `src/main/dsl/typescriptLibGen.ts` — add type declarations
- **Modify:** `src/shared/dsl/typeDocs.ts` — add method documentation
- **Tests:** Rust unit tests for $curve, TS executor tests for .gain() and .exp()
