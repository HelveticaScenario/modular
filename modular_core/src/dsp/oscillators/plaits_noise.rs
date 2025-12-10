use anyhow::{Result, anyhow};

use crate::{
    dsp::utils::clamp,
    types::InternalParam,
};

/// Parameters for the Plaits Noise engine
#[derive(Default, Params)]
struct PlaitsNoiseParams {
    #[param("freq", "filter frequency in v/oct")]
    freq: InternalParam,
    #[param("harmonics", "frequency spread/metallic character")]
    harmonics: InternalParam,
    #[param("timbre", "noise color (low to high frequency)")]
    timbre: InternalParam,
    #[param("morph", "resonance/feedback amount")]
    morph: InternalParam,
}

/// Plaits Noise Engine
/// 
/// Noise/particle synthesis inspired by Mutable Instruments Plaits.
/// Features:
/// - Variable noise color (white to filtered)
/// - Resonant filtering
/// - Metallic/clocked noise modes
#[derive(Module)]
#[module("plaits-noise", "Plaits noise/particle synthesis engine")]
pub struct PlaitsNoise {
    #[output("output", "main noise output", default)]
    sample: f32,
    #[output("aux", "alternative noise texture")]
    aux_sample: f32,
    
    // Filter state for main output
    filter_lp: f32,
    filter_bp: f32,
    filter_hp: f32,
    
    // Filter state for aux output
    aux_filter_lp: f32,
    aux_filter_bp: f32,
    aux_filter_hp: f32,
    
    // Clocked noise
    clock_phase: f32,
    hold_sample: f32,
    
    smoothed_freq: f32,
    
    // Random number generator state
    rng_state: u32,
    
    params: PlaitsNoiseParams,
}

impl Default for PlaitsNoise {
    fn default() -> Self {
        Self {
            sample: 0.0,
            aux_sample: 0.0,
            filter_lp: 0.0,
            filter_bp: 0.0,
            filter_hp: 0.0,
            aux_filter_lp: 0.0,
            aux_filter_bp: 0.0,
            aux_filter_hp: 0.0,
            clock_phase: 0.0,
            hold_sample: 0.0,
            smoothed_freq: 4.0,
            rng_state: 54321,
            params: PlaitsNoiseParams::default(),
        }
    }
}

impl PlaitsNoise {
    /// Generate random noise sample (-1.0 to 1.0)
    fn random(&mut self) -> f32 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 17;
        self.rng_state ^= self.rng_state << 5;
        (self.rng_state as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
    
    /// State variable filter (SVF)
    /// Returns (lowpass, bandpass, highpass)
    fn svf_tick(
        input: f32,
        frequency: f32,
        resonance: f32,
        lp: &mut f32,
        bp: &mut f32,
        hp: &mut f32,
        sample_rate: f32,
    ) -> (f32, f32, f32) {
        let f = 2.0 * (frequency / sample_rate * std::f32::consts::PI).sin();
        let q = 1.0 - resonance;
        
        *hp = input - *lp - q * *bp;
        *bp += f * *hp;
        *lp += f * *bp;
        
        (*lp, *bp, *hp)
    }
    
    fn update(&mut self, sample_rate: f32) {
        // Get parameters with defaults
        let freq_v = clamp(-10.0, 10.0, self.params.freq.get_value_or(4.0));
        let harmonics = clamp(0.0, 1.0, self.params.harmonics.get_value_or(0.5));
        let timbre = clamp(0.0, 1.0, self.params.timbre.get_value_or(0.5));
        let morph = clamp(0.0, 1.0, self.params.morph.get_value_or(0.0));
        
        // Smooth frequency
        self.smoothed_freq = crate::types::smooth_value(self.smoothed_freq, freq_v);
        
        // Convert v/oct to Hz for filter
        let base_hz = 27.5 * 2.0f32.powf(self.smoothed_freq);
        let filter_freq = clamp(20.0, sample_rate * 0.45, base_hz);
        
        // Resonance from morph
        let resonance = morph * 0.95;  // 0 to 0.95
        
        // Generate noise
        let noise = self.random();
        
        // Metallic/clocked mode based on harmonics
        let metallic_amount = harmonics;
        
        if metallic_amount > 0.1 {
            // Clocked noise - sample and hold
            let clock_freq = filter_freq * (1.0 + metallic_amount * 3.0);
            let clock_inc = clock_freq / sample_rate;
            
            self.clock_phase += clock_inc;
            if self.clock_phase >= 1.0 {
                self.clock_phase -= 1.0;
                self.hold_sample = noise;
            }
            
            let metallic_noise = self.hold_sample;
            
            // Blend between smooth and metallic
            let mixed_noise = noise * (1.0 - metallic_amount) + metallic_noise * metallic_amount;
            
            // Apply filtering
            let (lp, bp, hp) = Self::svf_tick(
                mixed_noise,
                filter_freq,
                resonance,
                &mut self.filter_lp,
                &mut self.filter_bp,
                &mut self.filter_hp,
                sample_rate,
            );
            
            // Mix filter outputs based on timbre
            let output = if timbre < 0.33 {
                // Low frequencies
                lp
            } else if timbre < 0.66 {
                // Mid frequencies (bandpass)
                let mix = (timbre - 0.33) * 3.0;
                lp * (1.0 - mix) + bp * mix
            } else {
                // High frequencies
                let mix = (timbre - 0.66) * 3.0;
                bp * (1.0 - mix) + hp * mix
            };
            
            self.sample = 5.0 * output;
            
            // Aux output: different filter configuration
            let (_, aux_bp, _) = Self::svf_tick(
                metallic_noise,
                filter_freq * 2.0,
                resonance * 0.5,
                &mut self.aux_filter_lp,
                &mut self.aux_filter_bp,
                &mut self.aux_filter_hp,
                sample_rate,
            );
            self.aux_sample = 5.0 * aux_bp;
            
        } else {
            // Smooth filtered noise
            let (lp, bp, hp) = Self::svf_tick(
                noise,
                filter_freq,
                resonance,
                &mut self.filter_lp,
                &mut self.filter_bp,
                &mut self.filter_hp,
                sample_rate,
            );
            
            // Color selection
            let output = if timbre < 0.33 {
                lp
            } else if timbre < 0.66 {
                let mix = (timbre - 0.33) * 3.0;
                lp * (1.0 - mix) + bp * mix
            } else {
                let mix = (timbre - 0.66) * 3.0;
                bp * (1.0 - mix) + hp * mix
            };
            
            self.sample = 5.0 * output;
            
            // Aux: just high-passed noise
            self.aux_sample = 5.0 * hp;
        }
    }
}
