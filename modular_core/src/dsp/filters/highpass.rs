use anyhow::{anyhow, Result};
use crate::types::{ChannelBuffer, InternalParam, NUM_CHANNELS};

#[derive(Default, Params)]
struct HighpassFilterParams {
    #[param("input", "signal input")]
    input: InternalParam,
    #[param("cutoff", "cutoff frequency in v/oct")]
    cutoff: InternalParam,
    #[param("q", "filter resonance (0-5)")]
    resonance: InternalParam,
}

#[derive(Default, Module)]
#[module("hpf", "12dB/octave highpass filter with resonance")]
pub struct HighpassFilter {
    #[output("output", "filtered signal", default)]
    sample: ChannelBuffer,
    // State variables for 2-pole (12dB/oct) filter
    z1: ChannelBuffer,
    z2: ChannelBuffer,
    smoothed_cutoff: ChannelBuffer,
    smoothed_resonance: ChannelBuffer,
    params: HighpassFilterParams,
}

impl HighpassFilter {
    fn update(&mut self, sample_rate: f32) -> () {
        let mut input = ChannelBuffer::default();
        let mut target_cutoff = [4.0; NUM_CHANNELS];
        let mut target_resonance = ChannelBuffer::default();

        self.params.input.get_value(&mut input);
        self.params
            .cutoff
            .get_value_or(&mut target_cutoff, &[4.0; NUM_CHANNELS]);
        self.params.resonance.get_value(&mut target_resonance);

        crate::types::smooth_buffer(&mut self.smoothed_cutoff, &target_cutoff);
        crate::types::smooth_buffer(&mut self.smoothed_resonance, &target_resonance);

        let sr = sample_rate.max(1.0);
        for i in 0..NUM_CHANNELS {
            let freq = 27.5f32 * self.smoothed_cutoff[i].exp2();
            let freq_clamped = freq.min(sr * 0.45).max(20.0);

            let omega = 2.0 * std::f32::consts::PI * freq_clamped / sr;
            let (sin_omega, cos_omega) = omega.sin_cos();
            let q = (self.smoothed_resonance[i] / 5.0 * 9.0 + 0.5).max(0.5);
            let alpha = sin_omega / (2.0 * q);

            let b0 = (1.0 + cos_omega) / 2.0;
            let b1 = -(1.0 + cos_omega);
            let b2 = (1.0 + cos_omega) / 2.0;
            let a0 = 1.0 + alpha;
            let a1 = -2.0 * cos_omega;
            let a2 = 1.0 - alpha;

            let b0_norm = b0 / a0;
            let b1_norm = b1 / a0;
            let b2_norm = b2 / a0;
            let a1_norm = a1 / a0;
            let a2_norm = a2 / a0;

            let w = input[i] - a1_norm * self.z1[i] - a2_norm * self.z2[i];
            let y = b0_norm * w + b1_norm * self.z1[i] + b2_norm * self.z2[i];
            self.z2[i] = self.z1[i];
            self.z1[i] = w;
            self.sample[i] = y.clamp(-5.0, 5.0);
        }
    }
}
