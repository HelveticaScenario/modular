//! Polyphonic signal support for multichannel cables.
//!
//! This module provides VCV Rack-style polyphonic signal handling,
//! allowing a single cable to carry up to 16 independent audio channels.
//!
//! - `PolyOutput`: A fixed-capacity output buffer with channel count metadata (for module outputs)
//! - `PolySignal`: A fixed-capacity input buffer containing Signal values (for polyphonic module inputs)

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use std::borrow::Cow;

/// Maximum channels per cable (matches VCV Rack / MIDI convention)
pub const PORT_MAX_CHANNELS: usize = 16;

/// A polyphonic output buffer with channel count metadata.
///
/// This is a fixed-capacity buffer that can hold up to 16 channels.
/// The `channels` field indicates how many channels are semantically valid:
/// - 0 = disconnected
/// - 1 = monophonic
/// - 2-16 = polyphonic
#[derive(Clone, Copy, Debug)]
pub struct PolyOutput {
    /// Voltage values for each channel (always allocated, not all may be active)
    voltages: [f32; PORT_MAX_CHANNELS],
    /// Number of active channels: 0 = disconnected, 1 = mono, 2-16 = poly
    channels: usize,
}

impl Default for PolyOutput {
    fn default() -> Self {
        Self {
            voltages: [0.0; PORT_MAX_CHANNELS],
            channels: 0, // Disconnected
        }
    }
}

impl PartialEq for PolyOutput {
    fn eq(&self, other: &Self) -> bool {
        if self.channels != other.channels {
            return false;
        }
        // Only compare active channels
        for i in 0..self.channels as usize {
            if self.voltages[i] != other.voltages[i] {
                return false;
            }
        }
        true
    }
}

impl PolyOutput {
    /// Create a monophonic signal with a single value
    pub fn mono(value: f32) -> Self {
        let mut sig = Self::default();
        sig.voltages[0] = value;
        sig.channels = 1;
        sig
    }

    // === Accessors ===

    /// Get voltage for a specific channel (returns 0.0 if out of range)
    pub fn get(&self, channel: usize) -> f32 {
        if channel < self.channels as usize {
            self.voltages[channel]
        } else {
            0.0
        }
    }

    /// Set voltage for a specific channel
    pub fn set(&mut self, channel: usize, value: f32) {
        if channel < PORT_MAX_CHANNELS {
            self.voltages[channel] = value;
        }
    }

    /// Get voltage with modulo cycling: channel wraps around available channels.
    /// This is consistent with Vec::cycle_get for non-signal params.
    /// A mono signal cycles to all channels, a 2-ch signal alternates, etc.
    pub fn get_cycling(&self, channel: usize) -> f32 {
        if self.channels == 0 {
            0.0 // Disconnected
        } else {
            self.voltages[channel % self.channels as usize]
        }
    }

    /// Set the number of active channels (clears higher channels to 0)
    pub fn set_channels(&mut self, channels: usize) {
        let channels = channels.clamp(0, PORT_MAX_CHANNELS);
        // Clear channels beyond the new count
        for c in channels..self.channels {
            self.voltages[c] = 0.0;
        }
        self.channels = channels;
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    pub fn set_all(&mut self, voltages: &[f32; PORT_MAX_CHANNELS]) {
        self.voltages = *voltages;
    }
}

// === Serialization ===

impl Serialize for PolyOutput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize as a struct with channels and voltages array
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("PolyOutput", 2)?;
        state.serialize_field("channels", &self.channels)?;
        state.serialize_field("voltages", &self.voltages[..self.channels as usize])?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for PolyOutput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct PolyOutputDe {
            channels: usize,
            voltages: Vec<f32>,
        }

        let de = PolyOutputDe::deserialize(deserializer)?;
        let mut sig = PolyOutput::default();
        sig.channels = de.channels.min(PORT_MAX_CHANNELS);
        for (i, &v) in de.voltages.iter().enumerate().take(sig.channels as usize) {
            sig.voltages[i] = v;
        }
        Ok(sig)
    }
}

