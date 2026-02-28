# $supersaw Module Design

## Summary

A self-contained supersaw oscillator module faithful to Strudel's superdough implementation, adapted to the Operator multichannel architecture. Takes polyphonic V/Oct pitch input, produces thick detuned sawtooth output using a matrix mixing approach.

## Parameters

| Param    | Type       | Default  | Description                                                                          |
| -------- | ---------- | -------- | ------------------------------------------------------------------------------------ |
| `freq`   | PolySignal | required | V/Oct pitch input (positional arg)                                                   |
| `voices` | usize      | 5        | Unison voice count, 1-16. Determines output channel count.                           |
| `detune` | PolySignal | 0.18     | Detune spread in semitones. Voices distributed linearly from -detune/2 to +detune/2. |

## Architecture

### Matrix Mixing

Internal oscillator count = `input_channels x voices` (up to 16x16 = 256).

Output channel count = `voices` (fixed, independent of input polyphony).

Each output channel sums across all input pitches at that voice index's detune offset:

```
output[voice_j] = (1/sqrt(input_channels)) * sum_i(saw(pitch_i + detune_j, phase[i][j]))
```

Example with 4-note chord, voices=4:

- 16 internal oscillators (4 pitches x 4 voices)
- Output ch0 = saw(p1, detune_0, phase_0_0) + saw(p2, detune_0, phase_0_1) + saw(p3, detune_0, phase_0_2) + saw(p4, detune_0, phase_0_3), normalized by 1/sqrt(4)
- Output ch1 = same but at detune_1 offsets
- etc.

### Detune Distribution

Linear in semitone space, symmetric around center (faithful to Strudel):

```
offset(voice_i) = lerp(-detune/2, +detune/2, i / (voices - 1))
```

For voices=1, offset is 0 (no detune).

Conversion to frequency: `freq * 2^(semitones / 12)`

### Waveform

PolyBLEP band-limited sawtooth (faithful to Strudel's polyBLEP approach, already in codebase via $pulse).

### Phase

Each internal oscillator gets a random initial phase on creation. Phase is a f32 in [0, 1), incremented by `frequency / sample_rate` each sample.

### Gain Compensation

Equal-power normalization: each output channel divides by `sqrt(input_channels)` to prevent level blowup with more input notes.

### Channel Count Derivation

Uses `channels_derive` with custom function. Output channels = `voices` parameter value, independent of input poly channel count.

## Relationship to $unison

`$unison` is kept as-is. It serves a different purpose: general-purpose pitch expansion before any oscillator type. `$supersaw` is the batteries-included version for the supersaw-specific case.

## Reference

Based on Strudel superdough's `SuperSawOscillatorProcessor`:

- `packages/superdough/worklets.mjs` (lines 465-578)
- `packages/superdough/synth.mjs` (lines 153-217)

Key Strudel behaviors preserved:

- Linear semitone distribution
- Random initial phases
- PolyBLEP anti-aliasing
- Default 5 voices, 0.18 semitone detune
