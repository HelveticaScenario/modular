//! MIDI input handling for the modular synthesizer.
//!
//! This module provides MIDI device enumeration, connection management,
//! and converts raw MIDI bytes to Message types for dispatch to DSP modules.
//! Supports multiple simultaneous MIDI device connections.
//! Messages are timestamped and sorted to ensure correct ordering.
//! Supports 14-bit CC messages (CC 0-31 MSB + CC 32-63 LSB).

use midir::{MidiInput, MidiInputConnection};
use modular_core::types::{
    Message, MidiChannelPressure, MidiControlChange, MidiControlChange14Bit, MidiNoteOff,
    MidiNoteOn, MidiPitchBend, MidiPolyPressure,
};
use parking_lot::Mutex;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

/// Maximum MIDI messages buffered
const MIDI_BUFFER_SIZE: usize = 1024;

/// A MIDI message with its timestamp (microseconds from midir)
#[derive(Debug, Clone)]
struct TimestampedMessage {
    /// Timestamp in microseconds (from midir callback)
    timestamp_us: u64,
    /// The parsed MIDI message
    message: Message,
}

/// State for tracking 14-bit CC MSB values per device
/// Key: (device_name, channel, cc_msb), Value: msb_value
#[derive(Debug, Default)]
struct MidiCcState {
    /// MSB values waiting for LSB: [channel][cc] -> msb_value
    /// Only CC 0-31 can have MSB (their LSB is CC 32-63)
    msb_values: [[Option<u8>; 32]; 16],
}

impl MidiCcState {
    fn new() -> Self {
        Self {
            msb_values: [[None; 32]; 16],
        }
    }

    /// Store MSB value for later combination with LSB
    fn set_msb(&mut self, channel: u8, cc: u8, value: u8) {
        if cc < 32 && channel < 16 {
            self.msb_values[channel as usize][cc as usize] = Some(value);
        }
    }

    /// Take the stored MSB value for a given channel/cc, if any
    fn take_msb(&mut self, channel: u8, cc_msb: u8) -> Option<u8> {
        if cc_msb < 32 && channel < 16 {
            self.msb_values[channel as usize][cc_msb as usize].take()
        } else {
            None
        }
    }
}

/// Information about a MIDI input port
#[derive(Debug, Clone)]
pub struct MidiPortInfo {
    pub name: String,
    pub index: usize,
}

/// Shared state for MIDI parsing across callbacks
struct MidiParseState {
    /// Pending messages with timestamps
    messages: Vec<TimestampedMessage>,
    /// 14-bit CC state per device
    cc_state: HashMap<String, MidiCcState>,
}

impl MidiParseState {
    fn new() -> Self {
        Self {
            messages: Vec::with_capacity(MIDI_BUFFER_SIZE),
            cc_state: HashMap::new(),
        }
    }

    fn get_cc_state(&mut self, device: &str) -> &mut MidiCcState {
        self.cc_state
            .entry(device.to_string())
            .or_insert_with(MidiCcState::new)
    }
}

/// Manages multiple MIDI input connections
pub struct MidiInputManager {
    /// Active connections keyed by device name
    connections: Mutex<HashMap<String, MidiInputConnection<()>>>,
    /// Shared parse state (messages + 14-bit CC tracking)
    parse_state: Arc<Mutex<MidiParseState>>,
    /// Device names we want to be connected to (for reconnection)
    desired_devices: Mutex<HashSet<String>>,
}

