use anyhow::{anyhow, Result};
use crate::types::InternalParam;

#[derive(Default, Params)]
struct FormantFilterParams {
    #[param("input", "signal input")]
    input: InternalParam,
    #[param("formant", "formant selection (0-5): a, e, i, o, u, mixed")]
    formant: InternalParam,
    #[param("morph", "morph between formants (0-5)")]
    morph: InternalParam,
    #[param("q", "resonance/Q factor (0-5)")]
    q: InternalParam,
}

#[derive(Default, Module)]
#[module("formant-filter", "Vowel formant filter for vocal-like sounds")]
pub struct FormantFilter {
    #[output("output", "formant filtered signal")]
    sample: f32,
    // Three parallel bandpass filters for formants
    bp1_z1: f32,
    bp1_z2: f32,
    bp2_z1: f32,
    bp2_z2: f32,
    bp3_z1: f32,
    bp3_z2: f32,
    smoothed_formant: f32,
    smoothed_morph: f32,
    smoothed_q: f32,
    params: FormantFilterParams,
}

impl FormantFilter {
    fn update(&mut self, sample_rate: f32) -> () {
        let input = self.params.input.get_value();
        let target_formant = self.params.formant.get_value_or(0.0);
        let target_morph = self.params.morph.get_value_or(0.0);
        let target_q = self.params.q.get_value_or(2.5);
        
        self.smoothed_formant = crate::types::smooth_value(self.smoothed_formant, target_formant);
        self.smoothed_morph = crate::types::smooth_value(self.smoothed_morph, target_morph);
        self.smoothed_q = crate::types::smooth_value(self.smoothed_q, target_q);
        
        // Formant frequencies for different vowels (in Hz)
        // Format: [F1, F2, F3] for each vowel
        let vowel_formants = [
            [730.0, 1090.0, 2440.0], // A (ah)
            [530.0, 1840.0, 2480.0], // E (eh)
            [390.0, 1990.0, 2550.0], // I (ee)
            [570.0, 840.0, 2410.0],  // O (oh)
            [440.0, 1020.0, 2240.0], // U (oo)
        ];
        
        // Select vowel based on formant parameter
        let formant_idx = (self.smoothed_formant / 5.0 * 4.0).floor() as usize;
        let formant_idx = formant_idx.min(4);
        let next_idx = (formant_idx + 1).min(4);
        let morph_amount = (self.smoothed_formant / 5.0 * 4.0).fract();
        
        // Interpolate between vowels
        let f1 = vowel_formants[formant_idx][0] * (1.0 - morph_amount) 
                + vowel_formants[next_idx][0] * morph_amount;
        let f2 = vowel_formants[formant_idx][1] * (1.0 - morph_amount) 
                + vowel_formants[next_idx][1] * morph_amount;
        let f3 = vowel_formants[formant_idx][2] * (1.0 - morph_amount) 
                + vowel_formants[next_idx][2] * morph_amount;
        
        // Apply morph parameter to shift all formants
        let shift = 1.0 + (self.smoothed_morph / 5.0 - 0.5) * 0.5;
        let f1_shifted = (f1 * shift).clamp(100.0, sample_rate * 0.45);
        let f2_shifted = (f2 * shift).clamp(100.0, sample_rate * 0.45);
        let f3_shifted = (f3 * shift).clamp(100.0, sample_rate * 0.45);
        
        let q_val = (self.smoothed_q / 5.0 * 9.0 + 2.0).max(0.5);
        
        // Process through three bandpass filters
        let process_bandpass = |freq: f32, z1: &mut f32, z2: &mut f32| -> f32 {
            let omega = 2.0 * std::f32::consts::PI * freq / sample_rate;
            let sin_omega = omega.sin();
            let cos_omega = omega.cos();
            let alpha = sin_omega / (2.0 * q_val);
            
            let b0 = alpha;
            let b2 = -alpha;
            let a0 = 1.0 + alpha;
            let a1 = -2.0 * cos_omega;
            let a2 = 1.0 - alpha;
            
            let b0_norm = b0 / a0;
            let b2_norm = b2 / a0;
            let a1_norm = a1 / a0;
            let a2_norm = a2 / a0;
            
            let w = input - a1_norm * *z1 - a2_norm * *z2;
            let output = b0_norm * w + b2_norm * *z2;
            *z2 = *z1;
            *z1 = w;
            
            output
        };
        
        let bp1 = process_bandpass(f1_shifted, &mut self.bp1_z1, &mut self.bp1_z2);
        let bp2 = process_bandpass(f2_shifted, &mut self.bp2_z1, &mut self.bp2_z2);
        let bp3 = process_bandpass(f3_shifted, &mut self.bp3_z1, &mut self.bp3_z2);
        
        // Mix the three formants with emphasis on lower formants
        self.sample = (bp1 * 1.5 + bp2 * 1.0 + bp3 * 0.5) / 3.0;
        
        self.sample = self.sample.clamp(-5.0, 5.0);
    }
}
