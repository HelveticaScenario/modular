use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    dsp::utils::{hz_to_voct, voct_to_hz_f64, SchmittTrigger},
    poly::MonoSignal,
    types::ClockMessages,
    PolyOutput,
};

fn default_four() -> u32 {
    4
}

/// Deserialize a u32 that must be >= 1 (positive integer).
/// Rejects 0 with a descriptive error so any clock instance gets validated.
fn deserialize_positive_u32<'de, D>(deserializer: D) -> std::result::Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v = u32::deserialize(deserializer)?;
    if v == 0 {
        return Err(serde::de::Error::custom(
            "must be a positive integer (>= 1)",
        ));
    }
    Ok(v)
}

#[derive(Deserialize, JsonSchema, Connect, ChannelCount)]
#[serde(default, rename_all = "camelCase")]
struct ClockParams {
    /// Tempo control in V/Oct. Defaults to 120 BPM when unpatched.
    tempo: MonoSignal,
    /// Run gate. High runs the clock, low stops it. Defaults to high when unpatched.
    run: MonoSignal,
    /// Reset trigger. A rising edge restarts the bar.
    reset: MonoSignal,
    /// Time signature numerator (beats per bar). Must be a positive integer. Defaults to 4.
    #[serde(
        default = "default_four",
        deserialize_with = "deserialize_positive_u32"
    )]
    numerator: u32,
    /// Time signature denominator (beat value). Must be a positive integer. Defaults to 4.
    #[serde(
        default = "default_four",
        deserialize_with = "deserialize_positive_u32"
    )]
    denominator: u32,
}

impl Default for ClockParams {
    fn default() -> Self {
        Self {
            tempo: MonoSignal::default(),
            run: MonoSignal::default(),
            reset: MonoSignal::default(),
            numerator: 4,
            denominator: 4,
        }
    }
}

