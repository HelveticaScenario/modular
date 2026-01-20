use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::types::{Clickless, ClockMessages, Signal};

#[derive(Deserialize, Default, JsonSchema, Connect)]
#[serde(default)]
struct ClockParams {
    /// tempo in v/oct (tempo)
    tempo: Signal,
}

#[derive(Module)]
#[module("clock", "A tempo clock with multiple outputs")]
#[args(tempo?)]
pub struct Clock {
    outputs: ClockOutputs,
    phase: f32,
    freq: Clickless,
    ppq_phase: f32,
    last_bar_trigger: bool,
    last_ppq_trigger: bool,
    running: bool,
    params: ClockParams,
    loop_index: usize,
}

#[derive(Outputs, JsonSchema)]
struct ClockOutputs {
    #[output("playhead", "how many bars have elapsed", default)]
    playhead: f32,
    #[output("barTrigger", "trigger output every bar")]
    bar_trigger: f32,
    #[output("ramp", "ramp from 0 to 5V every bar")]
    ramp: f32,
    #[output("ppqTrigger", "trigger output at 48 PPQ")]
    ppq_trigger: f32,
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            outputs: ClockOutputs::default(),
            phase: 0.0,
            freq: Clickless::default(),
            ppq_phase: 0.0,
            last_bar_trigger: false,
            last_ppq_trigger: false,
            running: true,
            params: ClockParams::default(),
            loop_index: 0,
        }
    }
}

message_handlers!(impl Clock {
    Clock(m) => Clock::on_clock_message,
});

impl Clock {
    fn update(&mut self, sample_rate: f32) {
        // Smooth frequency parameter to avoid clicks
        self.freq
            .update(self.params.tempo.get_value_or(0.0).clamp(-10.0, 10.0));

        // Convert V/Oct to Hz
        let frequency_hz = 55.0 * 2.0_f32.powf(*self.freq);

        // Calculate phase increment per sample
        // For a clock, we want the phase to go from 0 to 1 over one bar
        // At 120 BPM = 2 Hz, one bar (4 beats) = 2 seconds = 0.5 Hz
        // So bar frequency = tempo_hz / 4
        let bar_frequency = frequency_hz / 4.0;
        let phase_increment = bar_frequency / sample_rate;

        // Update phase if running
        if self.running {
            self.phase += phase_increment;
            self.ppq_phase += phase_increment;

            // Wrap phase at 1.0
            if self.phase >= 1.0 {
                self.phase -= 1.0;
                self.loop_index += 1;
            }

            // PPQ phase wraps more frequently (48 times per bar)
            if self.ppq_phase >= 1.0 / 48.0 {
                self.ppq_phase -= 1.0 / 48.0;
            }
        }

        self.outputs.playhead = self.loop_index as f32 + self.phase;

        // Generate ramp output (0 to 5V over one bar)
        self.outputs.ramp = self.phase * 5.0;

        // Generate bar trigger (trigger at start of bar)
        let should_bar_trigger = self.phase < phase_increment && self.running;
        if should_bar_trigger && !self.last_bar_trigger {
            self.outputs.bar_trigger = 5.0;
        } else {
            self.outputs.bar_trigger = 0.0;
        }
        self.last_bar_trigger = should_bar_trigger;

        // Generate PPQ trigger (48 times per bar)
        let should_ppq_trigger = self.ppq_phase < phase_increment && self.running;
        if should_ppq_trigger && !self.last_ppq_trigger {
            self.outputs.ppq_trigger = 5.0;
        } else {
            self.outputs.ppq_trigger = 0.0;
        }
        self.last_ppq_trigger = should_ppq_trigger;
    }

    fn on_clock_message(&mut self, m: &ClockMessages) -> Result<()> {
        match m {
            ClockMessages::Start => {
                self.running = true;
                // Start implies a transport reset.
                self.phase = 0.0;
                self.ppq_phase = 0.0;
                self.outputs.playhead = 0.0;
                self.loop_index = 0;
                self.last_bar_trigger = false;
                self.last_ppq_trigger = false;
            }
            ClockMessages::Stop => {
                self.running = false;
                println!("Clock stopped");
                // Ensure triggers are low while stopped.
                self.outputs.bar_trigger = 0.0;
                self.outputs.ppq_trigger = 0.0;
                self.outputs.playhead = 0.0;
                self.loop_index = 0;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_start_stop_via_message() {
        let mut c = Clock::default();
        let sr = 48_000.0;

        // Stop should freeze phase.
        let _ = c.on_clock_message(&ClockMessages::Stop);
        let phase_before = c.phase;
        for _ in 0..128 {
            c.update(sr);
        }
        assert!((c.phase - phase_before).abs() < 1e-9);

        // Start should reset and run.
        let _ = c.on_clock_message(&ClockMessages::Start);
        assert!((c.phase - 0.0).abs() < 1e-9);

        c.update(sr);
        assert!(c.phase > 0.0);
    }
}
