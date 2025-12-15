use anyhow::{anyhow, Result};
use crate::types::{ChannelBuffer, InternalParam, NUM_CHANNELS};

#[derive(Default, Params)]
struct StateVariableFilterParams {
    #[param("input", "signal input")]
    input: InternalParam,
    #[param("cutoff", "cutoff frequency in v/oct")]
    cutoff: InternalParam,
    #[param("q", "filter resonance (0-5)")]
    resonance: InternalParam,
}

#[derive(Default, Module)]
#[module("stateVariable", "State-variable filter with LP/BP/HP outputs")]
pub struct StateVariableFilter {
    #[output("lowpass", "lowpass output")]
    lowpass: ChannelBuffer,
    #[output("bandpass", "bandpass output")]
    bandpass: ChannelBuffer,
    #[output("highpass", "highpass output")]
    highpass: ChannelBuffer,
    // State variables
    z1_low: ChannelBuffer,
    z1_band: ChannelBuffer,
    smoothed_cutoff: ChannelBuffer,
    smoothed_resonance: ChannelBuffer,
    params: StateVariableFilterParams,
}

impl StateVariableFilter {
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

            let f = 2.0 * (std::f32::consts::PI * freq_clamped / sr).sin();
            let q = 1.0 - (self.smoothed_resonance[i] / 5.0 * 0.95);
            let q_clamped = q.max(0.05);

            let highpass = input[i] - self.z1_low[i] - q_clamped * self.z1_band[i];
            let bandpass = f * highpass + self.z1_band[i];
            let lowpass = f * bandpass + self.z1_low[i];

            self.z1_band[i] = bandpass;
            self.z1_low[i] = lowpass;

            self.lowpass[i] = lowpass.clamp(-5.0, 5.0);
            self.bandpass[i] = bandpass.clamp(-5.0, 5.0);
            self.highpass[i] = highpass.clamp(-5.0, 5.0);
        }
    }
}
