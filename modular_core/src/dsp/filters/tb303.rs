use anyhow::{anyhow, Result};
use crate::types::InternalParam;

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
    sample: f32,
    // State variables for 4-pole cascade
    z1: f32,
    z2: f32,
    z3: f32,
    z4: f32,
    smoothed_cutoff: f32,
    smoothed_resonance: f32,
    smoothed_env_mod: f32,
    params: TB303FilterParams,
}

impl TB303Filter {
    fn update(&mut self, sample_rate: f32) -> () {
        let input = self.params.input.get_value();
        let target_cutoff = self.params.cutoff.get_value_or(4.0);
        let target_resonance = self.params.resonance.get_value_or(0.0);
        let target_env_mod = self.params.env_mod.get_value_or(0.0);
        
        self.smoothed_cutoff = crate::types::smooth_value(self.smoothed_cutoff, target_cutoff);
        self.smoothed_resonance = crate::types::smooth_value(self.smoothed_resonance, target_resonance);
        self.smoothed_env_mod = crate::types::smooth_value(self.smoothed_env_mod, target_env_mod);
        
        // Apply envelope modulation to cutoff
        let modulated_cutoff = self.smoothed_cutoff + self.smoothed_env_mod * 2.0;
        
        // Convert v/oct to frequency
        let freq = 27.5f32 * 2.0f32.powf(modulated_cutoff);
        let freq_clamped = freq.min(sample_rate * 0.45).max(20.0);
        
        // TB-303 style resonance (very aggressive at high settings)
        let fc = (freq_clamped / sample_rate * std::f32::consts::PI).tan();
        let res = self.smoothed_resonance / 5.0;
        let fb = res * 4.0 + 0.1; // Feedback amount
        
        // Non-linear saturation function
        let saturate = |x: f32| {
            let x_clamped = x.clamp(-2.0, 2.0);
            x_clamped - (x_clamped * x_clamped * x_clamped) / 3.0
        };
        
        // Input with feedback
        let input_fb = saturate(input - self.z4 * fb);
        
        // Four one-pole stages with non-linearity
        let g = fc / (1.0 + fc);
        
        self.z1 = self.z1 + g * (saturate(input_fb) - self.z1);
        self.z2 = self.z2 + g * (saturate(self.z1) - self.z2);
        self.z3 = self.z3 + g * (saturate(self.z2) - self.z3);
        self.z4 = self.z4 + g * (saturate(self.z3) - self.z4);
        
        self.sample = self.z4;
        
        // Soft clipping
        self.sample = self.sample.clamp(-5.0, 5.0);
    }
}
