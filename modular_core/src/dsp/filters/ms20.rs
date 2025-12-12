use anyhow::{anyhow, Result};
use crate::types::{ChannelBuffer, InternalParam, NUM_CHANNELS};

#[derive(Default, Params)]
struct MS20FilterParams {
    #[param("input", "signal input")]
    input: InternalParam,
    #[param("cutoff", "cutoff frequency in v/oct")]
    cutoff: InternalParam,
    #[param("q", "filter resonance (0-5)")]
    resonance: InternalParam,
}

#[derive(Default, Module)]
#[module("ms20", "Korg MS-20 style lowpass with aggressive distortion")]
pub struct MS20Filter {
    #[output("output", "filtered signal", default)]
    sample: ChannelBuffer,
    // State variables for 2-pole filter
    z1: ChannelBuffer,
    z2: ChannelBuffer,
    smoothed_cutoff: ChannelBuffer,
    smoothed_resonance: ChannelBuffer,
    params: MS20FilterParams,
}

impl MS20Filter {
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

            let fc = 2.0 * (std::f32::consts::PI * freq_clamped / sr).sin();
            let res = self.smoothed_resonance[i] / 5.0;
            let fb = res * 4.5;

            let clip = |x: f32| {
                if x > 1.0 {
                    1.0
                } else if x < -1.0 {
                    -1.0
                } else {
                    x - x * x * x / 3.0
                }
            };

            let input_fb = input[i] - clip(self.z2[i] * fb);
            let hp = input_fb - self.z1[i];
            self.z1[i] = self.z1[i] + fc * clip(hp);

            let hp2 = self.z1[i] - self.z2[i];
            self.z2[i] = self.z2[i] + fc * clip(hp2);

            let y = clip(self.z2[i]).clamp(-5.0, 5.0);
            self.sample[i] = y;
        }
    }
}
