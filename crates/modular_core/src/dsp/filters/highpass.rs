use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::types::{Clickless, Signal};

#[derive(Deserialize, Default, JsonSchema, Connect)]
#[serde(default)]
struct HighpassFilterParams {
    /// signal input
    input: Signal,
    /// cutoff frequency in v/oct
    cutoff: Signal,
    /// filter resonance (0-5)
    resonance: Signal,
}

#[derive(Outputs, JsonSchema)]
struct HighpassFilterOutputs {
    #[output("output", "filtered signal", default)]
    sample: f32,
}

#[derive(Default, Module)]
#[module("hpf", "12dB/octave highpass filter with resonance")]
#[args(input, cutoff, resonance?)]
pub struct HighpassFilter {
    outputs: HighpassFilterOutputs,
    // State variables for 2-pole (12dB/oct) filter
    z1: f32,
    z2: f32,
    cutoff: Clickless,
    resonance: Clickless,
    params: HighpassFilterParams,
}

impl HighpassFilter {
    fn update(&mut self, sample_rate: f32) -> () {
        let input = self.params.input.get_value();

        self.cutoff.update(self.params.cutoff.get_value_or(4.0));
        self.resonance
            .update(self.params.resonance.get_value_or(0.0));

        // Convert v/oct to frequency
        let freq = 27.5f32 * 2.0f32.powf(*self.cutoff);
        let freq_clamped = freq.min(sample_rate * 0.45).max(20.0);

        // Calculate filter coefficients
        let omega = 2.0 * std::f32::consts::PI * freq_clamped / sample_rate;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let q = (*self.resonance / 5.0 * 9.0 + 0.5).max(0.5);
        let alpha = sin_omega / (2.0 * q);

        let b0 = (1.0 + cos_omega) / 2.0;
        let b1 = -(1.0 + cos_omega);
        let b2 = (1.0 + cos_omega) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha;

        // Normalize coefficients
        let b0_norm = b0 / a0;
        let b1_norm = b1 / a0;
        let b2_norm = b2 / a0;
        let a1_norm = a1 / a0;
        let a2_norm = a2 / a0;

        // Process sample (Direct Form II)
        let w = input - a1_norm * self.z1 - a2_norm * self.z2;
        self.outputs.sample = b0_norm * w + b1_norm * self.z1 + b2_norm * self.z2;
        self.z2 = self.z1;
        self.z1 = w;

        // Soft clipping to prevent overflow
        // self.sample = self.sample.clamp(-5.0, 5.0);
    }
}

message_handlers!(impl HighpassFilter {});
