use anyhow::{anyhow, Result};
use crate::types::{ChannelBuffer, InternalParam, NUM_CHANNELS};

#[derive(Default, Params)]
struct TB303FilterParams {
    #[param("input", "signal input")]
    input: InternalParam,
    #[param("cutoff", "cutoff frequency in v/oct")]
    cutoff: InternalParam,
    #[param("q", "filter resonance (0-5)")]
    resonance: InternalParam,
    #[param("envMod", "envelope modulation amount")]
    env_mod: InternalParam,
}

#[derive(Default, Module)]
#[module("tb303", "TB-303 style 24dB/octave lowpass with aggressive resonance")]
pub struct TB303Filter {
    #[output("output", "filtered signal", default)]
    sample: ChannelBuffer,
    // State variables for 4-pole cascade
    z1: ChannelBuffer,
    z2: ChannelBuffer,
    z3: ChannelBuffer,
    z4: ChannelBuffer,
    smoothed_cutoff: ChannelBuffer,
    smoothed_resonance: ChannelBuffer,
    smoothed_env_mod: ChannelBuffer,
    params: TB303FilterParams,
}

impl TB303Filter {
    fn update(&mut self, sample_rate: f32) -> () {
        let mut input = ChannelBuffer::default();
        let mut target_cutoff = [4.0; NUM_CHANNELS];
        let mut target_resonance = ChannelBuffer::default();
        let mut target_env_mod = ChannelBuffer::default();

        self.params.input.get_value(&mut input);
        self.params
            .cutoff
            .get_value_or(&mut target_cutoff, &[4.0; NUM_CHANNELS]);
        self.params.resonance.get_value(&mut target_resonance);
        self.params.env_mod.get_value(&mut target_env_mod);

        crate::types::smooth_buffer(&mut self.smoothed_cutoff, &target_cutoff);
        crate::types::smooth_buffer(&mut self.smoothed_resonance, &target_resonance);
        crate::types::smooth_buffer(&mut self.smoothed_env_mod, &target_env_mod);

        let sr = sample_rate.max(1.0);
        let saturate = |x: f32| {
            let x_clamped = x.clamp(-2.0, 2.0);
            x_clamped - (x_clamped * x_clamped * x_clamped) / 3.0
        };

        let mut g_per_channel = [0.0f32; NUM_CHANNELS];
        let mut input_fb_per_channel = [0.0f32; NUM_CHANNELS];

        for i in 0..NUM_CHANNELS {
            let modulated_cutoff = self.smoothed_cutoff[i] + self.smoothed_env_mod[i] * 2.0;

            let freq = 27.5f32 * modulated_cutoff.exp2();
            let freq_clamped = freq.min(sr * 0.45).max(20.0);

            let fc = (freq_clamped / sr * std::f32::consts::PI).tan();
            let res = self.smoothed_resonance[i] / 5.0;
            let fb = res * 4.0 + 0.1;

            g_per_channel[i] = fc / (1.0 + fc);
            input_fb_per_channel[i] = saturate(input[i] - self.z4[i] * fb);
        }

        for i in 0..NUM_CHANNELS {
            let g = g_per_channel[i];
            self.z1[i] = self.z1[i] + g * (saturate(input_fb_per_channel[i]) - self.z1[i]);
        }
        for i in 0..NUM_CHANNELS {
            let g = g_per_channel[i];
            self.z2[i] = self.z2[i] + g * (saturate(self.z1[i]) - self.z2[i]);
        }
        for i in 0..NUM_CHANNELS {
            let g = g_per_channel[i];
            self.z3[i] = self.z3[i] + g * (saturate(self.z2[i]) - self.z3[i]);
        }
        for i in 0..NUM_CHANNELS {
            let g = g_per_channel[i];
            self.z4[i] = self.z4[i] + g * (saturate(self.z3[i]) - self.z4[i]);
            self.sample[i] = self.z4[i].clamp(-5.0, 5.0);
        }
    }
}
