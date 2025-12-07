use anyhow::{anyhow, Result};
use crate::types::InternalParam;

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
    sample: f32,
    // State variables for 2-pole filter
    z1: f32,
    z2: f32,
    smoothed_cutoff: f32,
    smoothed_resonance: f32,
    params: MS20FilterParams,
}

impl MS20Filter {
    fn update(&mut self, sample_rate: f32) -> () {
        let input = self.params.input.get_value();
        let target_cutoff = self.params.cutoff.get_value_or(4.0);
        let target_resonance = self.params.resonance.get_value_or(0.0);
        
        self.smoothed_cutoff = crate::types::smooth_value(self.smoothed_cutoff, target_cutoff);
        self.smoothed_resonance = crate::types::smooth_value(self.smoothed_resonance, target_resonance);
        
        // Convert v/oct to frequency
        let freq = 27.5f32 * 2.0f32.powf(self.smoothed_cutoff);
        let freq_clamped = freq.min(sample_rate * 0.45).max(20.0);
        
        // MS-20 has very aggressive resonance
        let fc = 2.0 * (std::f32::consts::PI * freq_clamped / sample_rate).sin();
        let res = self.smoothed_resonance / 5.0;
        let fb = res * 4.5; // Very high feedback for MS-20 character
        
        // MS-20 style hard clipping distortion
        let clip = |x: f32| {
            if x > 1.0 {
                1.0
            } else if x < -1.0 {
                -1.0
            } else {
                // Soft saturation below clipping threshold
                x - x * x * x / 3.0
            }
        };
        
        // Input with aggressive feedback
        let input_fb = input - clip(self.z2 * fb);
        
        // Two pole filter with clipping between stages
        let hp = input_fb - self.z1;
        self.z1 = self.z1 + fc * clip(hp);
        
        let hp2 = self.z1 - self.z2;
        self.z2 = self.z2 + fc * clip(hp2);
        
        self.sample = self.z2;
        
        // Final stage clipping
        self.sample = clip(self.sample).clamp(-5.0, 5.0);
    }
}
