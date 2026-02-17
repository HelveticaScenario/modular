use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    PORT_MAX_CHANNELS,
    dsp::utils::{changed, voct_to_hz},
    poly::{PolyOutput, PolySignal},
};

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default, rename_all = "camelCase")]
struct HighpassFilterParams {
    /// signal input
    input: PolySignal,
    /// cutoff frequency in V/Oct (0V = C4)
    cutoff: PolySignal,
    /// filter resonance (0-5)
    resonance: PolySignal,
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct HighpassFilterOutputs {
    #[output("output", "filtered signal", default)]
    sample: PolyOutput,
}

#[derive(Clone, Copy, Default)]
struct BiquadCoeffs {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
}

#[derive(Default, Clone, Copy)]
struct HpfChannelState {
    z1: f32,
    z2: f32,
    coeffs: BiquadCoeffs,
    last_cutoff: f32,
    last_resonance: f32,
}

fn compute_hpf_biquad(cutoff: f32, resonance: f32, sample_rate: f32) -> BiquadCoeffs {
    let freq = voct_to_hz(cutoff);
    let freq = freq.min(sample_rate * 0.45).max(20.0);

    let omega = 2.0 * std::f32::consts::PI * freq / sample_rate;
    let sin_omega = omega.sin();
    let cos_omega = omega.cos();
    let q = (resonance / 5.0 * 9.0 + 0.5).max(0.5);
    let alpha = sin_omega / (2.0 * q);

    let b0 = (1.0 + cos_omega) / 2.0;
    let b1 = -(1.0 + cos_omega);
    let b2 = (1.0 + cos_omega) / 2.0;
    let a0 = 1.0 + alpha;
    let a1 = -2.0 * cos_omega;
    let a2 = 1.0 - alpha;

    BiquadCoeffs {
        b0: b0 / a0,
        b1: b1 / a0,
        b2: b2 / a0,
        a1: a1 / a0,
        a2: a2 / a0,
    }
}

/// Highpass filter that attenuates frequencies below the cutoff point.
///
/// Use it to remove low-end rumble, thin out a sound, or create rising
/// filter effects. Pairs well with lowpass filters for isolating a
/// frequency band.
///
/// - **cutoff** — set in V/Oct (0 V = C4). Accepts modulation for filter sweeps.
/// - **resonance** — boosts frequencies near the cutoff (0–5). High values
///   produce a ringing peak.
///
/// ```js
/// // remove low end from a noise source
/// $hpf($noise("white"), 'a3', 1)
/// ```
#[module(name = "$hpf", args(input, cutoff, resonance?))]
#[derive(Default)]
pub struct HighpassFilter {
    outputs: HighpassFilterOutputs,
    channels: [HpfChannelState; PORT_MAX_CHANNELS],
    // For mono optimization
    coeffs_mono: BiquadCoeffs,
    last_cutoff_mono: f32,
    last_resonance_mono: f32,
    params: HighpassFilterParams,
}

impl HighpassFilter {
    fn update(&mut self, sample_rate: f32) {
        let num_channels = self.channel_count();

        // Update coefficients
        if self.params.cutoff.is_monophonic() && self.params.resonance.is_monophonic() {
            let c = self.params.cutoff.get_value_or(0, 4.0);
            let r = self.params.resonance.get_value_or(0, 0.0);

            if changed(c, self.last_cutoff_mono) || changed(r, self.last_resonance_mono) {
                self.coeffs_mono = compute_hpf_biquad(c, r, sample_rate);
                self.last_cutoff_mono = c;
                self.last_resonance_mono = r;
            }
        } else {
            for i in 0..num_channels {
                let c = self.params.cutoff.get_value_or(i, 4.0);
                let r = self.params.resonance.get_value_or(i, 0.0);

                if changed(c, self.channels[i].last_cutoff)
                    || changed(r, self.channels[i].last_resonance)
                {
                    self.channels[i].coeffs = compute_hpf_biquad(c, r, sample_rate);
                    self.channels[i].last_cutoff = c;
                    self.channels[i].last_resonance = r;
                }
            }
        }

        for i in 0..num_channels {
            let input = self.params.input.get_value_or(i, 0.0);

            let c = if self.params.cutoff.is_monophonic() && self.params.resonance.is_monophonic() {
                self.coeffs_mono
            } else {
                self.channels[i].coeffs
            };

            let state = &mut self.channels[i];
            let w = input - c.a1 * state.z1 - c.a2 * state.z2;
            let y = c.b0 * w + c.b1 * state.z1 + c.b2 * state.z2;

            state.z2 = state.z1;
            state.z1 = w;
            self.outputs.sample.set(i, y);
        }
    }
}

message_handlers!(impl HighpassFilter {});
