use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    PORT_MAX_CHANNELS, dsp::utils::{changed, voct_to_hz}, poly::{PolyOutput, PolySignal}
};

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default)]
struct BandpassFilterParams {
    /// signal input
    input: PolySignal,
    /// center frequency in v/oct
    center: PolySignal,
    /// filter Q (bandwidth control, 0-5)
    resonance: PolySignal,
}

#[derive(Outputs, JsonSchema)]
struct BandpassFilterOutputs {
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
struct BpfChannelState {
    z1: f32,
    z2: f32,
    coeffs: BiquadCoeffs,
    last_center: f32,
    last_q: f32,
}

fn compute_bpf_biquad(center: f32, resonance: f32, sample_rate: f32) -> BiquadCoeffs {
    let freq = voct_to_hz(center);
    let freq = freq.min(sample_rate * 0.45).max(20.0);

    let omega = 2.0 * std::f32::consts::PI * freq / sample_rate;
    let sin_omega = omega.sin();
    let cos_omega = omega.cos();
    let q = (resonance / 5.0 * 9.0 + 0.5).max(0.5);
    let alpha = sin_omega / (2.0 * q);

    let b0 = alpha;
    let b1 = 0.0;
    let b2 = -alpha;
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

#[derive(Module)]
#[module("bpf", "12dB/octave bandpass filter")]
#[args(input, center, resonance?)]
pub struct BandpassFilter {
    outputs: BandpassFilterOutputs,
    channels: [BpfChannelState; PORT_MAX_CHANNELS],
    // For mono optimization
    coeffs_mono: BiquadCoeffs,
    last_center_mono: f32,
    last_q_mono: f32,
    params: BandpassFilterParams,
}

impl Default for BandpassFilter {
    fn default() -> Self {
        Self {
            outputs: Default::default(),
            channels: [BpfChannelState::default(); PORT_MAX_CHANNELS],
            coeffs_mono: BiquadCoeffs::default(),
            last_center_mono: 0.0,
            last_q_mono: 0.0,
            params: Default::default(),
        }
    }
}

impl BandpassFilter {
    fn update(&mut self, sample_rate: f32) {
        let num_channels = self.channel_count();
        self.outputs.sample.set_channels(num_channels);

        // Update coefficients
        if self.params.center.is_monophonic() && self.params.resonance.is_monophonic() {
            let c = self.params.center.get_value_or(0, 4.0);
            let r = self.params.resonance.get_value_or(0, 1.0);

            if changed(c, self.last_center_mono) || changed(r, self.last_q_mono) {
                self.coeffs_mono = compute_bpf_biquad(c, r, sample_rate);
                self.last_center_mono = c;
                self.last_q_mono = r;
            }
        } else {
            for i in 0..num_channels {
                let c = self.params.center.get_value_or(i, 4.0);
                let r = self.params.resonance.get_value_or(i, 1.0);

                if changed(c, self.channels[i].last_center)
                    || changed(r, self.channels[i].last_q)
                {
                    self.channels[i].coeffs = compute_bpf_biquad(c, r, sample_rate);
                    self.channels[i].last_center = c;
                    self.channels[i].last_q = r;
                }
            }
        }

        for i in 0..num_channels {
            let input = self.params.input.get_value_or(i, 0.0);

            let c = if self.params.center.is_monophonic() && self.params.resonance.is_monophonic() {
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

message_handlers!(impl BandpassFilter {});
