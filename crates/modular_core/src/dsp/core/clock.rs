use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    PolyOutput,
    dsp::utils::{SchmittTrigger, hz_to_voct, voct_to_hz_f64},
    poly::MonoSignal,
    types::ClockMessages,
};

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default, rename_all = "camelCase")]
struct ClockParams {
    /// Tempo control in V/Oct. Defaults to 120 BPM when unpatched.
    tempo: MonoSignal,
    /// Run gate. High runs the clock, low stops it. Defaults to high when unpatched.
    run: MonoSignal,
    /// Reset trigger. A rising edge restarts the bar.
    reset: MonoSignal,
}

/// Tempo-synced transport clock for driving sequencers, envelopes, and synced modulation.
#[module(name = "$clock", description = "Tempo clock with bar and subdivision timing outputs", channels = 2, args(tempo?))]
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
    run_trigger: SchmittTrigger,
    reset_trigger: SchmittTrigger,
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ClockOutputs {
    #[output(
        "playhead",
        "Bar playhead: channel 0 is bar phase (0..1), channel 1 is completed bar count",
        default
    )]
    playhead: PolyOutput,
    #[output("barTrigger", "5V trigger at the start of each bar", range = (0.0, 5.0))]
    bar_trigger: f32,
    #[output("ramp", "0..5V ramp that resets every bar", range = (0.0, 5.0))]
    ramp: f32,
    #[output("ppqTrigger", "5V trigger at 48 pulses per quarter note", range = (0.0, 5.0))]
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
            run_trigger: SchmittTrigger::default(),
            reset_trigger: SchmittTrigger::default(),
            _channel_count: 0,
        }
    }
}

message_handlers!(impl Clock {
    Clock(m) => Clock::on_clock_message,
});

lazy_static! {
    static ref BPM_120_VOCT: f32 = hz_to_voct(120.0 / 60.0);
}

impl Clock {
    fn update(&mut self, sample_rate: f32) {
        // Process run param through Schmitt trigger when connected
        // We need process_with_edge to get the continuous high/low state (not just rising edge)
        let running = if !self.params.run.is_disconnected() {
            let run_value = self.params.run.get_value();
            let (is_high, _) = self.run_trigger.process_with_edge(run_value);
            is_high && self.running
        } else {
            self.run_trigger.reset();
            self.running
        };

        // Process reset param through Schmitt trigger (default 0V = no reset)
        let reset_value = self.params.reset.get_value_or(0.0);
        if self.reset_trigger.process(reset_value) {
            // Rising edge on reset: reset phase
            self.phase = 0.0;
            self.ppq_phase = 0.0;
            self.loop_index = 0;
            self.last_bar_trigger = false;
            self.last_ppq_trigger = false;
        }

        if !running {
            return; // If not running, skip the rest of the update to keep outputs where they are until clock starts
        }
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
        self.outputs.playhead.set(0, self.phase as f32);
        self.outputs.playhead.set(1, self.loop_index as f32);

        // Generate ramp output (0 to 5V over one bar)
        self.outputs.ramp = self.phase as f32 * 5.0;

        // Generate bar trigger (trigger at start of bar)
        let should_bar_trigger = self.phase < phase_increment;
        if should_bar_trigger && !self.last_bar_trigger {
            self.outputs.bar_trigger = 5.0;
        } else {
            self.outputs.bar_trigger = 0.0;
        }
        self.last_bar_trigger = should_bar_trigger;

        // Generate PPQ trigger (48 times per bar)
        let should_ppq_trigger = self.ppq_phase < phase_increment;
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

    #[test]
    fn clock_run_param_stops_clock() {
        let mut c = Clock::default();
        let sr = 48_000.0;

        // Default: run is disconnected, defaults to 5V (running)
        c.update(sr);
        assert!(c.phase > 0.0);

        // Set run param to 0V (stopped)
        c.params.run = serde_json::from_str("0.0").unwrap();
        assert!(
            !c.params.run.is_disconnected(),
            "Run param should be connected"
        );
        let phase_before = c.phase;
        for _ in 0..128 {
            c.update(sr);
        }
        assert!(
            (c.phase - phase_before).abs() < 1e-9,
            "Clock should be stopped when run is 0V"
        );

        // Set run param back to 5V (running)
        c.params.run = serde_json::from_str("5.0").unwrap();
        let phase_before = c.phase;
        c.update(sr);
        assert!(c.phase > phase_before, "Clock should resume when run is 5V");
    }

    #[test]
    fn clock_run_disconnect_resumes_running() {
        let mut c = Clock::default();
        let sr = 48_000.0;
        c.params.tempo = serde_json::from_str("0").unwrap(); // Set tempo to 1 Hz to make phase changes more obvious
        // Connect run at 0V (stopped)
        c.params.run = serde_json::from_str("0.0").unwrap();
        let initial_phase = c.phase;
        let did_not_change = (0..128)
            .map(|_| {
                c.update(sr);
                c.phase
            })
            .all(|v| v == initial_phase);
        assert!(did_not_change, "Clock should be stopped when run is 0V");

        // Disconnect run (back to default) — clock should resume
        c.params.run = MonoSignal::default();
        assert!(
            c.params.run.is_disconnected(),
            "Run param should be disconnected"
        );
        let did_not_change = (0..128)
            .map(|_| {
                c.update(sr);
                c.phase
            })
            .all(|v| v == initial_phase);
        assert!(
            !did_not_change,
            "Clock should resume when run is disconnected"
        );
    }

    #[test]
    fn clock_reset_param_resets_phase() {
        let mut c = Clock::default();
        let sr = 48_000.0;

        // Advance clock to build up phase
        for _ in 0..1000 {
            c.update(sr);
        }
        assert!(c.phase > 0.0, "Clock should have advanced");

        // Send reset trigger (rising edge from 0 to 5V)
        c.params.reset = serde_json::from_str("5.0").unwrap();
        c.update(sr);
        // Phase should be reset to 0 (plus one sample of advancement since reset happens before phase update)
        assert!(c.phase < 0.001, "Phase should be near zero after reset");
        assert_eq!(c.loop_index, 0, "Loop index should be reset");

        // Keep reset high - no further resets (no new rising edge)
        let phase_after_reset = c.phase;
        c.update(sr);
        assert!(
            c.phase > phase_after_reset,
            "Clock should continue advancing after reset"
        );
    }
}
