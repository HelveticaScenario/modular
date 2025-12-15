use anyhow::{anyhow, Result};
use crate::types::{ChannelBuffer, InternalParam, NUM_CHANNELS};

#[derive(Default, Params)]
struct BandpassFilterParams {
    #[param("input", "signal input")]
    input: InternalParam,
    #[param("center", "center frequency in v/oct")]
    center: InternalParam,
    #[param("q", "filter Q (bandwidth control, 0-5)")]
    q: InternalParam,
}

#[derive(Default, Module)]
#[module("bpf", "12dB/octave bandpass filter")]
pub struct BandpassFilter {
    #[output("output", "filtered signal", default)]
    sample: ChannelBuffer,
    // State variables for 2-pole filter
    z1: ChannelBuffer,
    z2: ChannelBuffer,
    smoothed_center: ChannelBuffer,
    smoothed_q: ChannelBuffer,
    params: BandpassFilterParams,
}

impl BandpassFilter {
    fn update(&mut self, sample_rate: f32) -> () {
        let mut input = ChannelBuffer::default();
        let mut target_center = [4.0; NUM_CHANNELS];
        let mut target_q = [1.0; NUM_CHANNELS];

        self.params.input.get_value(&mut input);
        self.params
            .center
            .get_value_or(&mut target_center, &[4.0; NUM_CHANNELS]);
        self.params.q.get_value_or(&mut target_q, &[1.0; NUM_CHANNELS]);

        crate::types::smooth_buffer(&mut self.smoothed_center, &target_center);
        crate::types::smooth_buffer(&mut self.smoothed_q, &target_q);

        let sr = sample_rate.max(1.0);
        for i in 0..NUM_CHANNELS {
            let freq = 27.5f32 * self.smoothed_center[i].exp2();
            let freq_clamped = freq.min(sr * 0.45).max(20.0);

            let omega = 2.0 * std::f32::consts::PI * freq_clamped / sr;
            let (sin_omega, cos_omega) = omega.sin_cos();
            let q = (self.smoothed_q[i] / 5.0 * 9.0 + 0.5).max(0.5);
            let alpha = sin_omega / (2.0 * q);

            let b0 = alpha;
            let b2 = -alpha;
            let a0 = 1.0 + alpha;
            let a1 = -2.0 * cos_omega;
            let a2 = 1.0 - alpha;

            let b0_norm = b0 / a0;
            let b2_norm = b2 / a0;
            let a1_norm = a1 / a0;
            let a2_norm = a2 / a0;

            let w = input[i] - a1_norm * self.z1[i] - a2_norm * self.z2[i];
            let y = b0_norm * w + b2_norm * self.z2[i];
            self.z2[i] = self.z1[i];
            self.z1[i] = w;
            self.sample[i] = y.clamp(-5.0, 5.0);
        }
    }
}
