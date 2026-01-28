use crate::{
    dsp::utils::wrap,
    poly::{PORT_MAX_CHANNELS, PolyOutput, PolySignal},
    types::Clickless,
};
use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default)]
struct DSawOscillatorParams {
    /// phase input (0-1, will be wrapped)
    phase: PolySignal,
    /// waveform shape: 0=saw, 2.5=triangle, 5=ramp
    shape: PolySignal,
}

#[derive(Outputs, JsonSchema)]
struct DSawOscillatorOutputs {
    #[output("output", "signal output", range = (-1.0, 1.0))]
    sample: PolyOutput,
}

/// Per-channel state for shape smoothing
#[derive(Default, Clone, Copy)]
struct ChannelState {
    shape: Clickless,
}

#[derive(Module)]
#[module("dSaw", "A phase-driven sawtooth/triangle/ramp oscillator")]
#[args(phase)]
pub struct DSawOscillator {
    outputs: DSawOscillatorOutputs,
    channels: [ChannelState; PORT_MAX_CHANNELS],
    params: DSawOscillatorParams,
}

impl Default for DSawOscillator {
    fn default() -> Self {
        Self {
            outputs: DSawOscillatorOutputs::default(),
            channels: [ChannelState::default(); PORT_MAX_CHANNELS],
            params: DSawOscillatorParams::default(),
        }
    }
}

impl DSawOscillator {
    fn update(&mut self, _sample_rate: f32) {
        let num_channels = self.channel_count();

        let mut output = PolyOutput::default();
        output.set_channels(num_channels as u8);

        for ch in 0..num_channels {
            let state = &mut self.channels[ch];

            // Update shape with cycling - clamp to valid range
            let shape_val = self.params.shape.get_value_or(ch, 0.0).clamp(0.0, 5.0);
            state.shape.update(shape_val);

            let phase = wrap(0.0..1.0, self.params.phase.get_value(ch));

            // Shape parameter: 0 = saw, 2.5 = triangle, 5 = ramp (reversed saw)
            let shape_norm = *state.shape * 0.2; // /5.0 -> *0.2 for performance

            let raw_output = if shape_norm < 0.5 {
                // Blend from saw (0.0) to triangle (0.5)
                let blend = shape_norm * 2.0;
                let saw = generate_saw(phase);
                let triangle = generate_triangle(phase);
                saw + (triangle - saw) * blend
            } else {
                // Blend from triangle (0.5) to ramp (1.0)
                let blend = (shape_norm - 0.5) * 2.0;
                let triangle = generate_triangle(phase);
                let ramp = generate_ramp(phase);
                triangle + (ramp - triangle) * blend
            };
            output.set(ch, raw_output);
        }

        self.outputs.sample = output;
    }
}

/// Generate naive sawtooth wave (no anti-aliasing)
#[inline(always)]
fn generate_saw(phase: f32) -> f32 {
    2.0 * phase - 1.0
}

/// Generate naive ramp wave (reversed sawtooth, no anti-aliasing)
#[inline(always)]
fn generate_ramp(phase: f32) -> f32 {
    1.0 - 2.0 * phase
}

/// Generate naive triangle wave (no anti-aliasing)
#[inline(always)]
fn generate_triangle(phase: f32) -> f32 {
    if phase < 0.5 {
        4.0 * phase - 1.0
    } else {
        3.0 - 4.0 * phase
    }
}

message_handlers!(impl DSawOscillator {});
