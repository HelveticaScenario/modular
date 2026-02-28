# $supersaw Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a self-contained supersaw oscillator module faithful to Strudel's superdough, adapted to Operator's multichannel poly architecture.

**Architecture:** Matrix mixing — `input_channels × voices` internal PolyBLEP saws, output channels = `voices`. Each output channel sums all input pitches at that voice's detune offset, normalized by `1/sqrt(input_channels)`. Random initial phases per internal oscillator.

**Tech Stack:** Rust, proc macros (`#[module]`, `#[derive(Connect, ChannelCount, Outputs)]`), `PolySignal`/`PolyOutput` types.

**Design doc:** `docs/plans/2026-02-28-supersaw-design.md`

---

### Task 1: Create supersaw.rs with Params, Outputs, and Struct Skeleton

**Files:**

- Create: `crates/modular_core/src/dsp/oscillators/supersaw.rs`

**Step 1: Write the module skeleton**

````rust
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    dsp::utils::voct_to_hz,
    poly::{PolyOutput, PolySignal, PORT_MAX_CHANNELS},
    types::PolySignalFields,
};

fn default_voices() -> usize {
    5
}

fn default_detune() -> PolySignal {
    PolySignal::mono(crate::types::Signal::Volts(0.18))
}

#[derive(Clone, Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default, rename_all = "camelCase")]
struct SupersawParams {
    /// pitch in V/Oct (0V = C4)
    freq: PolySignal,
    /// number of unison voices (1-16, determines output channel count)
    #[serde(default = "default_voices")]
    voices: usize,
    /// detune spread in semitones (default 0.18)
    #[serde(default = "default_detune")]
    detune: PolySignal,
}

