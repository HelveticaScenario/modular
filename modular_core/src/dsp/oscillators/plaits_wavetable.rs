use anyhow::{Result, anyhow};

use crate::{
    dsp::utils::{clamp, wrap},
    types::InternalParam,
};

/// Parameters for the Plaits Wavetable engine
#[derive(Default, Params)]
struct PlaitsWavetableParams {
    #[param("freq", "frequency in v/oct")]
    freq: InternalParam,
    #[param("harmonics", "wavetable bank selection")]
    harmonics: InternalParam,
    #[param("timbre", "wave selection within bank")]
    timbre: InternalParam,
    #[param("morph", "wave interpolation/morphing")]
    morph: InternalParam,
}

/// Plaits Wavetable Engine
/// 
/// Wavetable synthesis inspired by Mutable Instruments Plaits.
/// Features:
/// - Multiple wavetable banks
/// - Smooth interpolation between waves
/// - Wave morphing
#[derive(Default, Module)]
#[module("plaits-wavetable", "Plaits wavetable synthesis engine")]
pub struct PlaitsWavetable {
    #[output("output", "main wavetable output", default)]
    sample: f32,
    
    phase: f32,
    smoothed_freq: f32,
    
    params: PlaitsWavetableParams,
}

impl PlaitsWavetable {
    /// Generate a single wavetable with various harmonic content
    /// wave_type: 0=saw-rich, 1=square-like, 2=triangle-like, 3=sine-like
    fn generate_wave(phase: f32, wave_type: i32, harmonics: i32) -> f32 {
        let mut sum = 0.0;
        let max_harmonics = (harmonics + 1).min(16);  // Limit to prevent aliasing
        
        match wave_type {
            0 => {
                // Saw-like (all harmonics, 1/n amplitude)
                for n in 1..=max_harmonics {
                    let amp = 1.0 / n as f32;
                    sum += amp * (phase * n as f32 * 2.0 * std::f32::consts::PI).sin();
                }
                sum * 0.5
            },
            1 => {
                // Square-like (odd harmonics, 1/n amplitude)
                for n in 0..max_harmonics {
                    let harmonic = 2 * n + 1;
                    let amp = 1.0 / harmonic as f32;
                    sum += amp * (phase * harmonic as f32 * 2.0 * std::f32::consts::PI).sin();
                }
                sum * 0.7
            },
            2 => {
                // Triangle-like (odd harmonics, 1/n² amplitude, alternating phase)
                for n in 0..max_harmonics {
                    let harmonic = 2 * n + 1;
                    let amp = 1.0 / (harmonic * harmonic) as f32;
                    let sign = if n % 2 == 0 { 1.0 } else { -1.0 };
                    sum += sign * amp * (phase * harmonic as f32 * 2.0 * std::f32::consts::PI).sin();
                }
                sum * 1.5
            },
            3 => {
                // Sine
                (phase * 2.0 * std::f32::consts::PI).sin()
            },
            _ => 0.0,
        }
    }
    
    /// Interpolate between two wavetables
    fn interpolate_waves(phase: f32, wave_a: i32, wave_b: i32, harmonics: i32, mix: f32) -> f32 {
        let sample_a = Self::generate_wave(phase, wave_a, harmonics);
        let sample_b = Self::generate_wave(phase, wave_b, harmonics);
        sample_a * (1.0 - mix) + sample_b * mix
    }
    
    fn update(&mut self, sample_rate: f32) {
        // Get parameters with defaults
        let freq_v = clamp(-10.0, 10.0, self.params.freq.get_value_or(4.0));
        let harmonics = clamp(0.0, 1.0, self.params.harmonics.get_value_or(0.5));
        let timbre = clamp(0.0, 1.0, self.params.timbre.get_value_or(0.5));
        let morph = clamp(0.0, 1.0, self.params.morph.get_value_or(0.0));
        
        // Smooth frequency
        self.smoothed_freq = crate::types::smooth_value(self.smoothed_freq, freq_v);
        
        // Convert v/oct to Hz
        let frequency = 27.5 * 2.0f32.powf(self.smoothed_freq);
        
        // Update phase
        let phase_inc = frequency / sample_rate;
        self.phase += phase_inc;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        
        // Select wavetable bank (number of harmonics)
        let harmonic_count = 1 + (harmonics * 15.0) as i32;  // 1 to 16 harmonics
        
        // Select wave within bank
        let wave_index = timbre * 3.999;  // 0.0 to 3.999
        let wave_a = wave_index.floor() as i32;
        let wave_b = (wave_a + 1).min(3);
        let wave_mix = wave_index - wave_index.floor();
        
        // Generate sample with interpolation
        let base_sample = Self::interpolate_waves(
            self.phase,
            wave_a,
            wave_b,
            harmonic_count,
            wave_mix,
        );
        
        // Apply morphing (phase distortion)
        let morph_amount = morph * 0.8;  // Limit to prevent extreme distortion
        let morphed_phase = if morph_amount > 0.01 {
            let distorted = self.phase + morph_amount * (self.phase * 2.0 * std::f32::consts::PI).sin();
            wrap(0.0..1.0, distorted)
        } else {
            self.phase
        };
        
        let morphed_sample = if morph_amount > 0.01 {
            Self::interpolate_waves(
                morphed_phase,
                wave_a,
                wave_b,
                harmonic_count,
                wave_mix,
            )
        } else {
            base_sample
        };
        
        // Blend between normal and morphed based on morph amount
        let output = if morph_amount > 0.01 {
            base_sample * (1.0 - morph_amount) + morphed_sample * morph_amount
        } else {
            base_sample
        };
        
        // Scale to ±5V range
        self.sample = 5.0 * output;
    }
}
