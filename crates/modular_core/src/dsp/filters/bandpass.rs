use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::types::{Clickless, Signal};

#[derive(Deserialize, Default, JsonSchema, Connect)]
#[serde(default)]
struct BandpassFilterParams {
    /// signal input
    input: Signal,
    /// center frequency in v/oct
    center: Signal,
    /// filter Q (bandwidth control, 0-5)
    resonance: Signal,
}

#[derive(Outputs, JsonSchema)]
struct BandpassFilterOutputs {
    #[output("output", "filtered signal", default)]
    sample: f32,
}

#[derive(Default, Module)]
#[module("bpf", "12dB/octave bandpass filter")]
#[args(input, center, resonance?)]
pub struct BandpassFilter {
    outputs: BandpassFilterOutputs,
    // State variables for 2-pole filter
    z1: f32,
    z2: f32,
    center: Clickless,
    q: Clickless,
    params: BandpassFilterParams,
}

impl BandpassFilter {
    fn update(&mut self, sample_rate: f32) -> () {
        let input = self.params.input.get_value();
        self.center.update(self.params.center.get_value_or(4.0));
        self.q.update(self.params.resonance.get_value_or(1.0));

        // Convert v/oct to frequency
        let freq = 27.5f32 * 2.0f32.powf(*self.center);
        let freq_clamped = freq.min(sample_rate * 0.45).max(20.0);

        // Calculate filter coefficients
        let omega = 2.0 * std::f32::consts::PI * freq_clamped / sample_rate;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let q = (*self.q / 5.0 * 9.0 + 0.5).max(0.5);
        let alpha = sin_omega / (2.0 * q);

        let b0 = alpha;
        let b1 = 0.0;
        let b2 = -alpha;
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

message_handlers!(impl BandpassFilter {});
