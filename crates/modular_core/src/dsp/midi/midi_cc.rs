//! MIDI CC (Control Change) to CV converter module.
//!
//! Converts MIDI CC messages to control voltage signals.

use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::types::MidiControlChange;

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default)]
struct MidiCcParams {
    /// CC number to monitor (0-127)
    #[serde(default)]
    cc: u8,

    /// MIDI channel filter (1-16, None = omni/all channels)
    #[serde(default)]
    channel: Option<u8>,

    /// Minimum output voltage (default 0.0)
    #[serde(default)]
    min_voltage: f32,

    /// Maximum output voltage (default 10.0)
    #[serde(default = "default_max_voltage")]
    max_voltage: f32,

    /// Smoothing time in milliseconds (0 = instant)
    #[serde(default)]
    smoothing_ms: f32,
}

fn default_max_voltage() -> f32 {
    10.0
}

#[derive(Outputs, JsonSchema)]
struct MidiCcOutputs {
    #[output("output", "CC value as voltage", default)]
    output: f32,
}

#[derive(Default, Module)]
#[module("midiCc", "MIDI CC to CV converter")]
#[args()]
pub struct MidiCc {
    outputs: MidiCcOutputs,
    params: MidiCcParams,
    sample_rate: f32,
    /// Current CC value (0-127)
    current_value: u8,
    /// Smoothed output value
    smoothed_value: f32,
}

impl MidiCc {
    /// Check if we should process events from a MIDI channel
    fn should_process_channel(&self, midi_channel: u8) -> bool {
        match self.params.channel {
            None => true, // Omni mode
            Some(ch) => midi_channel == (ch.saturating_sub(1)), // 1-indexed param to 0-indexed MIDI
        }
    }

    /// Handle MIDI CC message
    fn on_midi_cc(&mut self, msg: &MidiControlChange) -> Result<()> {
        if msg.cc == self.params.cc && self.should_process_channel(msg.channel) {
            self.current_value = msg.value;
        }
        Ok(())
    }

    fn update(&mut self, sample_rate: f32) {
        self.sample_rate = sample_rate;

        // Calculate target voltage
        let normalized = self.current_value as f32 / 127.0;
        let target =
            self.params.min_voltage + normalized * (self.params.max_voltage - self.params.min_voltage);

        // Apply smoothing
        if self.params.smoothing_ms > 0.0 {
            let smoothing_samples = self.params.smoothing_ms * sample_rate / 1000.0;
            let alpha = 1.0 / smoothing_samples.max(1.0);
            self.smoothed_value += (target - self.smoothed_value) * alpha;
        } else {
            self.smoothed_value = target;
        }

        self.outputs.output = self.smoothed_value;
    }
}

message_handlers!(impl MidiCc {
    MidiCC(m) => MidiCc::on_midi_cc,
});
