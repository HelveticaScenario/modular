use anyhow::{anyhow, Result};
use crate::types::InternalParam;

#[derive(Default, Params)]
struct AllpassFilterParams {
    #[param("input", "signal input")]
    input: InternalParam,
    #[param("frequency", "frequency in v/oct")]
    frequency: InternalParam,
    #[param("q", "filter Q (0-5)")]
    q: InternalParam,
}

#[derive(Default, Module)]
#[module("apf", "Allpass filter for phase shifting")]
pub struct AllpassFilter {
    #[output("output", "phase-shifted signal", default)]
    sample: f32,
    // State variables for 2-pole filter
    z1: f32,
    z2: f32,
    smoothed_frequency: f32,
    smoothed_q: f32,
    params: AllpassFilterParams,
}

impl AllpassFilter {
    fn update(&mut self, sample_rate: f32) -> () {
        let input = self.params.input.get_value();
        let target_frequency = self.params.frequency.get_value_or(4.0);
        let target_q = self.params.q.get_value_or(1.0);
        
        self.smoothed_frequency = crate::types::smooth_value(self.smoothed_frequency, target_frequency);
        self.smoothed_q = crate::types::smooth_value(self.smoothed_q, target_q);
        
        // Convert v/oct to frequency
        let freq = 27.5f32 * 2.0f32.powf(self.smoothed_frequency);
        let freq_clamped = freq.min(sample_rate * 0.45).max(20.0);
        
        // Calculate filter coefficients
        let omega = 2.0 * std::f32::consts::PI * freq_clamped / sample_rate;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        let q = (self.smoothed_q / 5.0 * 9.0 + 0.5).max(0.5);
        let alpha = sin_omega / (2.0 * q);
        
        let b0 = 1.0 - alpha;
        let b1 = -2.0 * cos_omega;
        let b2 = 1.0 + alpha;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha;
        
        // Normalize coefficients
        let b0_norm = b0 / a0;
        let b1_norm = b1 / a0;
        let b2_norm = b2 / a0;
        let a1_norm = a1 / a0;
        let a2_norm = a2 / a0;
        
        // Process sample (Direct Form II)
        let w = input - a1_norm * self.z1 - a2_norm * self.z2;
        self.sample = b0_norm * w + b1_norm * self.z1 + b2_norm * self.z2;
        self.z2 = self.z1;
        self.z1 = w;
        
        // Soft clipping to prevent overflow
        self.sample = self.sample.clamp(-5.0, 5.0);
    }
}
