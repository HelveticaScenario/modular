use anyhow::{anyhow, Result};
use crate::types::InternalParam;

#[derive(Default, Params)]
struct SallenKeyFilterParams {
    #[param("input", "signal input")]
    input: InternalParam,
    #[param("cutoff", "cutoff frequency in v/oct")]
    cutoff: InternalParam,
    #[param("q", "filter resonance (0-5)")]
    resonance: InternalParam,
    #[param("type", "filter type: 0=LP, 1=HP, 2=BP")]
    filter_type: InternalParam,
}

#[derive(Default, Module)]
#[module("sallenKey", "Sallen-Key topology filter with smooth response")]
pub struct SallenKeyFilter {
    #[output("output", "filtered signal", default)]
    sample: f32,
    // State variables for 2-pole Sallen-Key topology
    z1: f32,
    z2: f32,
    smoothed_cutoff: f32,
    smoothed_resonance: f32,
    smoothed_type: f32,
    params: SallenKeyFilterParams,
}

impl SallenKeyFilter {
    fn update(&mut self, sample_rate: f32) -> () {
        let input = self.params.input.get_value();
        let target_cutoff = self.params.cutoff.get_value_or(4.0);
        let target_resonance = self.params.resonance.get_value_or(0.0);
        let target_type = self.params.filter_type.get_value_or(0.0);
        
        self.smoothed_cutoff = crate::types::smooth_value(self.smoothed_cutoff, target_cutoff);
        self.smoothed_resonance = crate::types::smooth_value(self.smoothed_resonance, target_resonance);
        self.smoothed_type = crate::types::smooth_value(self.smoothed_type, target_type);
        
        // Convert v/oct to frequency
        let freq = 27.5f32 * 2.0f32.powf(self.smoothed_cutoff);
        let freq_clamped = freq.min(sample_rate * 0.45).max(20.0);
        
        // Sallen-Key has very smooth, musical resonance
        let omega = 2.0 * std::f32::consts::PI * freq_clamped / sample_rate;
        let sin_omega = omega.sin();
        let cos_omega = omega.cos();
        
        // Damping factor (inversely related to resonance)
        let q = (self.smoothed_resonance / 5.0 * 9.0 + 0.7).max(0.5);
        let alpha = sin_omega / (2.0 * q);
        
        // Determine filter type
        let filter_mode = (self.smoothed_type / 5.0 * 2.0).floor() as i32;
        
        let (b0, b1, b2) = match filter_mode {
            0 => {
                // Lowpass - Sallen-Key has very smooth rolloff
                (
                    (1.0 - cos_omega) / 2.0,
                    1.0 - cos_omega,
                    (1.0 - cos_omega) / 2.0,
                )
            }
            1 => {
                // Highpass
                (
                    (1.0 + cos_omega) / 2.0,
                    -(1.0 + cos_omega),
                    (1.0 + cos_omega) / 2.0,
                )
            }
            _ => {
                // Bandpass
                (alpha, 0.0, -alpha)
            }
        };
        
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha;
        
        // Normalize coefficients
        let b0_norm = b0 / a0;
        let b1_norm = b1 / a0;
        let b2_norm = b2 / a0;
        let a1_norm = a1 / a0;
        let a2_norm = a2 / a0;
        
        // Process sample (Direct Form II - optimal for Sallen-Key)
        let w = input - a1_norm * self.z1 - a2_norm * self.z2;
        self.sample = b0_norm * w + b1_norm * self.z1 + b2_norm * self.z2;
        
        // Update state
        self.z2 = self.z1;
        self.z1 = w;
        
        // Very gentle soft clipping to preserve Sallen-Key's smooth character
        self.sample = self.sample.clamp(-5.0, 5.0);
    }
}
