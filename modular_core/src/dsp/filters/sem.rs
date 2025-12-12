use anyhow::{anyhow, Result};
use crate::types::{ChannelBuffer, InternalParam, NUM_CHANNELS};

#[derive(Default, Params)]
struct SEMFilterParams {
    #[param("input", "signal input")]
    input: InternalParam,
    #[param("cutoff", "cutoff frequency in v/oct")]
    cutoff: InternalParam,
    #[param("q", "filter resonance (0-5)")]
    resonance: InternalParam,
    #[param("mode", "filter mode: 0=LP, 1=BP, 2=HP, 3=Notch")]
    mode: InternalParam,
}

#[derive(Default, Module)]
#[module("sem", "Oberheim SEM style multi-mode filter")]
pub struct SEMFilter {
    #[output("output", "filtered signal", default)]
    sample: ChannelBuffer,
    // State variables
    z1_low: ChannelBuffer,
    z1_band: ChannelBuffer,
    smoothed_cutoff: ChannelBuffer,
    smoothed_resonance: ChannelBuffer,
    smoothed_mode: ChannelBuffer,
    params: SEMFilterParams,
}

impl SEMFilter {
    fn update(&mut self, sample_rate: f32) -> () {
        let mut input = ChannelBuffer::default();
        let mut target_cutoff = [4.0; NUM_CHANNELS];
        let mut target_resonance = ChannelBuffer::default();
        let mut target_mode = ChannelBuffer::default();

        self.params.input.get_value(&mut input);
        self.params
            .cutoff
            .get_value_or(&mut target_cutoff, &[4.0; NUM_CHANNELS]);
        self.params.resonance.get_value(&mut target_resonance);
        self.params.mode.get_value(&mut target_mode);

        crate::types::smooth_buffer(&mut self.smoothed_cutoff, &target_cutoff);
        crate::types::smooth_buffer(&mut self.smoothed_resonance, &target_resonance);
        crate::types::smooth_buffer(&mut self.smoothed_mode, &target_mode);

        let sr = sample_rate.max(1.0);
        for i in 0..NUM_CHANNELS {
            let freq = 27.5f32 * self.smoothed_cutoff[i].exp2();
            let freq_clamped = freq.min(sr * 0.45).max(20.0);

            let g = (std::f32::consts::PI * freq_clamped / sr).tan();
            let k = 2.0 - 2.0 * (self.smoothed_resonance[i] / 5.0 * 0.99);
            let k_clamped = k.max(0.01);

            let highpass = (input[i] - self.z1_low[i] - k_clamped * self.z1_band[i])
                / (1.0 + g * k_clamped + g * g);
            let bandpass = g * highpass + self.z1_band[i];
            let lowpass = g * bandpass + self.z1_low[i];

            self.z1_band[i] = g * highpass + bandpass;
            self.z1_low[i] = g * bandpass + lowpass;

            let tanh_sat = |x: f32| x.clamp(-1.5, 1.5).tanh();
            let mode_normalized = (self.smoothed_mode[i] / 5.0 * 3.0).clamp(0.0, 2.999);
            let mode_int = mode_normalized.floor() as i32;
            let mode_frac = mode_normalized.fract();

            let y = match mode_int {
                0 => tanh_sat(lowpass * (1.0 - mode_frac) + bandpass * mode_frac),
                1 => tanh_sat(bandpass * (1.0 - mode_frac) + highpass * mode_frac),
                2 => {
                    let notch = lowpass + highpass;
                    tanh_sat(highpass * (1.0 - mode_frac) + notch * mode_frac)
                }
                _ => lowpass,
            };

            self.sample[i] = y.clamp(-5.0, 5.0);
        }
    }
}