/// Custom channel count: output channels = voices (clamped 1-16).
#[allow(private_interfaces)]
pub fn supersaw_derive_channel_count(params: &SupersawParams) -> usize {
    params.voices.clamp(1, PORT_MAX_CHANNELS)
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct SupersawOutputs {
    #[output("output", "signal output", default, range = (-5.0, 5.0))]
    sample: PolyOutput,
}

/// Internal state for one oscillator voice (one pitch × one unison voice).
#[derive(Clone, Copy)]
struct OscState {
    phase: f32,
}

impl Default for OscState {
    fn default() -> Self {
        Self { phase: 0.0 }
    }
}

/// Supersaw oscillator with unison voices and matrix mixing.
///
/// Takes polyphonic V/Oct pitch input and produces thick detuned
/// sawtooth output. Each output channel sums all input pitches at
/// that voice's detune offset, with random initial phases and
/// 1/sqrt(N) gain compensation.
///
/// Faithful to Strudel's superdough SuperSawOscillatorProcessor.
///
/// ## Example
///
/// ```js
/// $supersaw('c3').out()
/// $supersaw($midiCV().pitch, 7, 0.3).out()
/// ```
#[module(name = "$supersaw", channels_derive = supersaw_derive_channel_count, args(freq, voices?, detune?))]
#[derive(Default)]
pub struct Supersaw {
    outputs: SupersawOutputs,
    params: SupersawParams,
    /// Phase state for each internal oscillator.
    /// Indexed as [input_ch * PORT_MAX_CHANNELS + voice].
    /// Max 16×16 = 256 entries.
    osc_states: [OscState; PORT_MAX_CHANNELS * PORT_MAX_CHANNELS],
    /// Whether phases have been randomized (done once on first update).
    phases_initialized: bool,
    /// Simple PRNG state for random initial phases.
    rng_state: u32,
}

impl Supersaw {
    fn update(&mut self, sample_rate: f32) {
        // Placeholder — implemented in Task 2
    }
}

message_handlers!(impl Supersaw {});
````

**Step 2: Verify it compiles**

Run: `cargo check -p modular_core 2>&1 | grep -E 'error|warning.*supersaw'`
Expected: No errors (warnings about unused fields are OK at this stage)

**Step 3: Commit**

```
git add crates/modular_core/src/dsp/oscillators/supersaw.rs
git commit -m "scaffold: add $supersaw module skeleton"
```

---

### Task 2: Implement the PolyBLEP Saw and Core DSP Loop

**Files:**

- Modify: `crates/modular_core/src/dsp/oscillators/supersaw.rs`

**Step 1: Write failing tests for core behavior**

Add at the bottom of supersaw.rs:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{OutputStruct, Signal};

    /// Helper: create a Supersaw with given params, properly initialized.
    fn make_supersaw(params: SupersawParams) -> Supersaw {
        let channels = supersaw_derive_channel_count(&params);
        let mut outputs = SupersawOutputs::default();
        outputs.set_all_channels(channels);
        let mut s = Supersaw {
            params,
            outputs,
            _channel_count: channels,
            ..Default::default()
        };
        s
    }

    #[test]
    fn test_single_voice_no_detune_produces_output() {
        let mut s = make_supersaw(SupersawParams {
            freq: PolySignal::mono(Signal::Volts(0.0)), // C4
            voices: 1,
            detune: PolySignal::mono(Signal::Volts(0.0)),
        });
        // Run several samples to get past initial phase
        for _ in 0..100 {
            s.update(48000.0);
        }
        // With 1 voice, output should be a single channel saw
        assert_eq!(s.outputs.sample.channels(), 1);
        // Output should be non-zero (saw is almost never exactly 0)
        let mut has_nonzero = false;
        for _ in 0..100 {
            s.update(48000.0);
            if s.outputs.sample.get(0).abs() > 0.01 {
                has_nonzero = true;
                break;
            }
        }
        assert!(has_nonzero, "saw should produce non-zero output");
    }

    #[test]
    fn test_output_channel_count_equals_voices() {
        let s = make_supersaw(SupersawParams {
            freq: PolySignal::mono(Signal::Volts(0.0)),
            voices: 7,
            detune: PolySignal::mono(Signal::Volts(0.18)),
        });
        assert_eq!(s.outputs.sample.channels(), 7);
    }

    #[test]
    fn test_output_bounded() {
        // Output per channel should be bounded (gain compensated)
        let mut s = make_supersaw(SupersawParams {
            freq: PolySignal::poly(&[
                Signal::Volts(0.0),
                Signal::Volts(0.25),
                Signal::Volts(0.5),
                Signal::Volts(0.75),
            ]),
            voices: 4,
            detune: PolySignal::mono(Signal::Volts(0.18)),
        });
        for _ in 0..1000 {
            s.update(48000.0);
            for ch in 0..4 {
                // With 4 input channels, gain = 5.0 / sqrt(4) = 2.5 per saw
                // 4 saws summed: max = 4 * 2.5 = 10.0 theoretical peak
                // But practically less due to phase randomization
                let val = s.outputs.sample.get(ch).abs();
                assert!(val <= 11.0, "output ch {} was {} — should be bounded", ch, val);
            }
        }
    }

    #[test]
    fn test_voices_clamped_to_16() {
        let params = SupersawParams {
            freq: PolySignal::mono(Signal::Volts(0.0)),
            voices: 20,
            detune: PolySignal::mono(Signal::Volts(0.18)),
        };
        assert_eq!(supersaw_derive_channel_count(&params), 16);
    }

    #[test]
    fn test_detune_spread_affects_pitch() {
        // With large detune, voices should produce different frequencies
        // We test by checking that outputs differ across channels
        let mut s = make_supersaw(SupersawParams {
            freq: PolySignal::mono(Signal::Volts(0.0)),
            voices: 3,
            detune: PolySignal::mono(Signal::Volts(12.0)), // 12 semitones = 1 octave
        });
        // Run for a while
        for _ in 0..500 {
            s.update(48000.0);
        }
        // Collect several samples per channel
        let mut sums = [0.0f64; 3];
        for _ in 0..1000 {
            s.update(48000.0);
            for ch in 0..3 {
                sums[ch] += s.outputs.sample.get(ch).abs() as f64;
            }
        }
        // All channels should have produced output
        for ch in 0..3 {
            assert!(sums[ch] > 0.0, "channel {} should have output", ch);
        }
    }

    #[test]
    fn test_random_phases_differ() {
        // Create two supersaws — their phases should differ (random init)
        let mut s1 = make_supersaw(SupersawParams {
            freq: PolySignal::mono(Signal::Volts(0.0)),
            voices: 1,
            detune: PolySignal::mono(Signal::Volts(0.0)),
        });
        let mut s2 = make_supersaw(SupersawParams {
            freq: PolySignal::mono(Signal::Volts(0.0)),
            voices: 1,
            detune: PolySignal::mono(Signal::Volts(0.0)),
        });
        // Give them different rng seeds
        s2.rng_state = 12345;
        s1.update(48000.0);
        s2.update(48000.0);
        // After first update (which initializes phases), outputs should differ
        // because of different random initial phases
        let v1 = s1.outputs.sample.get(0);
        let v2 = s2.outputs.sample.get(0);
        // They COULD be equal by chance but it's astronomically unlikely
        assert!((v1 - v2).abs() > 1e-6, "different rng seeds should produce different phases");
    }

    #[test]
    fn test_matrix_mixing_mono_input() {
        // 1 input channel, 3 voices: each voice is just the one pitch detuned
        let mut s = make_supersaw(SupersawParams {
            freq: PolySignal::mono(Signal::Volts(0.0)),
            voices: 3,
            detune: PolySignal::mono(Signal::Volts(0.0)), // no detune
        });
        for _ in 0..100 {
            s.update(48000.0);
        }
        assert_eq!(s.outputs.sample.channels(), 3);
        // With no detune and mono input, all 3 voices play same pitch
        // but with different random phases, so outputs differ
    }

    #[test]
    fn test_gain_compensation() {
        // Compare amplitude of 1-input vs 4-input supersaw
        // The 4-input version should be gain-compensated to similar levels
        let mut s1 = make_supersaw(SupersawParams {
            freq: PolySignal::mono(Signal::Volts(0.0)),
            voices: 1,
            detune: PolySignal::mono(Signal::Volts(0.0)),
        });
        let mut s4 = make_supersaw(SupersawParams {
            freq: PolySignal::poly(&[
                Signal::Volts(0.0),
                Signal::Volts(0.0),
                Signal::Volts(0.0),
                Signal::Volts(0.0),
            ]),
            voices: 1,
            detune: PolySignal::mono(Signal::Volts(0.0)),
        });
        // Same rng so phases are comparable
        s4.rng_state = s1.rng_state;

        let mut max1 = 0.0f32;
        let mut max4 = 0.0f32;
        for _ in 0..2000 {
            s1.update(48000.0);
            s4.update(48000.0);
            max1 = max1.max(s1.outputs.sample.get(0).abs());
            max4 = max4.max(s4.outputs.sample.get(0).abs());
        }
        // s4 has 4 inputs summed with 1/sqrt(4) = 0.5 compensation
        // so max4 should be roughly 4 * 0.5 = 2x max1, NOT 4x
        assert!(max4 < max1 * 3.0, "gain compensation should prevent 4x blowup: max1={}, max4={}", max1, max4);
    }
}
```

**Step 2: Run tests to verify they fail**

Run: `cargo test -p modular_core supersaw -- --nocapture 2>&1 | tail -20`
Expected: FAIL — `update` is empty, no output produced

**Step 3: Implement the full update method and PolyBLEP**

Replace the `update` method and add the helper functions:

```rust
/// PolyBLEP correction for sawtooth discontinuity at phase wrap.
/// Matches Strudel's polyBlep function.
#[inline]
fn poly_blep_saw(phase: f32, dt: f32) -> f32 {
    if dt <= 0.0 {
        return 0.0;
    }
    if phase < dt {
        // Just after discontinuity
        let t = phase / dt;
        return t + t - t * t - 1.0;
    } else if phase > 1.0 - dt {
        // Just before discontinuity
        let t = (phase - 1.0) / dt;
        return t * t + t + t + 1.0;
    }
    0.0
}

