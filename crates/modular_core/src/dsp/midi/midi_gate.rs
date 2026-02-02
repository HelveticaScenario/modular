//! MIDI Gate module - outputs gate based on note range.
//!
//! Outputs a gate signal when any note in the specified range is held.

use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::types::{MidiNoteOff, MidiNoteOn};

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default)]
struct MidiGateParams {
    /// MIDI device name to receive from (None = all devices)
    #[serde(default)]
    device: Option<String>,

    /// Minimum note number (0-127, default 0)
    #[serde(default)]
    min_note: u8,

    /// Maximum note number (0-127, default 127)
    #[serde(default = "default_max_note")]
    max_note: u8,

    /// MIDI channel filter (1-16, None = omni/all channels)
    #[serde(default)]
    channel: Option<u8>,
}

fn default_max_note() -> u8 {
    127
}

fn default_high_voltage() -> f32 {
    10.0
}

#[derive(Outputs, JsonSchema)]
struct MidiGateOutputs {
    #[output("gate", "gate output", default)]
    gate: f32,
    #[output("noteCount", "number of notes currently held in range")]
    note_count: f32,
}

#[derive(Default, Module)]
#[module("midiGate", "MIDI note range to gate")]
#[args()]
pub struct MidiGate {
    outputs: MidiGateOutputs,
    params: MidiGateParams,
    /// Count of notes currently held in the range
    notes_held: u8,
}

impl MidiGate {
    /// Check if we should process events from a MIDI device
    fn should_process_device(&self, device: Option<&String>) -> bool {
        match (&self.params.device, device) {
            (None, _) => true,                          // No filter = accept all devices
            (Some(wanted), Some(got)) => wanted == got, // Exact match
            (Some(_), None) => false,                   // Filter set but no device info
        }
    }

    /// Check if we should process events from a MIDI channel
    fn should_process_channel(&self, midi_channel: u8) -> bool {
        match self.params.channel {
            None => true,                                       // Omni mode
            Some(ch) => midi_channel == (ch.saturating_sub(1)), // 1-indexed param to 0-indexed MIDI
        }
    }

    /// Check if a note is in the specified range
    fn is_in_range(&self, note: u8) -> bool {
        note >= self.params.min_note && note <= self.params.max_note
    }

    /// Handle MIDI note on message
    fn on_midi_note_on(&mut self, msg: &MidiNoteOn) -> Result<()> {
        if self.should_process_device(msg.device.as_ref())
            && self.should_process_channel(msg.channel)
            && self.is_in_range(msg.note)
        {
            self.notes_held = self.notes_held.saturating_add(1);
        }
        Ok(())
    }

    /// Handle MIDI note off message
    fn on_midi_note_off(&mut self, msg: &MidiNoteOff) -> Result<()> {
        if self.should_process_device(msg.device.as_ref())
            && self.should_process_channel(msg.channel)
            && self.is_in_range(msg.note)
        {
            self.notes_held = self.notes_held.saturating_sub(1);
        }
        Ok(())
    }

    /// Handle MIDI panic
    fn on_midi_panic(&mut self) -> Result<()> {
        self.notes_held = 0;
        Ok(())
    }

    fn update(&mut self, _sample_rate: f32) {
        self.outputs.gate = if self.notes_held > 0 { 5.0 } else { 0.0 };
        self.outputs.note_count = self.notes_held as f32;
    }
}

message_handlers!(impl MidiGate {
    MidiNoteOn(m) => MidiGate::on_midi_note_on,
    MidiNoteOff(m) => MidiGate::on_midi_note_off,
    MidiPanic => MidiGate::on_midi_panic,
});
