use anyhow::{Result, anyhow};

use crate::{
    dsp::utils::clamp,
    types::{InternalParam, smooth_value},
};

#[derive(Default, Params)]
struct ClockParams {
    #[param("freq", "frequency in v/oct (tempo)")]
    freq: InternalParam,
    #[param("reset", "trigger to reset clock phase")]
    reset: InternalParam,
    #[param("run", "run gate (>2.5V = running, defaults to 5V)")]
    run: InternalParam,
}

#[derive(Module)]
#[module("clock", "A tempo clock with multiple outputs")]
pub struct Clock {
    #[output("bar_trigger", "trigger output every bar", default)]
    bar_trigger: f32,
    #[output("ramp", "ramp from 0 to 5V every bar")]
    ramp: f32,
    #[output("ppq_trigger", "trigger output at 48 PPQ")]
    ppq_trigger: f32,
    
    phase: f32,
    smoothed_freq: f32,
    last_reset: f32,
    ppq_phase: f32,
    last_bar_trigger: bool,
    last_ppq_trigger: bool,
    params: ClockParams,
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            bar_trigger: 0.0,
            ramp: 0.0,
            ppq_trigger: 0.0,
            phase: 0.0,
            smoothed_freq: 4.0,
            last_reset: 0.0,
            ppq_phase: 0.0,
            last_bar_trigger: false,
            last_ppq_trigger: false,
            params: ClockParams::default(),
        }
    }
}

impl Clock {
    fn update(&mut self, sample_rate: f32) {
        // Check if running (defaults to high if disconnected)
        let run_gate = self.params.run.get_value_or(5.0);
        let is_running = run_gate > 2.5;
        
        // Check for reset trigger (rising edge)
        let reset_value = self.params.reset.get_value_or(0.0);
        let reset_triggered = reset_value > 2.5 && self.last_reset <= 2.5;
        self.last_reset = reset_value;
        
        if reset_triggered {
            self.phase = 0.0;
            self.ppq_phase = 0.0;
        }
        
        // Smooth frequency parameter to avoid clicks
        let target_freq = clamp(-10.0, 10.0, self.params.freq.get_value_or(4.0));
        self.smoothed_freq = smooth_value(self.smoothed_freq, target_freq);
        
        // Convert V/Oct to Hz
        let frequency_hz = 27.5 * 2.0_f32.powf(self.smoothed_freq);
        
        // Calculate phase increment per sample
        // For a clock, we want the phase to go from 0 to 1 over one bar
        // At 120 BPM = 2 Hz, one bar (4 beats) = 2 seconds = 0.5 Hz
        // So bar frequency = tempo_hz / 4
        let bar_frequency = frequency_hz / 4.0;
        let phase_increment = bar_frequency / sample_rate;
        
        // Update phase if running
        if is_running {
            self.phase += phase_increment;
            self.ppq_phase += phase_increment;
            
            // Wrap phase at 1.0
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }
            
            // PPQ phase wraps more frequently (48 times per bar)
            if self.ppq_phase >= 1.0 / 48.0 {
                self.ppq_phase -= 1.0 / 48.0;
            }
        }
        
        // Generate ramp output (0 to 5V over one bar)
        self.ramp = self.phase * 5.0;
        
        // Generate bar trigger (trigger at start of bar)
        let should_bar_trigger = self.phase < phase_increment && is_running;
        if should_bar_trigger && !self.last_bar_trigger {
            self.bar_trigger = 5.0;
        } else {
            self.bar_trigger = 0.0;
        }
        self.last_bar_trigger = should_bar_trigger;
        
        // Generate PPQ trigger (48 times per bar)
        let should_ppq_trigger = self.ppq_phase < phase_increment && is_running;
        if should_ppq_trigger && !self.last_ppq_trigger {
            self.ppq_trigger = 5.0;
        } else {
            self.ppq_trigger = 0.0;
        }
        self.last_ppq_trigger = should_ppq_trigger;
    }
}