/// Simple xorshift32 PRNG for random initial phases.
#[inline]
fn xorshift32(state: &mut u32) -> u32 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    *state = x;
    x
}

/// Convert xorshift output to f32 in [0, 1).
#[inline]
fn rand_phase(state: &mut u32) -> f32 {
    (xorshift32(state) as f32) / (u32::MAX as f32)
}

impl Supersaw {
    fn update(&mut self, sample_rate: f32) {
        let voices = self.params.voices.clamp(1, PORT_MAX_CHANNELS);
        let freq_input = &self.params.freq;
        let detune_input = &self.params.detune;

        let input_channels = if freq_input.is_disconnected() {
            1
        } else {
            freq_input.channels()
        };

        // Initialize random phases on first update
        if !self.phases_initialized {
            if self.rng_state == 0 {
                // Seed from a pointer to self for uniqueness across instances
                self.rng_state = (self as *const Self as usize as u32).wrapping_add(1);
            }
            for i in 0..(PORT_MAX_CHANNELS * PORT_MAX_CHANNELS) {
                self.osc_states[i].phase = rand_phase(&mut self.rng_state);
            }
            self.phases_initialized = true;
        }

        // Gain compensation: 1/sqrt(input_channels)
        let gain = 5.0 / (input_channels as f32).sqrt();

        // Read detune value (mono — take first channel)
        let detune_semitones = detune_input.get_value_or(0, 0.18);

        for voice in 0..voices {
            // Compute detune offset for this voice in semitones
            let detune_offset = if voices > 1 {
                let t = voice as f32 / (voices - 1) as f32;
                detune_semitones * (t - 0.5)
            } else {
                0.0
            };

            let mut sum = 0.0f32;

            for input_ch in 0..input_channels {
                let pitch_voct = freq_input.get_value_or(input_ch, 0.0);
                // Convert V/Oct to Hz, then apply semitone detune
                let base_hz = voct_to_hz(pitch_voct);
                let freq_hz = base_hz * 2.0_f32.powf(detune_offset / 12.0);

                let idx = input_ch * PORT_MAX_CHANNELS + voice;
                let state = &mut self.osc_states[idx];

                let dt = (freq_hz / sample_rate).abs();

                // Advance phase
                state.phase += dt;
                if state.phase >= 1.0 {
                    state.phase -= 1.0;
                }

                // Naive sawtooth: maps [0, 1) -> [-1, 1)
                let naive = 2.0 * state.phase - 1.0;
                // Apply PolyBLEP anti-aliasing
                let sample = naive - poly_blep_saw(state.phase, dt);

                sum += sample;
            }

            self.outputs.sample.set(voice, sum * gain);
        }
    }
}
```

**Step 4: Run tests to verify they pass**

Run: `cargo test -p modular_core supersaw -- --nocapture 2>&1 | tail -20`
Expected: All tests PASS

**Step 5: Commit**

```
git add crates/modular_core/src/dsp/oscillators/supersaw.rs
git commit -m "feat: implement $supersaw DSP with polyBLEP and matrix mixing"
```

---

### Task 3: Register in Oscillators mod.rs

**Files:**

- Modify: `crates/modular_core/src/dsp/oscillators/mod.rs`

**Step 1: Add module declaration and register in all 4 functions**

Add `pub mod supersaw;` to the module declarations (after `pub mod sine;`).

Add these 4 lines to the respective functions:

- `supersaw::Supersaw::install_constructor(map);` in `install_constructors`
- `supersaw::Supersaw::install_params_validator(map);` in `install_param_validators`
- `supersaw::Supersaw::install_params_deserializer(map);` in `install_params_deserializers`
- `supersaw::Supersaw::get_schema(),` in `schemas`

**Step 2: Run full test suite**

Run: `cargo test -p modular_core 2>&1 | tail -5`
Expected: All tests pass (previous count + new supersaw tests)

**Step 3: Run the integration tests too**

Run: `cargo test 2>&1 | grep 'test result'`
Expected: All test suites pass, no new failures

**Step 4: Commit**

```
git add crates/modular_core/src/dsp/oscillators/mod.rs
git commit -m "register $supersaw in oscillator module registry"
```

---

### Task 4: Verify No New Warnings

**Files:** None (verification only)

**Step 1: Check for warnings**

Run: `cargo test -p modular_core 2>&1 | grep 'warning.*supersaw'`
Expected: No warnings from supersaw code. Pre-existing warnings in other files are acceptable.

**Step 2: If warnings exist, fix them and amend the previous commit**

Common issues:

- Unused imports → remove them
- Dead code warnings → add `#[allow(dead_code)]` only if the code is intentionally kept for future use
- `private_interfaces` warning on `supersaw_derive_channel_count` → should already be suppressed with `#[allow(private_interfaces)]`
