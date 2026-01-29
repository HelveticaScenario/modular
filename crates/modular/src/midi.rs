//! MIDI input handling for the modular synthesizer.
//!
//! This module provides MIDI device enumeration, connection management,
//! and converts raw MIDI bytes to Message types for dispatch to DSP modules.

use midir::{MidiInput, MidiInputConnection};
use modular_core::types::{
    Message, MidiChannelPressure, MidiControlChange, MidiNoteOff, MidiNoteOn, MidiPitchBend,
    MidiPolyPressure,
};
use parking_lot::Mutex;
use std::sync::Arc;

/// Maximum MIDI messages buffered
const MIDI_BUFFER_SIZE: usize = 1024;

/// Information about a MIDI input port
#[derive(Debug, Clone)]
pub struct MidiPortInfo {
    pub name: String,
    pub index: usize,
}

/// Manages MIDI input connections
pub struct MidiInputManager {
    /// Currently active connection (if any)
    connection: Mutex<Option<MidiInputConnection<()>>>,
    /// Pending messages to dispatch
    pending_messages: Arc<Mutex<Vec<Message>>>,
    /// Name of currently connected port
    connected_port: Mutex<Option<String>>,
}

impl MidiInputManager {
    /// Create a new MIDI input manager
    pub fn new() -> Self {
        Self {
            connection: Mutex::new(None),
            pending_messages: Arc::new(Mutex::new(Vec::with_capacity(MIDI_BUFFER_SIZE))),
            connected_port: Mutex::new(None),
        }
    }

    /// List available MIDI input ports
    pub fn list_ports() -> Vec<MidiPortInfo> {
        let midi_in = match MidiInput::new("modular-list") {
            Ok(m) => m,
            Err(_) => return Vec::new(),
        };

        midi_in
            .ports()
            .iter()
            .enumerate()
            .filter_map(|(index, port)| {
                midi_in
                    .port_name(port)
                    .ok()
                    .map(|name| MidiPortInfo { name, index })
            })
            .collect()
    }

    /// Connect to a MIDI input port by name
    pub fn connect(&self, port_name: &str) -> Result<(), String> {
        // Disconnect existing connection first
        self.disconnect();

        let midi_in = MidiInput::new("modular")
            .map_err(|e| format!("Failed to create MIDI input: {}", e))?;

        // Find port by name
        let port = midi_in
            .ports()
            .into_iter()
            .find(|p| midi_in.port_name(p).ok().as_deref() == Some(port_name))
            .ok_or_else(|| format!("MIDI port '{}' not found", port_name))?;

        let pending = self.pending_messages.clone();

        let connection = midi_in
            .connect(
                &port,
                "modular-input",
                move |_timestamp_us, data, _| {
                    if let Some(msg) = parse_midi_message(data) {
                        let mut msgs = pending.lock();
                        if msgs.len() < MIDI_BUFFER_SIZE {
                            msgs.push(msg);
                        }
                    }
                },
                (),
            )
            .map_err(|e| format!("Failed to connect to MIDI port: {}", e))?;

        *self.connection.lock() = Some(connection);
        *self.connected_port.lock() = Some(port_name.to_string());

        Ok(())
    }

    /// Disconnect from the current MIDI input
    pub fn disconnect(&self) {
        *self.connection.lock() = None;
        *self.connected_port.lock() = None;
    }

    /// Get the name of the currently connected port
    pub fn connected_port(&self) -> Option<String> {
        self.connected_port.lock().clone()
    }

    /// Take all pending messages (clears the buffer)
    pub fn take_messages(&self) -> Vec<Message> {
        std::mem::take(&mut *self.pending_messages.lock())
    }
}

impl Default for MidiInputManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse raw MIDI bytes into a Message
fn parse_midi_message(data: &[u8]) -> Option<Message> {
    if data.is_empty() {
        return None;
    }

    let status_byte = data[0];

    // Skip system messages (0xF0-0xFF)
    if status_byte >= 0xF0 {
        return None;
    }

    let channel = status_byte & 0x0F;
    let status = status_byte & 0xF0;
    let data1 = data.get(1).copied().unwrap_or(0);
    let data2 = data.get(2).copied().unwrap_or(0);

    match status {
        0x90 if data2 > 0 => {
            // Note On
            Some(Message::MidiNoteOn(MidiNoteOn {
                channel,
                note: data1,
                velocity: data2,
            }))
        }
        0x80 | 0x90 => {
            // Note Off (or Note On with velocity 0)
            Some(Message::MidiNoteOff(MidiNoteOff {
                channel,
                note: data1,
                velocity: data2,
            }))
        }
        0xB0 => {
            // Control Change
            Some(Message::MidiCC(MidiControlChange {
                channel,
                cc: data1,
                value: data2,
            }))
        }
        0xE0 => {
            // Pitch Bend
            let value = (((data2 as u16) << 7) | (data1 as u16)) as i16 - 8192;
            Some(Message::MidiPitchBend(MidiPitchBend { channel, value }))
        }
        0xD0 => {
            // Channel Pressure (Aftertouch)
            Some(Message::MidiChannelPressure(MidiChannelPressure {
                channel,
                pressure: data1,
            }))
        }
        0xA0 => {
            // Polyphonic Key Pressure
            Some(Message::MidiPolyPressure(MidiPolyPressure {
                channel,
                note: data1,
                pressure: data2,
            }))
        }
        _ => None,
    }
}
