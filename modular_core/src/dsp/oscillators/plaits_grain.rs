use anyhow::{Result, anyhow};

use crate::{
    dsp::utils::clamp,
    types::InternalParam,
};

/// Parameters for the Plaits Grain engine
#[derive(Default, Params)]
struct PlaitsGrainParams {
    #[param("freq", "base frequency in v/oct")]
    freq: InternalParam,
    #[param("harmonics", "grain frequency/density")]
    harmonics: InternalParam,
    #[param("timbre", "grain shape and size")]
    timbre: InternalParam,
    #[param("morph", "randomization amount")]
    morph: InternalParam,
}

/// Single grain generator
#[derive(Clone)]
struct Grain {
    phase: f32,
    envelope_phase: f32,
    frequency: f32,
    size: f32,
    active: bool,
}

impl Grain {
    fn new() -> Self {
        Self {
            phase: 0.0,
            envelope_phase: 0.0,
            frequency: 440.0,
            size: 0.1,
            active: false,
        }
    }
    
    /// Trigger a new grain
    fn trigger(&mut self, frequency: f32, size: f32) {
        self.phase = 0.0;
        self.envelope_phase = 0.0;
        self.frequency = frequency;
        self.size = clamp(0.001, 1.0, size);
        self.active = true;
    }
    
    /// Generate one sample from the grain
    fn process(&mut self, sample_rate: f32) -> f32 {
        if !self.active {
            return 0.0;
        }
        
        // Update oscillator phase
        self.phase += self.frequency / sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        
        // Sine wave oscillator
        let osc = (self.phase * 2.0 * std::f32::consts::PI).sin();
        
        // Hann window envelope
        let env_inc = 1.0 / (self.size * sample_rate);
        self.envelope_phase += env_inc;
        
        if self.envelope_phase >= 1.0 {
            self.active = false;
            return 0.0;
        }
        
        let env = (1.0 - (self.envelope_phase * std::f32::consts::PI).cos()) * 0.5;
        
        osc * env
    }
}

/// Plaits Grain Engine
/// 
/// Granular synthesis inspired by Mutable Instruments Plaits.
/// Features:
/// - Multiple overlapping grains
/// - Variable grain size and density
/// - Frequency modulation
/// - Random grain parameters
#[derive(Module)]
#[module("plaits-grain", "Plaits granular synthesis engine")]
pub struct PlaitsGrain {
    #[output("output", "main granular output", default)]
    sample: f32,
    #[output("aux", "alternative grain texture")]
    aux_sample: f32,
    
    grains: Vec<Grain>,
    grain_counter: f32,
    
    smoothed_freq: f32,
    
    // Simple random number generator state
    rng_state: u32,
    
    params: PlaitsGrainParams,
}

impl Default for PlaitsGrain {
    fn default() -> Self {
        Self {
            sample: 0.0,
            aux_sample: 0.0,
            grains: vec![Grain::new(); 8],  // 8 concurrent grains
            grain_counter: 0.0,
            smoothed_freq: 4.0,
            rng_state: 12345,
            params: PlaitsGrainParams::default(),
        }
    }
}

impl PlaitsGrain {
    /// Simple pseudo-random number generator (xorshift)
    fn random(&mut self) -> f32 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 17;
        self.rng_state ^= self.rng_state << 5;
        self.rng_state as f32 / u32::MAX as f32
    }
    
    fn update(&mut self, sample_rate: f32) {
        // Get parameters with defaults
        let freq_v = clamp(-10.0, 10.0, self.params.freq.get_value_or(4.0));
        let harmonics = clamp(0.0, 1.0, self.params.harmonics.get_value_or(0.5));
        let timbre = clamp(0.0, 1.0, self.params.timbre.get_value_or(0.5));
        let morph = clamp(0.0, 1.0, self.params.morph.get_value_or(0.0));
        
        // Smooth frequency
        self.smoothed_freq = crate::types::smooth_value(self.smoothed_freq, freq_v);
        
        // Base frequency
        let base_hz = 27.5 * 2.0f32.powf(self.smoothed_freq);
        
        // Grain density (grains per second)
        let density = 10.0 + harmonics * 190.0;  // 10 to 200 grains/sec
        let grain_interval = sample_rate / density;
        
        // Grain size (duration in seconds)
        let grain_size = 0.01 + timbre * 0.49;  // 10ms to 500ms
        
        // Randomization amount
        let randomness = morph;
        
        // Trigger new grains
        self.grain_counter += 1.0;
        if self.grain_counter >= grain_interval {
            self.grain_counter -= grain_interval;
            
            // Generate random values first before iterating
            let random1 = self.random();
            let random2 = self.random();
            
            // Find an inactive grain to trigger
            for grain in self.grains.iter_mut() {
                if !grain.active {
                    // Calculate grain frequency with randomization
                    let freq_variation = if randomness > 0.01 {
                        let random_semitones = (random1 - 0.5) * randomness * 24.0;
                        2.0f32.powf(random_semitones / 12.0)
                    } else {
                        1.0
                    };
                    
                    let grain_freq = base_hz * freq_variation;
                    
                    // Size variation
                    let size_variation = if randomness > 0.01 {
                        0.8 + random2 * 0.4 * randomness
                    } else {
                        1.0
                    };
                    
                    let grain_duration = grain_size * size_variation;
                    
                    grain.trigger(grain_freq, grain_duration);
                    break;
                }
            }
        }
        
        // Process all grains
        let mut main_out = 0.0;
        let mut aux_out = 0.0;
        
        for (i, grain) in self.grains.iter_mut().enumerate() {
            if grain.active {
                let grain_sample = grain.process(sample_rate);
                
                // Distribute grains between main and aux outputs
                if i % 2 == 0 {
                    main_out += grain_sample;
                } else {
                    aux_out += grain_sample;
                }
            }
        }
        
        // Normalize by number of possible grains to avoid clipping
        let normalization = 1.0 / (self.grains.len() as f32 * 0.5);
        
        self.sample = 5.0 * main_out * normalization;
        self.aux_sample = 5.0 * aux_out * normalization;
    }
}
