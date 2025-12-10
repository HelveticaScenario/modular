use anyhow::{Result, anyhow};

use crate::{
    dsp::utils::clamp,
    types::InternalParam,
};

/// Parameters for the Plaits Virtual Analog engine
#[derive(Default, Params)]
struct PlaitsVAParams {
    #[param("freq", "frequency in v/oct")]
    freq: InternalParam,
    #[param("harmonics", "detune amount for second oscillator")]
    harmonics: InternalParam,
    #[param("timbre", "waveshape (saw to square) and pulse width")]
    timbre: InternalParam,
    #[param("morph", "crossfade and sync amount")]
    morph: InternalParam,
}

/// Plaits Virtual Analog Engine
/// 
/// Variable-shape oscillator inspired by Mutable Instruments Plaits.
/// Features:
/// - Morphing from saw to pulse wave via timbre
/// - Pulse width modulation
/// - Dual oscillators with detuning
/// - Hard sync capability
#[derive(Default, Module)]
#[module("plaits-va", "Plaits virtual analog synthesis engine")]
pub struct PlaitsVA {
    #[output("output", "main output", default)]
    sample: f32,
    #[output("aux", "auxiliary output (detuned oscillator)")]
    aux_sample: f32,
    
    osc1_phase: f32,
    osc2_phase: f32,
    
    smoothed_freq: f32,
    smoothed_timbre: f32,
    smoothed_morph: f32,
    
    params: PlaitsVAParams,
}

impl PlaitsVA {
    /// Compute detuning in semitones from harmonics parameter (0.0-1.0)
    /// Maps to musical intervals
    fn compute_detune(harmonics: f32) -> f32 {
        let detune = 2.0 * harmonics - 1.0;  // Map to -1.0 to 1.0
        let sign = if detune < 0.0 { -1.0 } else { 1.0 };
        let abs_detune = detune.abs();
        
        // Quantize to musical intervals (in semitones)
        let intervals = [0.0, 7.0, 12.0, 19.0, 24.0];  // Unison, Perfect Fifth, Octave, Octave+Perfect Fifth, Two Octaves
        let scaled = abs_detune * (intervals.len() - 1) as f32;
        let index = scaled.floor() as usize;
        let frac = scaled - scaled.floor();
        
        let interval = if index >= intervals.len() - 1 {
            intervals[intervals.len() - 1]
        } else {
            intervals[index] + (intervals[index + 1] - intervals[index]) * frac
        };
        
        interval * sign
    }
    
    /// Generate variable waveshape (saw to square)
    /// shape: 0.0 = saw, 0.5 = triangle, 1.0 = square
    /// pw: pulse width (0.0 to 1.0)
    fn variable_shape(phase: f32, shape: f32, pw: f32) -> f32 {
        let pw_clamped = clamp(0.01, 0.99, pw);
        
        if shape < 0.33 {
            // Saw wave
            let saw = 2.0 * phase - 1.0;
            saw
        } else if shape < 0.66 {
            // Triangle wave (modified saw)
            let tri_phase = if phase < 0.5 {
                4.0 * phase - 1.0
            } else {
                3.0 - 4.0 * phase
            };
            tri_phase
        } else {
            // Pulse/square wave
            if phase < pw_clamped {
                1.0
            } else {
                -1.0
            }
        }
    }
    
    /// Polyblep antialiasing for discontinuities
    fn polyblep(phase: f32, phase_inc: f32) -> f32 {
        if phase < phase_inc {
            let t = phase / phase_inc;
            t + t - t * t - 1.0
        } else if phase > 1.0 - phase_inc {
            let t = (phase - 1.0) / phase_inc;
            t * t + t + t + 1.0
        } else {
            0.0
        }
    }
    
    /// Band-limited saw wave using polyblep
    fn polyblep_saw(phase: f32, phase_inc: f32) -> f32 {
        let naive = 2.0 * phase - 1.0;
        naive - Self::polyblep(phase, phase_inc)
    }
    
    fn update(&mut self, sample_rate: f32) {
        // Get parameters with defaults
        let freq_v = clamp(-10.0, 10.0, self.params.freq.get_value_or(4.0));
        let harmonics = clamp(0.0, 1.0, self.params.harmonics.get_value_or(0.0));
        let timbre = clamp(0.0, 1.0, self.params.timbre.get_value_or(0.5));
        let morph = clamp(0.0, 1.0, self.params.morph.get_value_or(0.0));
        
        // Smooth parameters
        self.smoothed_freq = crate::types::smooth_value(self.smoothed_freq, freq_v);
        self.smoothed_timbre = crate::types::smooth_value(self.smoothed_timbre, timbre);
        self.smoothed_morph = crate::types::smooth_value(self.smoothed_morph, morph);
        
        // Convert v/oct to Hz
        let base_hz = 27.5 * 2.0f32.powf(self.smoothed_freq);
        let detune_semitones = Self::compute_detune(harmonics);
        let osc2_hz = base_hz * 2.0f32.powf(detune_semitones / 12.0);
        
        // Phase increments
        let osc1_inc = base_hz / sample_rate;
        let osc2_inc = osc2_hz / sample_rate;
        
        // Extract shape and pulse width from timbre
        let shape = self.smoothed_timbre * 1.5;
        let shape_clamped = clamp(0.0, 1.0, shape);
        
        let pw = 0.5 + (self.smoothed_timbre - 0.66) * 1.4;
        let pw_clamped = clamp(0.1, 0.9, pw);
        
        // Oscillator 1
        self.osc1_phase += osc1_inc;
        if self.osc1_phase >= 1.0 {
            self.osc1_phase -= 1.0;
        }
        
        let osc1_out = if shape_clamped < 0.33 {
            // Use band-limited saw
            Self::polyblep_saw(self.osc1_phase, osc1_inc)
        } else {
            // Use variable shape
            Self::variable_shape(self.osc1_phase, shape_clamped, pw_clamped)
        };
        
        // Oscillator 2 with optional hard sync
        let sync_amount = self.smoothed_morph * self.smoothed_morph;  // Square for smoother control
        
        self.osc2_phase += osc2_inc;
        
        // Hard sync: reset osc2 when osc1 wraps
        if self.osc1_phase < osc1_inc && sync_amount > 0.01 {
            self.osc2_phase = self.osc1_phase * (osc2_inc / osc1_inc) * sync_amount;
        }
        
        if self.osc2_phase >= 1.0 {
            self.osc2_phase -= 1.0;
        }
        
        let osc2_out = if shape_clamped < 0.33 {
            Self::polyblep_saw(self.osc2_phase, osc2_inc)
        } else {
            Self::variable_shape(self.osc2_phase, shape_clamped, pw_clamped)
        };
        
        // Mix oscillators
        let mix = self.smoothed_morph;
        
        // Main output: blend between osc1 and osc1+osc2
        self.sample = 5.0 * (osc1_out * (1.0 - mix * 0.5) + osc2_out * mix * 0.5);
        
        // Aux output: just osc2
        self.aux_sample = 5.0 * osc2_out;
    }
}
