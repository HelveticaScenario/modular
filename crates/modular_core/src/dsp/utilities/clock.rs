use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    PolyOutput,
    dsp::utils::{hz_to_voct, voct_to_hz_f64},
    types::{ClockMessages, Signal},
};

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default)]
struct ClockParams {
    /// tempo in v/oct (tempo)
    tempo: Signal,
}

#[derive(Module)]
#[module("clock", "A tempo clock with multiple outputs", channels = 2)]
#[args(tempo?)]
pub struct Clock {
    outputs: ClockOutputs,
    phase: f64,
    freq: f32,
    ppq_phase: f64,
    last_bar_trigger: bool,
    last_ppq_trigger: bool,
    running: bool,
    params: ClockParams,
    loop_index: u64,
}

#[derive(Outputs, JsonSchema)]
struct ClockOutputs {
    #[output(
        "playhead",
        "how many bars have elapsed. 2 channel output with phase and loop index",
        default
    )]
    playhead: PolyOutput,
    #[output("barTrigger", "trigger output every bar", range = (0.0, 5.0))]
    bar_trigger: f32,
    #[output("ramp", "ramp from 0 to 5V every bar", range = (0.0, 5.0))]
    ramp: f32,
    #[output("ppqTrigger", "trigger output at 48 PPQ", range = (0.0, 5.0))]
    ppq_trigger: f32,
}

impl Default for Clock {
    fn default() -> Self {
        Self {
            outputs: ClockOutputs::default(),
            phase: 0.0,
            freq: 0.0,
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

lazy_static! {
    static ref BPM_120_VOCT: f32 = hz_to_voct(120.0 / 60.0);
};

impl Clock {
    fn update(&mut self, sample_rate: f32) {
        // Smooth frequency parameter to avoid clicks
        self.freq = self.params.tempo.get_value_or(*BPM_120_VOCT);

        // Convert V/Oct to Hz (use f64 for precision)
        let frequency_hz = voct_to_hz_f64(self.freq as f64);
        // Calculate phase increment per sample
        // For a clock, we want the phase to go from 0 to 1 over one bar
        // At 120 BPM = 2 Hz, one bar (4 beats) = 2 seconds = 0.5 Hz
        // So bar frequency = tempo_hz / 4
        let bar_frequency = frequency_hz / 4.0;
        let phase_increment = bar_frequency / sample_rate as f64;

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
        self.outputs.playhead.set_channels(2);
        self.outputs.playhead.set(0, self.phase as f32);
        self.outputs.playhead.set(1, self.loop_index as f32);

        // Generate ramp output (0 to 5V over one bar)
        self.outputs.ramp = self.phase as f32 * 5.0;

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
                self.outputs.playhead.set(0, 0.0);
                self.outputs.playhead.set(1, 0.0);
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
                self.outputs.playhead.set(0, 0.0);
                self.outputs.playhead.set(1, 0.0);
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