// === JsonSchema ===

impl JsonSchema for PolyOutput {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("PolyOutput")
    }

    fn json_schema(r#gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
        // Schema matches the serialized form
        #[derive(JsonSchema)]
        #[allow(dead_code)]
        struct PolyOutputSchema {
            channels: usize,
            voltages: Vec<f32>,
        }
        PolyOutputSchema::json_schema(r#gen)
    }
}

// =============================================================================
// PolySignal - Polyphonic input containing multiple Signal values
// =============================================================================

use crate::types::Signal;

/// A polyphonic input buffer containing multiple Signal values.
///
/// This is used for module inputs that need to accept polyphonic connections.
/// Each slot in the array can be a separate mono Signal (Volts, Cable, or Disconnected).
/// The `channels` field indicates how many signals are semantically valid:
/// - 0 = disconnected (no signals)
/// - 1 = monophonic (single signal)
/// - 2-16 = polyphonic (multiple signals)
#[derive(Clone, Debug, Default)]
pub struct PolySignal {
    /// Signal values for each channel
    signals: [Signal; PORT_MAX_CHANNELS],
    /// Number of active channels: 0 = disconnected, 1 = mono, 2-16 = poly
    channels: usize,
}

impl PolySignal {
    /// Create a monophonic input from a single signal
    pub fn mono(signal: Signal) -> Self {
        let mut ps = Self::default();
        ps.signals[0] = signal;
        ps.channels = 1;
        ps
    }

    /// Create a polyphonic input from a slice of signals
    pub fn poly(signals: &[Signal]) -> Self {
        let channels = signals.len().min(PORT_MAX_CHANNELS);
        let mut ps = Self::default();
        for (i, s) in signals.iter().enumerate().take(channels) {
            ps.signals[i] = s.clone();
        }
        ps.channels = channels;
        ps
    }

    // === Accessors ===

    /// Get the number of active channels
    pub fn channels(&self) -> usize {
        self.channels
    }

    /// Check if disconnected (no active channels)
    pub fn is_disconnected(&self) -> bool {
        self.channels == 0
    }

    /// Check if monophonic (exactly 1 channel)
    pub fn is_monophonic(&self) -> bool {
        self.channels == 1
    }

    /// Check if polyphonic (more than 1 channel)
    pub fn is_polyphonic(&self) -> bool {
        self.channels > 1
    }

    /// Get signal at a specific channel (returns Disconnected if out of range)
    pub fn get(&self, channel: usize) -> &Signal {
        if channel < self.channels as usize {
            &self.signals[channel]
        } else {
            // Return a static disconnected signal for out-of-range
            static DISCONNECTED: Signal = Signal::Disconnected;
            &DISCONNECTED
        }
    }

    /// Get signal with cycling (wraps around available channels)
    pub fn get_cycling(&self, channel: usize) -> &Signal {
        if self.channels == 0 {
            static DISCONNECTED: Signal = Signal::Disconnected;
            &DISCONNECTED
        } else {
            &self.signals[channel % self.channels as usize]
        }
    }

    /// Get the f32 value at a channel with cycling
    pub fn get_value(&self, channel: usize) -> f32 {
        self.get_cycling(channel).get_value()
    }

    /// Get value with fallback for disconnected inputs
    pub fn get_value_or(&self, channel: usize, default: f32) -> f32 {
        if self.is_disconnected() {
            default
        } else {
            self.get_value(channel)
        }
    }

    /// Calculate the maximum channel count across multiple PolySignals
    pub fn max_channels(poly_signals: &[&PolySignal]) -> usize {
        poly_signals
            .iter()
            .map(|sig| sig.channels)
            .max()
            .unwrap_or(0)
    }
}

// === Connect implementation for PolySignal ===

