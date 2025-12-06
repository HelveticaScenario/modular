use anyhow::{anyhow, Result};
use crate::types::InternalParam;

#[derive(Default, Params)]
struct MoogLadderFilterParams {
    #[param("input", "signal input")]
    input: InternalParam,
    #[param("cutoff", "cutoff frequency in v/oct")]
    cutoff: InternalParam,
    #[param("resonance", "filter resonance (0-5)")]
    resonance: InternalParam,
}

#[derive(Default, Module)]
#[module("moog-ladder-filter", "24dB/octave Moog-style ladder filter")]
pub struct MoogLadderFilter {
    #[output("output", "filtered signal", default)]
    sample: f32,
    // State variables for 4-pole (24dB/oct) ladder filter
    stage: [f32; 4],
    delay: [f32; 4],
    smoothed_cutoff: f32,
    smoothed_resonance: f32,
    params: MoogLadderFilterParams,
}

impl MoogLadderFilter {
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
        let fc = freq_clamped / sample_rate;
        let f = fc * 1.16;
        let fb = self.smoothed_resonance / 5.0 * 4.0;
        
        // Input with feedback
        let input_fb = input - self.sample * fb;
        
        // Tanh saturation for non-linearity (simplified)
        let saturate = |x: f32| {
            if x > 1.0 { 1.0 }
            else if x < -1.0 { -1.0 }
            else { x }
        };
        
        // Process through 4 one-pole stages
        for i in 0..4 {
            let stage_input = if i == 0 { input_fb } else { self.stage[i - 1] };
            self.stage[i] = self.delay[i] + f * (saturate(stage_input) - self.delay[i]);
            self.delay[i] = self.stage[i];
        }
        
        self.sample = self.stage[3];
        
        // Soft clipping to prevent overflow
        self.sample = self.sample.clamp(-5.0, 5.0);
    }
}
