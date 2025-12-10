use anyhow::{Result, anyhow};

use crate::{
    dsp::utils::{clamp, wrap},
    types::InternalParam,
};

/// Parameters for the Plaits FM engine
/// Based on the 2-operator FM synthesis from Mutable Instruments Plaits
#[derive(Default, Params)]
struct PlaitsFMParams {
    #[param("freq", "frequency in v/oct")]
    freq: InternalParam,
    #[param("harmonics", "carrier/modulator ratio (0.0-1.0)")]
    harmonics: InternalParam,
    #[param("timbre", "modulation amount (0.0-1.0)")]
    timbre: InternalParam,
    #[param("morph", "feedback amount (-1.0 to 1.0)")]
    morph: InternalParam,
}

/// Plaits FM Engine
/// 
/// Two-operator FM synthesis inspired by Mutable Instruments Plaits.
/// Features:
/// - Variable carrier/modulator ratio controlled by harmonics
/// - Modulation depth controlled by timbre
/// - Feedback controlled by morph (negative values add phase offset)
#[derive(Default, Module)]
#[module("plaits-fm", "Plaits FM synthesis engine (2-op FM)")]
pub struct PlaitsFM {
    #[output("output", "main FM output", default)]
    sample: f32,
    #[output("aux", "sub-octave output")]
    aux_sample: f32,
    
    carrier_phase: f32,
    modulator_phase: f32,
    sub_phase: f32,
    
    smoothed_freq: f32,
    smoothed_mod_freq: f32,
    smoothed_amount: f32,
    smoothed_feedback: f32,
    
    previous_sample: f32,
    
    params: PlaitsFMParams,
}

impl PlaitsFM {
    /// Compute modulator frequency ratio from harmonics parameter
    /// Maps 0.0-1.0 to musically useful ratios
    fn compute_ratio(harmonics: f32) -> f32 {
        // Quantized FM ratios (similar to Plaits)
        let ratios = [
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0,  // Octaves and harmonics
            1.0/2.0, 1.0/3.0, 1.0/4.0,  // Sub-harmonics
            3.0/2.0, 4.0/3.0, 5.0/4.0,  // Intervals
        ];
        
        let scaled = harmonics * (ratios.len() - 1) as f32;
        let index = scaled.floor() as usize;
        let frac = scaled - scaled.floor();
        
        if index >= ratios.len() - 1 {
            ratios[ratios.len() - 1]
        } else {
            // Linear interpolation between ratios
            ratios[index] + (ratios[index + 1] - ratios[index]) * frac
        }
    }
    
    /// Fast sine approximation using polynomial
    fn fast_sin(phase: f32) -> f32 {
        // Phase should be 0.0 to 1.0
        let x = phase * 2.0 - 1.0; // Map to -1.0 to 1.0
        
        // Polynomial approximation
        let sign = if x >= 0.0 { 1.0 } else { -1.0 };
        let abs_x = x.abs();
        sign * (abs_x * (4.0 - 4.0 * abs_x))
    }
    
    fn update(&mut self, sample_rate: f32) {
        // Get parameters with defaults
        let freq_v = clamp(-10.0, 10.0, self.params.freq.get_value_or(4.0));
        let harmonics = clamp(0.0, 1.0, self.params.harmonics.get_value_or(0.5));
        let timbre = clamp(0.0, 1.0, self.params.timbre.get_value_or(0.5));
        let morph = clamp(-1.0, 1.0, self.params.morph.get_value_or(0.0));
        
        // Smooth parameters to avoid clicks
        self.smoothed_freq = crate::types::smooth_value(self.smoothed_freq, freq_v);
        
        // Convert v/oct to Hz
        let carrier_hz = 27.5 * 2.0f32.powf(self.smoothed_freq);
        let ratio = Self::compute_ratio(harmonics);
        let modulator_hz = carrier_hz * ratio;
        
        // Convert to phase increment (normalized to 0.0-1.0 per sample)
        let carrier_inc = carrier_hz / sample_rate;
        let modulator_inc = modulator_hz / sample_rate;
        let sub_inc = carrier_hz / (2.0 * sample_rate);  // One octave down
        
        // Smooth modulation parameters
        let target_amount = timbre * timbre * 2.0;  // Square for better control
        self.smoothed_amount = crate::types::smooth_value(self.smoothed_amount, target_amount);
        
        let target_feedback = morph;
        self.smoothed_feedback = crate::types::smooth_value(self.smoothed_feedback, target_feedback);
        
        // Calculate feedback/phase offset
        let phase_feedback = if self.smoothed_feedback < 0.0 {
            0.5 * self.smoothed_feedback * self.smoothed_feedback
        } else {
            0.0
        };
        
        let amplitude_feedback = if self.smoothed_feedback > 0.0 {
            self.smoothed_feedback * self.previous_sample
        } else {
            0.0
        };
        
        // Update modulator phase
        self.modulator_phase += modulator_inc;
        self.modulator_phase = wrap(0.0..1.0, self.modulator_phase);
        
        // Calculate modulator output with feedback
        let mod_phase_with_fb = wrap(0.0..1.0, 
            self.modulator_phase + phase_feedback + amplitude_feedback);
        let modulator_out = Self::fast_sin(mod_phase_with_fb);
        
        // Update carrier phase with FM
        let fm_offset = modulator_out * self.smoothed_amount;
        self.carrier_phase += carrier_inc;
        self.carrier_phase = wrap(0.0..1.0, self.carrier_phase);
        
        let carrier_phase_with_fm = wrap(0.0..1.0, self.carrier_phase + fm_offset);
        
        // Output main signal (scaled to ±5V range)
        self.sample = 5.0 * Self::fast_sin(carrier_phase_with_fm);
        self.previous_sample = self.sample;
        
        // Update sub-oscillator (one octave down)
        self.sub_phase += sub_inc;
        self.sub_phase = wrap(0.0..1.0, self.sub_phase);
        self.aux_sample = 5.0 * Self::fast_sin(self.sub_phase);
    }
}
