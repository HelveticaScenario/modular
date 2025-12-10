use anyhow::{Result, anyhow};

use crate::{
    dsp::utils::clamp,
    types::InternalParam,
};

/// Parameters for the Plaits Modal engine
#[derive(Default, Params)]
struct PlaitsModalParams {
    #[param("freq", "base frequency in v/oct")]
    freq: InternalParam,
    #[param("harmonics", "harmonic structure/inharmonicity")]
    harmonics: InternalParam,
    #[param("timbre", "damping/decay time")]
    timbre: InternalParam,
    #[param("morph", "exciter brightness")]
    morph: InternalParam,
    #[param("trigger", "strike/excitation trigger")]
    trigger: InternalParam,
}

/// Single modal resonator
#[derive(Clone)]
struct ModalResonator {
    frequency: f32,
    damping: f32,
    amplitude: f32,
    phase: f32,
}

impl ModalResonator {
    fn new() -> Self {
        Self {
            frequency: 440.0,
            damping: 0.999,
            amplitude: 0.0,
            phase: 0.0,
        }
    }
    
    fn excite(&mut self, amplitude: f32) {
        self.amplitude = amplitude;
    }
    
    fn process(&mut self, sample_rate: f32) -> f32 {
        if self.amplitude < 0.001 {
            return 0.0;
        }
        
        // Apply damping
        self.amplitude *= self.damping;
        
        // Update phase
        let phase_inc = self.frequency / sample_rate;
        self.phase += phase_inc;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        
        // Generate sine wave
        let output = (self.phase * 2.0 * std::f32::consts::PI).sin() * self.amplitude;
        output
    }
}

/// Plaits Modal Engine
/// 
/// Modal synthesis inspired by Mutable Instruments Plaits.
/// Simulates resonant objects like bells, bars, and membranes.
/// Features:
/// - Multiple modal resonators
/// - Harmonic and inharmonic modes
/// - Variable damping
/// - Noise exciter
#[derive(Module)]
#[module("plaits-modal", "Plaits modal synthesis engine")]
pub struct PlaitsModal {
    #[output("output", "main modal output", default)]
    sample: f32,
    
    resonators: Vec<ModalResonator>,
    
    smoothed_freq: f32,
    
    // Exciter noise generator
    rng_state: u32,
    
    // Trigger detection
    prev_trigger: f32,
    
    params: PlaitsModalParams,
}

impl Default for PlaitsModal {
    fn default() -> Self {
        Self {
            sample: 0.0,
            resonators: vec![ModalResonator::new(); 6],
            smoothed_freq: 4.0,
            rng_state: 98765,
            prev_trigger: 0.0,
            params: PlaitsModalParams::default(),
        }
    }
}

impl PlaitsModal {
    /// Generate random noise sample
    fn random(&mut self) -> f32 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 17;
        self.rng_state ^= self.rng_state << 5;
        (self.rng_state as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
    
    /// Calculate mode frequencies
    /// inharmonicity: 0.0 = harmonic, 1.0 = bell-like inharmonic
    fn calculate_mode_frequencies(base_freq: f32, inharmonicity: f32) -> Vec<f32> {
        let mut freqs = Vec::new();
        
        if inharmonicity < 0.5 {
            // More harmonic (bar/string-like)
            let blend = inharmonicity * 2.0;
            for i in 0..6 {
                let harmonic_ratio = (i + 1) as f32;
                let inharmonic_ratio = ((i + 1) * (i + 1)) as f32;
                let ratio = harmonic_ratio * (1.0 - blend) + inharmonic_ratio * blend;
                freqs.push(base_freq * ratio);
            }
        } else {
            // More inharmonic (bell/membrane-like)
            let modes = [1.0, 2.76, 5.40, 8.93, 13.34, 18.64];  // Bell-like ratios
            for &ratio in modes.iter() {
                freqs.push(base_freq * ratio);
            }
        }
        
        freqs
    }
    
    fn update(&mut self, sample_rate: f32) {
        // Get parameters
        let freq_v = clamp(-10.0, 10.0, self.params.freq.get_value_or(4.0));
        let harmonics = clamp(0.0, 1.0, self.params.harmonics.get_value_or(0.5));
        let timbre = clamp(0.0, 1.0, self.params.timbre.get_value_or(0.5));
        let morph = clamp(0.0, 1.0, self.params.morph.get_value_or(0.5));
        let trigger = self.params.trigger.get_value_or(0.0);
        
        // Smooth frequency
        self.smoothed_freq = crate::types::smooth_value(self.smoothed_freq, freq_v);
        
        // Convert to Hz
        let base_hz = 27.5 * 2.0f32.powf(self.smoothed_freq);
        
        // Detect trigger (rising edge)
        let triggered = trigger > 0.5 && self.prev_trigger <= 0.5;
        self.prev_trigger = trigger;
        
        if triggered {
            // Calculate mode frequencies based on inharmonicity
            let mode_freqs = Self::calculate_mode_frequencies(base_hz, harmonics);
            
            // Calculate damping from timbre (0.0 = fast decay, 1.0 = long decay)
            let base_damping = 0.99 + timbre * 0.009;  // 0.99 to 0.999
            
            // Generate noise values before iterating (use fixed-size array to avoid heap allocation)
            let mut noise_values = [0.0f32; 6];
            for i in 0..6 {
                noise_values[i] = self.random();
            }
            
            // Excite resonators
            for (i, resonator) in self.resonators.iter_mut().enumerate() {
                if i < mode_freqs.len() {
                    resonator.frequency = mode_freqs[i];
                    
                    // Higher modes decay faster
                    let mode_damping = base_damping - (i as f32 * 0.001);
                    resonator.damping = clamp(0.95, 0.9999, mode_damping);
                    
                    // Amplitude decreases with mode number
                    let amplitude = 1.0 / ((i + 1) as f32).sqrt();
                    
                    // Add some noise excitation based on morph
                    let noise_excitation = if morph > 0.1 {
                        noise_values[i] * morph * 0.3
                    } else {
                        0.0
                    };
                    
                    resonator.excite(amplitude + noise_excitation);
                }
            }
        }
        
        // Process all resonators
        let mut output = 0.0;
        for resonator in self.resonators.iter_mut() {
            output += resonator.process(sample_rate);
        }
        
        // Normalize
        output /= self.resonators.len() as f32 * 0.5;
        
        // Add a bit of noise for texture (controlled by morph)
        if morph > 0.05 {
            let noise = self.random() * morph * 0.1;
            output += noise;
        }
        
        // Scale to ±5V range
        self.sample = 5.0 * clamp(-1.0, 1.0, output);
    }
}