/// Tempo-synced transport clock for driving sequencers, envelopes, and synced modulation.
#[module(name = "$clock", channels = 2, args(tempo?))]
pub struct Clock {
    outputs: ClockOutputs,
    phase: f64,
    freq: f32,
    ppq_phase: f64,
    beat_phase: f64,
    last_bar_trigger: bool,
    last_ppq_trigger: bool,
    last_beat_trigger: bool,
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
    #[output("beatTrigger", "5V trigger at the start of each beat", range = (0.0, 5.0))]
    beat_trigger: f32,
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
            beat_phase: 0.0,
            last_bar_trigger: false,
            last_ppq_trigger: false,
            last_beat_trigger: false,
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
            self.beat_phase = 0.0;
            self.loop_index = 0;
            self.last_bar_trigger = false;
            self.last_ppq_trigger = false;
            self.last_beat_trigger = false;
        }

        if !running {
            return; // If not running, skip the rest of the update to keep outputs where they are until clock starts
        }
        // Smooth frequency parameter to avoid clicks
        self.freq = self.params.tempo.get_value_or(*BPM_120_VOCT);

        // Convert V/Oct to Hz (use f64 for precision)
        let frequency_hz = voct_to_hz_f64(self.freq as f64);

        // Time signature: numerator = beats per bar, denominator = beat value
        // Clamp to valid values (minimum 1) to avoid division by zero
        let numerator = self.params.numerator.max(1) as f64;
        let denominator = self.params.denominator.max(1) as f64;

        // Calculate phase increment per sample
        // BPM tempo is in quarter notes per minute, so frequency_hz = quarter notes per second.
        // quarter_notes_per_bar tells us how many quarter notes fit in one bar given the time sig.
        // e.g. 4/4 = 4 quarter notes, 3/4 = 3, 6/8 = 3, 7/8 = 3.5
        let quarter_notes_per_bar = numerator * 4.0 / denominator;
        let bar_frequency = frequency_hz / quarter_notes_per_bar;
        let phase_increment = bar_frequency / sample_rate as f64;

        self.phase += phase_increment;
        self.ppq_phase += phase_increment;
        self.beat_phase += phase_increment;

        // Wrap phase at 1.0
        if self.phase >= 1.0 {
            self.phase -= 1.0;
            self.loop_index += 1;
        }

        // PPQ phase wraps at 12 PPQ per quarter note (= 12 * quarter_notes_per_bar per bar)
        let ppq_period = 1.0 / (12.0 * quarter_notes_per_bar);
        if self.ppq_phase >= ppq_period {
            self.ppq_phase -= ppq_period;
        }

        // Beat phase wraps once per beat (numerator beats per bar)
        let beat_period = 1.0 / numerator;
        if self.beat_phase >= beat_period {
            self.beat_phase -= beat_period;
        }

        self.outputs.playhead.set(0, self.phase as f32);
        self.outputs.playhead.set(1, self.loop_index as f32);

        // Generate ramp output (0 to 5V over one bar)
        self.outputs.ramp = self.phase as f32 * 5.0;

        // Generate bar trigger (trigger at start of bar)
        // Use <= so the trigger fires on the very first sample after start/reset
        // (phase == phase_increment after the first increment from 0).
        let should_bar_trigger = self.phase <= phase_increment;
        if should_bar_trigger && !self.last_bar_trigger {
            self.outputs.bar_trigger = 5.0;
        } else {
            self.outputs.bar_trigger = 0.0;
        }
        self.last_bar_trigger = should_bar_trigger;

        // Generate beat trigger (trigger at start of each beat)
        let should_beat_trigger = self.beat_phase <= phase_increment;
        if should_beat_trigger && !self.last_beat_trigger {
            self.outputs.beat_trigger = 5.0;
        } else {
            self.outputs.beat_trigger = 0.0;
        }
        self.last_beat_trigger = should_beat_trigger;

        // Generate PPQ trigger
        let should_ppq_trigger = self.ppq_phase <= phase_increment;
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
                self.beat_phase = 0.0;
                self.outputs.playhead.set(0, 0.0);
                self.outputs.playhead.set(1, 0.0);
                self.loop_index = 0;
                self.last_bar_trigger = false;
                self.last_ppq_trigger = false;
                self.last_beat_trigger = false;
            }
            ClockMessages::Stop => {
                self.running = false;
                println!("Clock stopped");
                // Ensure triggers are low while stopped.
                self.outputs.bar_trigger = 0.0;
                self.outputs.beat_trigger = 0.0;
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

    /// Helper: count how many times beat_trigger fires 5V over a given number of samples
    fn count_beat_triggers(c: &mut Clock, sr: f32, samples: usize) -> usize {
        let mut count = 0;
        for _ in 0..samples {
            c.update(sr);
            if c.outputs.beat_trigger == 5.0 {
                count += 1;
            }
        }
        count
    }

    /// Helper: count how many times bar_trigger fires 5V over a given number of samples
    fn count_bar_triggers(c: &mut Clock, sr: f32, samples: usize) -> usize {
        let mut count = 0;
        for _ in 0..samples {
            c.update(sr);
            if c.outputs.bar_trigger == 5.0 {
                count += 1;
            }
        }
        count
    }

    #[test]
    fn clock_default_time_sig_is_4_4() {
        let c = Clock::default();
        assert_eq!(c.params.numerator, 4);
        assert_eq!(c.params.denominator, 4);
    }

    #[test]
    fn clock_beat_trigger_fires_4_times_per_bar_in_4_4() {
        let mut c = Clock::default();
        let sr = 48_000.0;
        // 120 BPM in 4/4 = one bar every 2 seconds = 96000 samples.
        // Use samples_per_bar - 1 to avoid counting the next bar's opening trigger.
        let samples = 96_000 - 1;

        let _ = c.on_clock_message(&ClockMessages::Start);
        let beats = count_beat_triggers(&mut c, sr, samples);
        assert_eq!(beats, 4, "4/4 time should produce 4 beat triggers per bar");
    }

    #[test]
    fn clock_beat_trigger_fires_3_times_per_bar_in_3_4() {
        let mut c = Clock::default();
        c.params.numerator = 3;
        c.params.denominator = 4;
        let sr = 48_000.0;
        // 120 BPM in 3/4 = 3 quarter notes per bar = 1.5 seconds = 72000 samples
        let samples = 72_000 - 1;

        let _ = c.on_clock_message(&ClockMessages::Start);
        let beats = count_beat_triggers(&mut c, sr, samples);
        assert_eq!(beats, 3, "3/4 time should produce 3 beat triggers per bar");
    }

    #[test]
    fn clock_beat_trigger_fires_6_times_per_bar_in_6_8() {
        let mut c = Clock::default();
        c.params.numerator = 6;
        c.params.denominator = 8;
        let sr = 48_000.0;
        // 120 BPM in 6/8 = 6 eighth notes per bar = 3 quarter notes = 1.5 seconds = 72000 samples
        let samples = 72_000 - 1;

        let _ = c.on_clock_message(&ClockMessages::Start);
        let beats = count_beat_triggers(&mut c, sr, samples);
        assert_eq!(beats, 6, "6/8 time should produce 6 beat triggers per bar");
    }

    #[test]
    fn clock_beat_trigger_fires_7_times_per_bar_in_7_8() {
        let mut c = Clock::default();
        c.params.numerator = 7;
        c.params.denominator = 8;
        let sr = 48_000.0;
        // 120 BPM in 7/8 = 7 eighth notes = 3.5 quarter notes = 1.75 seconds = 84000 samples
        let samples = 84_000 - 1;

        let _ = c.on_clock_message(&ClockMessages::Start);
        let beats = count_beat_triggers(&mut c, sr, samples);
        assert_eq!(beats, 7, "7/8 time should produce 7 beat triggers per bar");
    }

    #[test]
    fn clock_bar_trigger_count_matches_time_sig() {
        let sr = 48_000.0;
        // Run 4 bars worth at 120 BPM in 3/4 time
        // 3/4: 3 quarter notes per bar = 1.5s per bar, 4 bars = 6s = 288000 samples
        let mut c = Clock::default();
        c.params.numerator = 3;
        c.params.denominator = 4;
        let _ = c.on_clock_message(&ClockMessages::Start);

        let bar_triggers = count_bar_triggers(&mut c, sr, 288_000 - 1);
        assert_eq!(
            bar_triggers, 4,
            "Should produce 4 bar triggers over 4 bars in 3/4"
        );
    }

    #[test]
    fn clock_time_sig_deserialization() {
        // Verify time sig params deserialize correctly from JSON
        let params: ClockParams =
            serde_json::from_str(r#"{"numerator": 6, "denominator": 8}"#).unwrap();
        assert_eq!(params.numerator, 6);
        assert_eq!(params.denominator, 8);
    }

    #[test]
    fn clock_time_sig_defaults_when_missing() {
        // Verify defaults when not provided in JSON
        let params: ClockParams = serde_json::from_str("{}").unwrap();
        assert_eq!(params.numerator, 4);
        assert_eq!(params.denominator, 4);
    }

    #[test]
    fn clock_beat_trigger_resets_on_start() {
        let mut c = Clock::default();
        let sr = 48_000.0;

        // Advance partway through a bar
        for _ in 0..24_000 {
            c.update(sr);
        }

        // Start should reset beat phase
        let _ = c.on_clock_message(&ClockMessages::Start);
        assert!(
            (c.beat_phase - 0.0).abs() < 1e-9,
            "beat_phase should be reset on Start"
        );
        assert!(
            !c.last_beat_trigger,
            "last_beat_trigger should be reset on Start"
        );
    }

    #[test]
    fn clock_stop_clears_beat_trigger() {
        let mut c = Clock::default();
        let sr = 48_000.0;

        c.update(sr);
        let _ = c.on_clock_message(&ClockMessages::Stop);
        assert_eq!(
            c.outputs.beat_trigger, 0.0,
            "beat_trigger should be 0 after Stop"
        );
    }

    #[test]
    fn clock_rejects_zero_numerator() {
        let result: std::result::Result<ClockParams, _> =
            serde_json::from_str(r#"{"numerator": 0}"#);
        assert!(result.is_err(), "numerator=0 should be rejected");
    }

    #[test]
    fn clock_rejects_zero_denominator() {
        let result: std::result::Result<ClockParams, _> =
            serde_json::from_str(r#"{"denominator": 0}"#);
        assert!(result.is_err(), "denominator=0 should be rejected");
    }
}
