use anyhow::{anyhow, Result};
use crate::{dsp::utils::clamp, types::InternalParam};

#[derive(Default, Params)]
struct PulseOscillatorParams {
    #[param("freq", "frequency in v/oct")]
    freq: InternalParam,
    #[param("width", "pulse width (0-5, 2.5 is square)")]
    width: InternalParam,
    #[param("pwm", "pulse width modulation input")]
    pwm: InternalParam,
}

#[derive(Default, Module)]
#[module("pulse", "Pulse/Square oscillator with PWM")]
pub struct PulseOscillator {
    #[output("output", "signal output", default)]
    sample: f32,
    phase: f32,
    smoothed_freq: f32,
    smoothed_width: f32,
    params: PulseOscillatorParams,
}

impl PulseOscillator {
    fn update(&mut self, sample_rate: f32) -> () {
        let target_freq = clamp(-10.0, 10.0, self.params.freq.get_value_or(4.0));
        let base_width = self.params.width.get_value_or(2.5);
        let pwm = self.params.pwm.get_value_or(0.0);
        let target_width = (base_width + pwm).clamp(0.0, 5.0);
        
        self.smoothed_freq = crate::types::smooth_value(self.smoothed_freq, target_freq);
        self.smoothed_width = crate::types::smooth_value(self.smoothed_width, target_width);
        
        let voltage = self.smoothed_freq;
        let frequency = 27.5f32 * 2.0f32.powf(voltage);
        let phase_increment = frequency / sample_rate;
        
        // Pulse width (0.0 to 1.0, 0.5 is square wave)
        let pulse_width = (self.smoothed_width / 5.0).clamp(0.01, 0.99);
        
        self.phase += phase_increment;
        
        // Wrap phase
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        
        // Naive pulse wave
        let mut naive_pulse = if self.phase < pulse_width { 1.0 } else { -1.0 };
        
        // Apply PolyBLEP at the rising edge (phase = 0)
        naive_pulse += poly_blep_pulse(self.phase, phase_increment);
        
        // Apply PolyBLEP at the falling edge (phase = pulse_width)
        naive_pulse -= poly_blep_pulse(
            if self.phase >= pulse_width {
                self.phase - pulse_width
            } else {
                self.phase - pulse_width + 1.0
            },
            phase_increment,
        );
        
        self.sample = naive_pulse * 5.0;
    }
}

// PolyBLEP for pulse wave
fn poly_blep_pulse(phase: f32, phase_increment: f32) -> f32 {
    // Detect discontinuity at phase wrap (0.0)
    if phase < phase_increment {
        let t = phase / phase_increment;
        return t + t - t * t - 1.0;
    }
    // Detect discontinuity approaching 1.0
    else if phase > 1.0 - phase_increment {
        let t = (phase - 1.0) / phase_increment;
        return t * t + t + t + 1.0;
    }
    0.0
}
