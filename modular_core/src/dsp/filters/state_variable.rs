use anyhow::{anyhow, Result};
use crate::types::InternalParam;

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
    lowpass: f32,
    #[output("bandpass", "bandpass output")]
    bandpass: f32,
    #[output("highpass", "highpass output")]
    highpass: f32,
    // State variables
    z1_low: f32,
    z1_band: f32,
    smoothed_cutoff: f32,
    smoothed_resonance: f32,
    params: StateVariableFilterParams,
}

impl StateVariableFilter {
    fn update(&mut self, sample_rate: f32) -> () {
        let input = self.params.input.get_value();
        let target_cutoff = self.params.cutoff.get_value_or(4.0);
        let target_resonance = self.params.resonance.get_value_or(0.0);
        
        self.smoothed_cutoff = crate::types::smooth_value(self.smoothed_cutoff, target_cutoff);
        self.smoothed_resonance = crate::types::smooth_value(self.smoothed_resonance, target_resonance);
        
        // Convert v/oct to frequency
        let freq = 27.5f32 * 2.0f32.powf(self.smoothed_cutoff);
        let freq_clamped = freq.min(sample_rate * 0.45).max(20.0);
        
        // Calculate filter coefficients
        let f = 2.0 * (std::f32::consts::PI * freq_clamped / sample_rate).sin();
        let q = 1.0 - (self.smoothed_resonance / 5.0 * 0.95);
        let q_clamped = q.max(0.05);
        
        // State-variable filter topology
        self.highpass = input - self.z1_low - q_clamped * self.z1_band;
        self.bandpass = f * self.highpass + self.z1_band;
        self.lowpass = f * self.bandpass + self.z1_low;
        
        // Update state
        self.z1_band = self.bandpass;
        self.z1_low = self.lowpass;
        
        // Soft clipping to prevent overflow
        self.lowpass = self.lowpass.clamp(-5.0, 5.0);
        self.bandpass = self.bandpass.clamp(-5.0, 5.0);
        self.highpass = self.highpass.clamp(-5.0, 5.0);
    }
}
