use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    dsp::utils::voct_to_hz,
    poly::{PORT_MAX_CHANNELS, PolyOutput, PolySignal},
    types::Clickless,
};

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default, rename_all = "camelCase")]
struct SawOscillatorParams {
    /// frequency in v/oct
    freq: PolySignal,
    /// waveform shape: 0=saw, 2.5=triangle, 5=ramp
    shape: PolySignal,
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct SawOscillatorOutputs {
    #[output("output", "signal output", default, range = (-5.0, 5.0))]
    sample: PolyOutput,
}

/// Per-channel oscillator state
#[derive(Default, Clone, Copy)]
struct ChannelState {
    phase: f32,
    shape: Clickless,
}

#[module(name = "osc.saw", description = "Sawtooth/Triangle/Ramp oscillator", args(freq))]
#[derive(Default)]
pub struct SawOscillator {
    outputs: SawOscillatorOutputs,
    channels: [ChannelState; PORT_MAX_CHANNELS],
    params: SawOscillatorParams,
}

impl SawOscillator {
    fn update(&mut self, sample_rate: f32) {
        let num_channels = self.channel_count();

        // Pre-compute inverse sample rate for frequency calculation
        let inv_sample_rate = 1.0 / sample_rate;

        for ch in 0..num_channels {
            let state = &mut self.channels[ch];

            // Update shape with cycling - clamp to valid range
            let shape_val = self.params.shape.get_value_or(ch, 0.0).clamp(0.0, 5.0);
            state.shape.update(shape_val);

            let frequency = voct_to_hz(self.params.freq.get_value_or(ch, 0.0));
            let phase_increment = frequency * inv_sample_rate;

            state.phase += phase_increment;

            // Wrap phase
            if state.phase >= 1.0 {
                state.phase -= 1.0;
            }

            // Shape parameter: 0 = saw, 2.5 = triangle, 5 = ramp (reversed saw)
            let shape_norm = *state.shape * 0.2; // /5.0 -> *0.2 for performance

            let raw_output = if shape_norm < 0.5 {
                // Blend from saw (0.0) to triangle (0.5)
                let blend = shape_norm * 2.0;
                let saw = generate_saw(state.phase, phase_increment);
                let triangle = generate_triangle(state.phase, phase_increment);
                saw + (triangle - saw) * blend
            } else {
                // Blend from triangle (0.5) to ramp (1.0)
                let blend = (shape_norm - 0.5) * 2.0;
                let triangle = generate_triangle(state.phase, phase_increment);
                let ramp = generate_ramp(state.phase, phase_increment);
                triangle + (ramp - triangle) * blend
            };
            self.outputs.sample.set(ch, raw_output * 5.0);
        }
    }
}

/// PolyBLEP (Polynomial Band-Limited Step) function
/// Reduces aliasing at discontinuities
#[inline(always)]
fn poly_blep(phase: f32, phase_increment: f32) -> f32 {
    // Detect discontinuity at phase wrap (0.0)
    if phase < phase_increment {
        let t = phase / phase_increment;
        return t + t - t * t - 1.0;
    }
    // Detect discontinuity at phase = 1.0
    else if phase > 1.0 - phase_increment {
        let t = (phase - 1.0) / phase_increment;
        return t * t + t + t + 1.0;
    }
    0.0
}

/// Generate band-limited sawtooth wave
#[inline(always)]
fn generate_saw(phase: f32, phase_increment: f32) -> f32 {
    let mut saw = 2.0 * phase - 1.0;
    saw -= poly_blep(phase, phase_increment);
    saw
}

/// Generate band-limited ramp wave (reversed sawtooth)
#[inline(always)]
fn generate_ramp(phase: f32, phase_increment: f32) -> f32 {
    let mut ramp = 1.0 - 2.0 * phase;
    ramp += poly_blep(phase, phase_increment);
    ramp
}

/// Generate band-limited triangle wave
#[inline(always)]
fn generate_triangle(phase: f32, phase_increment: f32) -> f32 {
    // Triangle is the integral of a square wave
    // We can generate it by integrating a PolyBLEP pulse
    let mut triangle = if phase < 0.5 {
        4.0 * phase - 1.0
    } else {
        3.0 - 4.0 * phase
    };

    // Apply PolyBLEP correction at the peak (phase = 0.5)
    triangle += poly_blep_integrated(phase, phase_increment);
    triangle -= poly_blep_integrated(
        if phase >= 0.5 {
            phase - 0.5
        } else {
            phase + 0.5
        },
        phase_increment,
    );

    triangle
}

/// Integrated PolyBLEP for triangle wave
#[inline(always)]
fn poly_blep_integrated(phase: f32, phase_increment: f32) -> f32 {
    if phase < phase_increment {
        let t = phase / phase_increment;
        return (t * t * t) / 3.0 - (t * t) / 2.0 + t / 2.0;
    } else if phase > 1.0 - phase_increment {
        let t = (phase - 1.0) / phase_increment;
        return -(t * t * t) / 3.0 - (t * t) / 2.0 - t / 2.0;
    }
    0.0
}

message_handlers!(impl SawOscillator {});
