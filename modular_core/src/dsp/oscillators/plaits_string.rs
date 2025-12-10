use anyhow::{Result, anyhow};

use crate::{
    dsp::utils::clamp,
    types::InternalParam,
};

/// Parameters for the Plaits String engine
#[derive(Default, Params)]
struct PlaitsStringParams {
    #[param("freq", "frequency in v/oct")]
    freq: InternalParam,
    #[param("harmonics", "harmonic content/brightness")]
    harmonics: InternalParam,
    #[param("timbre", "damping/decay time")]
    timbre: InternalParam,
    #[param("morph", "exciter position/color")]
    morph: InternalParam,
    #[param("trigger", "pluck/strike trigger")]
    trigger: InternalParam,
}

/// Plaits String Engine
/// 
/// Karplus-Strong string synthesis inspired by Mutable Instruments Plaits.
/// Features:
/// - Physical modeling of plucked strings
/// - Variable damping and brightness
/// - Exciter position control
#[derive(Module)]
#[module("plaits-string", "Plaits string synthesis engine (Karplus-Strong)")]
pub struct PlaitsString {
    #[output("output", "main string output", default)]
    sample: f32,
    
    // Delay line for Karplus-Strong
    delay_line: Vec<f32>,
    write_pos: usize,
    
    // Filter state for damping
    filter_state: f32,
    
    smoothed_freq: f32,
    
    // Random number generator for excitation
    rng_state: u32,
    
    // Trigger detection
    prev_trigger: f32,
    
    params: PlaitsStringParams,
}

impl Default for PlaitsString {
    fn default() -> Self {
        // Start with a reasonable delay line size
        let initial_size = 2048;
        Self {
            sample: 0.0,
            delay_line: vec![0.0; initial_size],
            write_pos: 0,
            filter_state: 0.0,
            smoothed_freq: 4.0,
            rng_state: 24680,
            prev_trigger: 0.0,
            params: PlaitsStringParams::default(),
        }
    }
}

impl PlaitsString {
    /// Generate random noise sample
    fn random(&mut self) -> f32 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 17;
        self.rng_state ^= self.rng_state << 5;
        (self.rng_state as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
    
    /// Excite the string with initial energy
    fn excite(&mut self, brightness: f32, position: f32) {
        let delay_length = self.delay_line.len();
        
        for i in 0..delay_length {
            // Generate excitation signal
            let mut excitation = self.random();
            
            // Apply brightness filter (simple one-pole lowpass)
            excitation = self.filter_state + brightness * (excitation - self.filter_state);
            self.filter_state = excitation;
            
            // Apply exciter position (creates different harmonic emphasis)
            let pos_factor = (i as f32 / delay_length as f32 - position).abs();
            let envelope = 1.0 - pos_factor;
            
            if envelope > 0.0 {
                self.delay_line[i] = excitation * envelope;
            } else {
                self.delay_line[i] = excitation * 0.1;
            }
        }
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
        let frequency = 27.5 * 2.0f32.powf(self.smoothed_freq);
        
        // Calculate required delay line length
        let required_length = (sample_rate / frequency).max(2.0) as usize;
        
        // Resize delay line if needed (with some hysteresis to avoid constant resizing)
        if required_length > self.delay_line.len() || required_length < self.delay_line.len() / 2 {
            let new_size = required_length.max(64).min(8192);
            self.delay_line.resize(new_size, 0.0);
            self.write_pos = 0;
        }
        
        // Detect trigger
        let triggered = trigger > 0.5 && self.prev_trigger <= 0.5;
        self.prev_trigger = trigger;
        
        if triggered {
            // Brightness from harmonics
            let brightness = 0.1 + harmonics * 0.89;
            
            // Exciter position from morph (0.0 = bridge, 1.0 = center)
            let position = morph;
            
            self.excite(brightness, position);
        }
        
        // Read from delay line
        let delay_length = self.delay_line.len();
        let read_pos = self.write_pos;
        let output = self.delay_line[read_pos];
        
        // Karplus-Strong: average current and previous sample, with damping
        let prev_pos = if read_pos == 0 {
            delay_length - 1
        } else {
            read_pos - 1
        };
        
        let damping = 0.5 + timbre * 0.499;  // 0.5 to 0.999 (higher = longer decay)
        let averaged = (output + self.delay_line[prev_pos]) * 0.5;
        
        // Apply damping filter (one-pole lowpass)
        let damping_coeff = 0.3 + timbre * 0.69;  // More timbre = less damping
        let filtered = self.filter_state + damping_coeff * (averaged - self.filter_state);
        self.filter_state = filtered;
        
        // Write back to delay line
        self.delay_line[self.write_pos] = filtered * damping;
        
        // Advance write position
        self.write_pos = (self.write_pos + 1) % delay_length;
        
        // Scale to ±5V range
        self.sample = 5.0 * clamp(-1.0, 1.0, output);
    }
}