impl MidiInputManager {
    /// Create a new MIDI input manager
    pub fn new() -> Self {
        Self {
            connections: Mutex::new(HashMap::new()),
            parse_state: Arc::new(Mutex::new(MidiParseState::new())),
            desired_devices: Mutex::new(HashSet::new()),
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

    /// Connect to a MIDI input port by name.
    /// Returns Ok(true) if newly connected, Ok(false) if already connected.
    pub fn connect(&self, port_name: &str) -> Result<bool, String> {
        // Add to desired devices for reconnection
        self.desired_devices.lock().insert(port_name.to_string());

        // Check if already connected
        if self.connections.lock().contains_key(port_name) {
            return Ok(false);
        }

        self.connect_internal(port_name)?;
        Ok(true)
    }

    /// Internal connection logic
    fn connect_internal(&self, port_name: &str) -> Result<(), String> {
        let midi_in = MidiInput::new("modular")
            .map_err(|e| format!("Failed to create MIDI input: {}", e))?;

        // Find port by name
        let port = midi_in
            .ports()
            .into_iter()
            .find(|p| midi_in.port_name(p).ok().as_deref() == Some(port_name))
            .ok_or_else(|| format!("MIDI port '{}' not found", port_name))?;

        let parse_state = self.parse_state.clone();
        let device_name = port_name.to_string();

        let connection = midi_in
            .connect(
                &port,
                "modular-input",
                move |timestamp_us, data, _| {
                    let mut state = parse_state.lock();
                    if state.messages.len() < MIDI_BUFFER_SIZE {
                        parse_midi_message(data, &device_name, timestamp_us, &mut state);
                    }
                },
                (),
            )
            .map_err(|e| format!("Failed to connect to MIDI port '{}': {}", port_name, e))?;

        self.connections.lock().insert(port_name.to_string(), connection);
        println!("[MIDI] Connected to: {}", port_name);

        Ok(())
    }

    /// Disconnect from a specific MIDI input
    pub fn disconnect(&self, port_name: &str) {
        self.desired_devices.lock().remove(port_name);
        if self.connections.lock().remove(port_name).is_some() {
            println!("[MIDI] Disconnected from: {}", port_name);
        }
    }

    /// Disconnect from all MIDI inputs
    pub fn disconnect_all(&self) {
        self.desired_devices.lock().clear();
        let connections = std::mem::take(&mut *self.connections.lock());
        for (name, _conn) in connections {
            println!("[MIDI] Disconnected from: {}", name);
        }
    }

    /// Get list of currently connected port names
    pub fn connected_ports(&self) -> Vec<String> {
        self.connections.lock().keys().cloned().collect()
    }

    /// Get the name of a single connected port (for backward compatibility)
    /// Returns the first connected port, or None if no ports are connected
    pub fn connected_port(&self) -> Option<String> {
        self.connections.lock().keys().next().cloned()
    }

    /// Update desired devices and sync connections.
    /// Connects to new devices, disconnects from devices no longer needed.
    pub fn sync_devices(&self, device_names: &HashSet<String>) {
        let mut desired = self.desired_devices.lock();
        let mut connections = self.connections.lock();

        // Find devices to disconnect (in connections but not in device_names)
        let to_disconnect: Vec<String> = connections
            .keys()
            .filter(|name| !device_names.contains(*name))
            .cloned()
            .collect();

        for name in to_disconnect {
            if connections.remove(&name).is_some() {
                println!("[MIDI] Disconnected from: {}", name);
            }
        }

        // Update desired devices
        *desired = device_names.clone();

        // Release locks before connecting
        drop(connections);
        drop(desired);

        // Connect to new devices
        for name in device_names {
            if !self.connections.lock().contains_key(name) {
                if let Err(e) = self.connect_internal(name) {
                    eprintln!("[MIDI] Failed to connect to '{}': {}", name, e);
                }
            }
        }
    }

    /// Attempt to reconnect to any desired devices that aren't currently connected.
    /// Call this periodically to handle hot-plugged devices.
    pub fn try_reconnect(&self) {
        let desired: Vec<String> = self.desired_devices.lock().iter().cloned().collect();
        
        for name in desired {
            if !self.connections.lock().contains_key(&name) {
                // Try to reconnect silently (device may not be plugged in)
                if let Ok(()) = self.connect_internal(&name) {
                    println!("[MIDI] Reconnected to: {}", name);
                }
            }
        }
    }

    /// Take all pending messages, sorted by timestamp (clears the buffer).
    /// Messages from multiple devices are interleaved in the correct temporal order.
    pub fn take_messages(&self) -> Vec<Message> {
        let mut state = self.parse_state.lock();
        let mut timestamped = std::mem::take(&mut state.messages);
        
        // Sort by timestamp to ensure messages are processed in the correct order
        // even when coming from multiple MIDI devices
        timestamped.sort_by_key(|m| m.timestamp_us);
        
        // Extract just the messages, now in correct order
        timestamped.into_iter().map(|tm| tm.message).collect()
    }
}

impl Default for MidiInputManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse raw MIDI bytes and add messages to state.
/// Handles 14-bit CC by tracking MSB (CC 0-31) and combining with LSB (CC 32-63).
fn parse_midi_message(data: &[u8], device: &str, timestamp_us: u64, state: &mut MidiParseState) {
    if data.is_empty() {
        return;
    }

    let status_byte = data[0];

    // Skip system messages (0xF0-0xFF)
    if status_byte >= 0xF0 {
        return;
    }

    let channel = status_byte & 0x0F;
    let status = status_byte & 0xF0;
    let data1 = data.get(1).copied().unwrap_or(0);
    let data2 = data.get(2).copied().unwrap_or(0);
    let device_opt = Some(device.to_string());

    let message = match status {
        0x90 if data2 > 0 => {
            // Note On
            Some(Message::MidiNoteOn(MidiNoteOn {
                device: device_opt,
                channel,
                note: data1,
                velocity: data2,
            }))
        }
        0x80 | 0x90 => {
            // Note Off (or Note On with velocity 0)
            Some(Message::MidiNoteOff(MidiNoteOff {
                device: device_opt,
                channel,
                note: data1,
                velocity: data2,
            }))
        }
        0xB0 => {
            // Control Change - handle 14-bit CC
            let cc = data1;
            let value = data2;
            let cc_state = state.get_cc_state(device);

            if cc < 32 {
                // MSB for 14-bit CC (CC 0-31)
                // Store MSB and emit regular 7-bit CC message
                // The 14-bit message will be emitted when LSB arrives
                cc_state.set_msb(channel, cc, value);
                Some(Message::MidiCC(MidiControlChange {
                    device: device_opt,
                    channel,
                    cc,
                    value,
                }))
            } else if cc < 64 {
                // LSB for 14-bit CC (CC 32-63)
                // Check if we have a stored MSB
                let cc_msb = cc - 32;
                if let Some(msb) = cc_state.take_msb(channel, cc_msb) {
                    // Combine MSB and LSB into 14-bit value
                    let value_14bit = ((msb as u16) << 7) | (value as u16);
                    // Emit both the regular LSB CC message and the 14-bit message
                    state.messages.push(TimestampedMessage {
                        timestamp_us,
                        message: Message::MidiCC(MidiControlChange {
                            device: device_opt.clone(),
                            channel,
                            cc,
                            value,
                        }),
                    });
                    Some(Message::MidiCC14Bit(MidiControlChange14Bit {
                        device: device_opt,
                        channel,
                        cc: cc_msb,
                        value: value_14bit,
                    }))
                } else {
                    // No MSB stored, just emit regular CC
                    Some(Message::MidiCC(MidiControlChange {
                        device: device_opt,
                        channel,
                        cc,
                        value,
                    }))
                }
            } else {
                // Regular CC (64-127)
                Some(Message::MidiCC(MidiControlChange {
                    device: device_opt,
                    channel,
                    cc,
                    value,
                }))
            }
        }
        0xE0 => {
            // Pitch Bend
            let value = (((data2 as u16) << 7) | (data1 as u16)) as i16 - 8192;
            Some(Message::MidiPitchBend(MidiPitchBend {
                device: device_opt,
                channel,
                value,
            }))
        }
        0xD0 => {
            // Channel Pressure (Aftertouch)
            Some(Message::MidiChannelPressure(MidiChannelPressure {
                device: device_opt,
                channel,
                pressure: data1,
            }))
        }
        0xA0 => {
            // Polyphonic Key Pressure
            Some(Message::MidiPolyPressure(MidiPolyPressure {
                device: device_opt,
                channel,
                note: data1,
                pressure: data2,
            }))
        }
        _ => None,
    };

    if let Some(msg) = message {
        state.messages.push(TimestampedMessage {
            timestamp_us,
            message: msg,
        });
    }
}