impl crate::types::Connect for PolySignal {
    fn connect(&mut self, patch: &crate::Patch) {
        for signal in self.signals.iter_mut().take(self.channels as usize) {
            signal.connect(patch);
        }
    }
}

// === Serialization for PolySignal ===

impl Serialize for PolySignal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize as array of signals (only active channels)
        self.signals[..self.channels as usize].serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PolySignal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Accept either a single signal or an array of signals
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum PolySignalDe {
            Single(Signal),
            Array(Vec<Signal>),
        }

        match PolySignalDe::deserialize(deserializer)? {
            PolySignalDe::Single(s) => {
                // A single Disconnected signal means no connection (channels = 0)
                if matches!(s, Signal::Disconnected) {
                    Ok(PolySignal::default())
                } else {
                    Ok(PolySignal::mono(s))
                }
            }
            PolySignalDe::Array(signals) => Ok(PolySignal::poly(&signals)),
        }
    }
}

impl JsonSchema for PolySignal {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("PolySignal")
    }

    fn json_schema(r#gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
        // Schema: either a single Signal or array of Signals
        #[derive(JsonSchema)]
        #[serde(untagged)]
        #[allow(dead_code)]
        enum PolySignalSchema {
            Single(crate::types::Signal),
            Array(Vec<crate::types::Signal>),
        }
        PolySignalSchema::json_schema(r#gen)
    }
}

// =============================================================================
// MonoSignal - Polyphonic input that sums to mono output
// =============================================================================

/// A polyphonic input that sums all channels to a single mono value.
///
/// This wraps a `PolySignal` but provides a `Signal`-like API where:
/// - `is_disconnected()` returns true if the inner PolySignal has 0 channels
/// - `get_value()` returns the sum of all channels in the PolySignal
///
/// Use this for module inputs that accept polyphonic connections but produce
/// a mono control signal (e.g., stereo width, etc.).
///
/// For polyphony propagation, MonoSignal is treated as a single channel.
#[derive(Clone, Debug, Default)]
pub struct MonoSignal {
    inner: PolySignal,
}

impl MonoSignal {
    /// Create a MonoSignal from a PolySignal
    pub fn from_poly(poly: PolySignal) -> Self {
        Self { inner: poly }
    }

    /// Check if disconnected (no active channels in inner PolySignal)
    pub fn is_disconnected(&self) -> bool {
        self.inner.is_disconnected()
    }

    /// Get the summed value of all channels
    pub fn get_value(&self) -> f32 {
        self.inner
            .signals
            .iter()
            .map(|ch| ch.get_value())
            .sum::<f32>()
    }

    /// Get value with fallback for disconnected inputs (normalled input)
    pub fn get_value_or(&self, default: f32) -> f32 {
        if self.is_disconnected() {
            default
        } else {
            self.get_value()
        }
    }
}

// === Connect implementation for MonoSignal ===

impl crate::types::Connect for MonoSignal {
    fn connect(&mut self, patch: &crate::Patch) {
        self.inner.connect(patch);
    }
}

// === Serialization for MonoSignal (delegates to PolySignal) ===

impl Serialize for MonoSignal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.inner.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for MonoSignal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(MonoSignal {
            inner: PolySignal::deserialize(deserializer)?,
        })
    }
}

impl JsonSchema for MonoSignal {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("MonoSignal")
    }

    fn json_schema(r#gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
        // Reuse PolySignal's schema
        PolySignal::json_schema(r#gen)
    }
}

#[cfg(test)]
mod tests {
    use crate::dsp::utils::hz_to_voct;

    use super::*;

    #[test]
    fn test_poly_signal_default() {
        let sig = PolySignal::default();
        assert_eq!(sig.channels(), 0);
        assert!(sig.is_disconnected());
        assert_eq!(sig.get_value(0), 0.0);
    }

