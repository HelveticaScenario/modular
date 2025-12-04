use anyhow::{anyhow, Result};
use crate::{dsp::utils::clamp, types::InternalParam};

#[derive(Default, Params)]
struct SawOscillatorParams {
    #[param("freq", "frequency in v/oct")]
    freq: InternalParam,
    #[param("shape", "waveform shape: 0=saw, 2.5=triangle, 5=ramp")]
    shape: InternalParam,
    #[param("phase", "the phase of the oscillator, overrides freq if present")]
    phase: InternalParam,
}

#[derive(Default, Module)]
#[module("saw-osc", "Sawtooth/Triangle/Ramp oscillator")]
pub struct SawOscillator {
    #[output("output", "signal output")]
    sample: f32,
    phase: f32,
    last_phase: f32,
    smoothed_freq: f32,
    smoothed_shape: f32,
    params: SawOscillatorParams,
}

impl SawOscillator {
    fn update(&mut self, sample_rate: f32) -> () {
        let target_shape = self.params.shape.get_value_or(0.0).clamp(0.0, 5.0);
        self.smoothed_shape = crate::types::smooth_value(self.smoothed_shape, target_shape);
        
        // If phase input is connected, use it directly (for syncing)
        let (current_phase, phase_increment) = if self.params.phase != InternalParam::Disconnected {
            let phase_input = self.params.phase.get_value();
            let wrapped_phase = crate::dsp::utils::wrap(0.0..1.0, phase_input);
            // Calculate phase increment from phase change for PolyBLEP
            let phase_inc = if wrapped_phase >= self.last_phase {
                wrapped_phase - self.last_phase
            } else {
                wrapped_phase + (1.0 - self.last_phase)
            };
            (wrapped_phase, phase_inc)
        } else {
            // Normal frequency-driven oscillation
            let target_freq = clamp(self.params.freq.get_value_or(4.0), 12.0, 0.0);
            self.smoothed_freq = crate::types::smooth_value(self.smoothed_freq, target_freq);
            
            let voltage = self.smoothed_freq;
            let frequency = 27.5f32 * 2.0f32.powf(voltage);
            let phase_increment = frequency / sample_rate;
            
            self.phase += phase_increment;
            
            // Wrap phase
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }
            
            (self.phase, phase_increment)
        };
        
        self.last_phase = current_phase;
        
        // Shape parameter: 0 = saw, 2.5 = triangle, 5 = ramp (reversed saw)
        let shape_norm = self.smoothed_shape / 5.0; // 0.0 to 1.0
        
        let output = if shape_norm < 0.5 {
            // Blend from saw (0.0) to triangle (0.5)
            let blend = shape_norm * 2.0;
            let saw = generate_saw(current_phase, phase_increment);
            let triangle = generate_triangle(current_phase, phase_increment);
            saw * (1.0 - blend) + triangle * blend
        } else {
            // Blend from triangle (0.5) to ramp (1.0)
            let blend = (shape_norm - 0.5) * 2.0;
            let triangle = generate_triangle(current_phase, phase_increment);
            let ramp = generate_ramp(current_phase, phase_increment);
            triangle * (1.0 - blend) + ramp * blend
        };
        
        self.sample = output * 5.0;
    }
}

// PolyBLEP (Polynomial Band-Limited Step) function
// Reduces aliasing at discontinuities
fn poly_blep(phase: f32, phase_increment: f32) -> f32 {
    // Detect discontinuity at phase wrap (0.0)
    if phase < phase_increment {
        let t = phase / phase_increment;
        return t + t - t * t - 1.0;
    }
    // Detect discontinuity at phase = 1.0
    else if phase > 1.0 - phase_increment {
        let t = (phase - 1.0) / phase_increment;
        return t * t + t + t + 1.0;
    }
    0.0
}

// Generate band-limited sawtooth wave
fn generate_saw(phase: f32, phase_increment: f32) -> f32 {
    let mut saw = 2.0 * phase - 1.0;
    saw -= poly_blep(phase, phase_increment);
    saw
}

// Generate band-limited ramp wave (reversed sawtooth)
fn generate_ramp(phase: f32, phase_increment: f32) -> f32 {
    let mut ramp = 1.0 - 2.0 * phase;
    ramp += poly_blep(phase, phase_increment);
    ramp
}

// Generate band-limited triangle wave
fn generate_triangle(phase: f32, phase_increment: f32) -> f32 {
    // Triangle is the integral of a square wave
    // We can generate it by integrating a PolyBLEP pulse
    let mut triangle = if phase < 0.5 {
        4.0 * phase - 1.0
    } else {
        3.0 - 4.0 * phase
    };
    
    // Apply PolyBLEP correction at the peak (phase = 0.5)
    triangle += poly_blep_integrated(phase, phase_increment);
    triangle -= poly_blep_integrated(
        if phase >= 0.5 { phase - 0.5 } else { phase + 0.5 },
        phase_increment,
    );
    
    triangle
}

// Integrated PolyBLEP for triangle wave
fn poly_blep_integrated(phase: f32, phase_increment: f32) -> f32 {
    if phase < phase_increment {
        let t = phase / phase_increment;
        return (t * t * t) / 3.0 - (t * t) / 2.0 + t / 2.0;
    } else if phase > 1.0 - phase_increment {
        let t = (phase - 1.0) / phase_increment;
        return -(t * t * t) / 3.0 - (t * t) / 2.0 - t / 2.0;
    }
    0.0
}
