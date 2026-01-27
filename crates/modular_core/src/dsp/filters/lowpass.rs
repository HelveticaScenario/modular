use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    PORT_MAX_CHANNELS, poly::{PolyOutput, PolySignal},
    dsp::utils::changed,
};

#[derive(Deserialize, Default, JsonSchema, Connect)]
#[serde(default)]
struct LowpassFilterParams {
    /// signal input
    input: PolySignal,
    /// cutoff frequency in v/oct
    cutoff: PolySignal,
    /// filter resonance (0-5)
    resonance: PolySignal,
}

#[derive(Outputs, JsonSchema)]
struct LowpassFilterOutputs {
    #[output("output", "filtered signal", default)]
    sample: PolyOutput,
}

#[derive(Default, Module)]
#[module("lpf", "12dB/octave lowpass filter with resonance")]
#[args(input, cutoff, resonance?)]
pub struct LowpassFilter {
    outputs: LowpassFilterOutputs,
    // Per-channel state (audio-rate)
    z1: [f32; PORT_MAX_CHANNELS],
    z2: [f32; PORT_MAX_CHANNELS],

    // Cached coefficients (control-rate)
    coeffs: [BiquadCoeffs; PORT_MAX_CHANNELS],

    // Last seen params (for change detection)
    last_cutoff: [f32; PORT_MAX_CHANNELS],
    last_resonance: [f32; PORT_MAX_CHANNELS],

    // For mono optimization
    coeffs_mono: BiquadCoeffs,
    last_cutoff_mono: f32,
    last_resonance_mono: f32,

    params: LowpassFilterParams,
}

#[derive(Clone, Copy, Default)]
struct BiquadCoeffs {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

fn compute_biquad(cutoff: f32, resonance: f32, sample_rate: f32) -> BiquadCoeffs {
    let freq = 55.0 * 2.0f32.powf(cutoff);
    let freq = freq.min(sample_rate * 0.45).max(20.0);

    let omega = 2.0 * std::f32::consts::PI * freq / sample_rate;
    let sin = omega.sin();
    let cos = omega.cos();
    let q = (resonance / 5.0 * 9.0 + 0.5).max(0.5);
    let alpha = sin / (2.0 * q);

    let b0 = (1.0 - cos) / 2.0;
    let b1 = 1.0 - cos;
    let b2 = (1.0 - cos) / 2.0;
    let a0 = 1.0 + alpha;
    let a1 = -2.0 * cos;
    let a2 = 1.0 - alpha;

    BiquadCoeffs {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    }
}

impl LowpassFilter {
    fn update_coeffs(
        &mut self,
        cutoff: &PolySignal,
        resonance: &PolySignal,
        channels: usize,
        sample_rate: f32,
    ) {
        if cutoff.is_monophonic() && resonance.is_monophonic() {
            let c = cutoff.get_value_or(0, 0.0);
            let r = resonance.get_value_or(0, 0.0);

            if changed(c, self.last_cutoff_mono) || changed(r, self.last_resonance_mono) {
                self.coeffs_mono = compute_biquad(c, r, sample_rate);
                self.last_cutoff_mono = c;
                self.last_resonance_mono = r;
            }
        } else {
            for i in 0..channels {
                let c = cutoff.get_value_or(i, 0.0);
                let r = resonance.get_value_or(i, 0.0);

                if changed(c, self.last_cutoff[i]) || changed(r, self.last_resonance[i]) {
                    self.coeffs[i] = compute_biquad(c, r, sample_rate);
                    self.last_cutoff[i] = c;
                    self.last_resonance[i] = r;
                }
            }
        }
    }

    fn update(&mut self, sample_rate: f32) -> () {
        let channels = PolySignal::max_channels(&[
            &self.params.input,
            &self.params.cutoff,
            &self.params.resonance,
        ]);

        self.outputs.sample.set_channels(channels);

        // Update coefficients (borrows cutoff and resonance for reading)
        if self.params.cutoff.is_monophonic() && self.params.resonance.is_monophonic() {
            let c = self.params.cutoff.get_value_or(0, 0.0);
            let r = self.params.resonance.get_value_or(0, 0.0);

            if changed(c, self.last_cutoff_mono) || changed(r, self.last_resonance_mono) {
                self.coeffs_mono = compute_biquad(c, r, sample_rate);
                self.last_cutoff_mono = c;
                self.last_resonance_mono = r;
            }
        } else {
            for i in 0..channels as usize {
                let c = self.params.cutoff.get_value_or(i, 0.0);
                let r = self.params.resonance.get_value_or(i, 0.0);

                if changed(c, self.last_cutoff[i]) || changed(r, self.last_resonance[i]) {
                    self.coeffs[i] = compute_biquad(c, r, sample_rate);
                    self.last_cutoff[i] = c;
                    self.last_resonance[i] = r;
                }
            }
        }

        for i in 0..channels as usize {
            let input = self.params.input.get_value_or(i, 0.0);

            let c = if self.params.cutoff.is_monophonic() && self.params.resonance.is_monophonic() {
                self.coeffs_mono
            } else {
                self.coeffs[i]
            };

            let w = input - c.a1 * self.z1[i] - c.a2 * self.z2[i];
            let y = c.b0 * w + c.b1 * self.z1[i] + c.b2 * self.z2[i];

            self.z2[i] = self.z1[i];
            self.z1[i] = w;
            self.outputs.sample.set(i, y);
        }
    }
}

message_handlers!(impl LowpassFilter {});