    #[test]
    fn test_poly_signal_deserialize_string() {
        // Deserialize "440hz" string into PolySignal
        let json = r#""440hz""#;
        let result: PolySignal = serde_json::from_str(json).expect("Failed to deserialize");
        println!("Deserialized '440hz': channels = {}", result.channels());
        assert_eq!(
            result.channels(),
            1,
            "String should deserialize to 1 channel"
        );

        let value = result.get_value(0);
        println!("Value at channel 0: {}", value);
        let target = hz_to_voct(440.0);
        assert!(
            (value - target).abs() < 0.01,
            "Value should be {} v/oct, got {}",
            target,
            value
        );
    }

    #[test]
    fn test_poly_signal_deserialize_disconnected() {
        // A single disconnected signal should result in channels = 0
        let json = r#"{"type": "disconnected"}"#;
        let result: PolySignal = serde_json::from_str(json).expect("Failed to deserialize");
        assert_eq!(result.channels(), 0, "Disconnected should have 0 channels");
        assert!(result.is_disconnected(), "Should be disconnected");
    }

    #[test]
    fn test_poly_signal_deserialize_number() {
        let json = "4.0";
        let result: PolySignal = serde_json::from_str(json).expect("Failed to deserialize");
        assert_eq!(
            result.channels(),
            1,
            "Number should deserialize to 1 channel"
        );
        assert_eq!(result.get_value(0), 4.0);
    }

    #[test]
    fn test_poly_output() {
        let mut sig = PolyOutput::default();
        sig.set_channels(3);
        sig.set(0, 1.0);
        sig.set(1, 2.0);
        sig.set(2, 3.0);
        assert_eq!(sig.channels(), 3);
        assert_eq!(sig.get(0), 1.0);
        assert_eq!(sig.get(1), 2.0);
        assert_eq!(sig.get(2), 3.0);
        assert_eq!(sig.get(3), 0.0);
    }

    #[test]
    fn test_poly_output_cycling() {
        let mut sig = PolyOutput::default();
        sig.set_channels(2);
        sig.set(0, 1.0);
        sig.set(1, 2.0);
        assert_eq!(sig.get_cycling(0), 1.0);
        assert_eq!(sig.get_cycling(1), 2.0);
        assert_eq!(sig.get_cycling(2), 1.0); // wraps
        assert_eq!(sig.get_cycling(3), 2.0); // wraps
    }

    #[test]
    fn test_poly_output_disconnected() {
        let sig = PolyOutput::default();
        assert_eq!(sig.channels(), 0);
        assert_eq!(sig.get_cycling(0), 0.0);
    }

    #[test]
    fn test_deserialize_poly() {
        use serde_json::from_str;

        // Single value
        let v: Vec<String> = from_str(r#""pink""#).map(|s: String| vec![s]).unwrap();
        assert_eq!(v, vec!["pink"]);

        // Array
        let v: Vec<String> = from_str(r#"["white", "pink", "brown"]"#).unwrap();
        assert_eq!(v, vec!["white", "pink", "brown"]);
    }

    #[test]
    fn test_mono_signal_disconnected() {
        let mono = MonoSignal::default();
        assert!(mono.is_disconnected());
        assert_eq!(mono.get_value(), 0.0);
        assert_eq!(mono.get_value_or(5.0), 5.0);
    }

    #[test]
    fn test_mono_signal_single_channel() {
        let json = "2.5";
        let mono: MonoSignal = serde_json::from_str(json).expect("Failed to deserialize");
        assert!(!mono.is_disconnected());
        assert_eq!(mono.get_value(), 2.5);
        assert_eq!(mono.get_value_or(0.0), 2.5);
    }

    #[test]
    fn test_mono_signal_sums_channels() {
        // MonoSignal should sum all channels
        let json = "[1.0, 2.0, 3.0]";
        let mono: MonoSignal = serde_json::from_str(json).expect("Failed to deserialize");
        assert!(!mono.is_disconnected());
        assert_eq!(mono.get_value(), 6.0); // 1 + 2 + 3 = 6
    }
}
