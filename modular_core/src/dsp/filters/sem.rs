use anyhow::{anyhow, Result};
use crate::types::InternalParam;

#[derive(Default, Params)]
struct SEMFilterParams {
    #[param("input", "signal input")]
    input: InternalParam,
    #[param("cutoff", "cutoff frequency in v/oct")]
    cutoff: InternalParam,
    #[param("resonance", "filter resonance (0-5)")]
    resonance: InternalParam,
    #[param("mode", "filter mode: 0=LP, 1=BP, 2=HP, 3=Notch")]
    mode: InternalParam,
}

#[derive(Default, Module)]
#[module("sem-filter", "Oberheim SEM style multi-mode filter")]
pub struct SEMFilter {
    #[output("output", "filtered signal", default)]
    sample: f32,
    // State variables
    z1_low: f32,
    z1_band: f32,
    smoothed_cutoff: f32,
    smoothed_resonance: f32,
    smoothed_mode: f32,
    params: SEMFilterParams,
}

impl SEMFilter {
    fn update(&mut self, sample_rate: f32) -> () {
        let input = self.params.input.get_value();
        let target_cutoff = self.params.cutoff.get_value_or(4.0);
        let target_resonance = self.params.resonance.get_value_or(0.0);
        let target_mode = self.params.mode.get_value_or(0.0);
        
        self.smoothed_cutoff = crate::types::smooth_value(self.smoothed_cutoff, target_cutoff);
        self.smoothed_resonance = crate::types::smooth_value(self.smoothed_resonance, target_resonance);
        self.smoothed_mode = crate::types::smooth_value(self.smoothed_mode, target_mode);
        
        // Convert v/oct to frequency
        let freq = 27.5f32 * 2.0f32.powf(self.smoothed_cutoff);
        let freq_clamped = freq.min(sample_rate * 0.45).max(20.0);
        
        // Calculate filter coefficients with SEM-style response
        let g = (std::f32::consts::PI * freq_clamped / sample_rate).tan();
        let k = 2.0 - 2.0 * (self.smoothed_resonance / 5.0 * 0.99);
        let k_clamped = k.max(0.01);
        
        // State variable filter with feedback
        let highpass = (input - self.z1_low - k_clamped * self.z1_band) / (1.0 + g * k_clamped + g * g);
        let bandpass = g * highpass + self.z1_band;
        let lowpass = g * bandpass + self.z1_low;
        
        // Update state
        self.z1_band = g * highpass + bandpass;
        self.z1_low = g * bandpass + lowpass;
        
        // SEM-style non-linear mixing adds character
        let tanh_sat = |x: f32| x.clamp(-1.5, 1.5).tanh();
        
        // Mix between modes based on mode parameter
        let mode_normalized = (self.smoothed_mode / 5.0 * 3.0).clamp(0.0, 2.999);
        let mode_int = mode_normalized.floor() as i32;
        let mode_frac = mode_normalized.fract();
        
        self.sample = match mode_int {
            0 => {
                // LP to BP
                tanh_sat(lowpass * (1.0 - mode_frac) + bandpass * mode_frac)
            }
            1 => {
                // BP to HP
                tanh_sat(bandpass * (1.0 - mode_frac) + highpass * mode_frac)
            }
            2 => {
                // HP to Notch
                let notch = lowpass + highpass;
                tanh_sat(highpass * (1.0 - mode_frac) + notch * mode_frac)
            }
            _ => lowpass,
        };
        
        self.sample = self.sample.clamp(-5.0, 5.0);
    }
}
