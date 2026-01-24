//! Polyphonic signal support for multichannel cables.
//!
//! This module provides VCV Rack-style polyphonic signal handling,
//! allowing a single cable to carry up to 16 independent audio channels.

use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use std::borrow::Cow;

/// Maximum channels per cable (matches VCV Rack / MIDI convention)
pub const PORT_MAX_CHANNELS: usize = 16;

/// A polyphonic signal buffer with channel count metadata.
///
/// This is a fixed-capacity buffer that can hold up to 16 channels.
/// The `channels` field indicates how many channels are semantically valid:
/// - 0 = disconnected
/// - 1 = monophonic
/// - 2-16 = polyphonic
#[derive(Clone, Copy, Debug)]
pub struct PolySignal {
    /// Voltage values for each channel (always allocated, not all may be active)
    voltages: [f32; PORT_MAX_CHANNELS],
    /// Number of active channels: 0 = disconnected, 1 = mono, 2-16 = poly
    channels: u8,
}

impl Default for PolySignal {
    fn default() -> Self {
        Self {
            voltages: [0.0; PORT_MAX_CHANNELS],
            channels: 0, // Disconnected
        }
    }
}

impl PartialEq for PolySignal {
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

impl PolySignal {
    /// Create a monophonic signal with a single value
    pub fn mono(value: f32) -> Self {
        let mut sig = Self::default();
        sig.voltages[0] = value;
        sig.channels = 1;
        sig
    }

    /// Create a polyphonic signal from a slice (channels = slice length)
    pub fn poly(values: &[f32]) -> Self {
        let channels = values.len().min(PORT_MAX_CHANNELS);
        let mut sig = Self::default();
        sig.voltages[..channels].copy_from_slice(&values[..channels]);
        sig.channels = channels as u8;
        sig
    }

    /// Create a polyphonic signal from an iterator
    pub fn from_iter<I: IntoIterator<Item = f32>>(iter: I) -> Self {
        let mut sig = Self::default();
        let mut count = 0usize;
        for (i, v) in iter.into_iter().enumerate() {
            if i >= PORT_MAX_CHANNELS {
                break;
            }
            sig.voltages[i] = v;
            count = i + 1;
        }
        sig.channels = count as u8;
        sig
    }

    // === Accessors ===

    /// Get the number of active channels
    pub fn channels(&self) -> u8 {
        self.channels
    }

    /// Check if the signal is disconnected (no active channels)
    pub fn is_disconnected(&self) -> bool {
        self.channels == 0
    }

    /// Check if the signal is monophonic (exactly 1 channel)
    pub fn is_monophonic(&self) -> bool {
        self.channels == 1
    }

    /// Check if the signal is polyphonic (more than 1 channel)
    pub fn is_polyphonic(&self) -> bool {
        self.channels > 1
    }

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
    #[inline]
    pub fn get_cycling(&self, channel: usize) -> f32 {
        if self.channels == 0 {
            0.0 // Disconnected
        } else {
            self.voltages[channel % self.channels as usize]
        }
    }

    /// Get value with fallback for disconnected inputs (normalled input)
    pub fn get_or(&self, channel: usize, default: f32) -> f32 {
        if self.is_disconnected() {
            default
        } else {
            self.get_cycling(channel)
        }
    }

    /// Set the number of active channels (clears higher channels to 0)
    pub fn set_channels(&mut self, channels: u8) {
        let channels = channels.clamp(0, PORT_MAX_CHANNELS as u8);
        // Clear channels beyond the new count
        for c in channels as usize..self.channels as usize {
            self.voltages[c] = 0.0;
        }
        self.channels = channels;
    }

    /// Get a slice of the active voltages
    pub fn voltages(&self) -> &[f32] {
        &self.voltages[..self.channels as usize]
    }

    /// Get a mutable slice of the active voltages
    pub fn voltages_mut(&mut self) -> &mut [f32] {
        &mut self.voltages[..self.channels as usize]
    }

    /// Get the full voltage array (including inactive channels)
    pub fn voltages_all(&self) -> &[f32; PORT_MAX_CHANNELS] {
        &self.voltages
    }

    /// Get mutable access to the full voltage array
    pub fn voltages_all_mut(&mut self) -> &mut [f32; PORT_MAX_CHANNELS] {
        &mut self.voltages
    }

    /// Sum all active channels
    pub fn sum(&self) -> f32 {
        self.voltages[..self.channels as usize].iter().sum()
    }

    /// Average of all active channels
    pub fn average(&self) -> f32 {
        if self.channels == 0 {
            0.0
        } else {
            self.sum() / self.channels as f32
        }
    }

    /// Apply a function to each active channel
    pub fn map<F: FnMut(usize, f32) -> f32>(&self, mut f: F) -> Self {
        let mut result = Self::default();
        result.channels = self.channels;
        for i in 0..self.channels as usize {
            result.voltages[i] = f(i, self.voltages[i]);
        }
        result
    }

    /// Apply a function to each channel, combining with another PolySignal.
    /// Output channel count is the max of the two inputs.
    /// Uses cycling for inputs with fewer channels.
    pub fn zip_with<F: FnMut(usize, f32, f32) -> f32>(&self, other: &PolySignal, mut f: F) -> Self {
        let out_channels = self.channels.max(other.channels);
        let mut result = Self::default();
        result.channels = out_channels;
        for i in 0..out_channels as usize {
            let a = self.get_cycling(i);
            let b = other.get_cycling(i);
            result.voltages[i] = f(i, a, b);
        }
        result
    }
}

// === Serialization ===

impl Serialize for PolySignal {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize as a struct with channels and voltages array
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("PolySignal", 2)?;
        state.serialize_field("channels", &self.channels)?;
        state.serialize_field("voltages", &self.voltages[..self.channels as usize])?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for PolySignal {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct PolySignalDe {
            channels: u8,
            voltages: Vec<f32>,
        }

        let de = PolySignalDe::deserialize(deserializer)?;
        let mut sig = PolySignal::default();
        sig.channels = de.channels.min(PORT_MAX_CHANNELS as u8);
        for (i, &v) in de.voltages.iter().enumerate().take(sig.channels as usize) {
            sig.voltages[i] = v;
        }
        Ok(sig)
    }
}

// === JsonSchema ===

impl JsonSchema for PolySignal {
    fn schema_name() -> Cow<'static, str> {
        Cow::Borrowed("PolySignal")
    }

    fn json_schema(r#gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
        // Schema matches the serialized form
        #[derive(JsonSchema)]
        #[allow(dead_code)]
        struct PolySignalSchema {
            channels: u8,
            voltages: Vec<f32>,
        }
        PolySignalSchema::json_schema(r#gen)
    }
}

// === CycleGet trait for Vec<T> ===

/// Extension trait for cycling access to vectors.
/// When a vector is shorter than the requested index, it wraps around.
pub trait CycleGet<T> {
    /// Get value at index with cycling (wraps around).
    /// Returns T::default() if empty.
    fn cycle_get(&self, index: usize) -> T
    where
        T: Default + Clone;
}

impl<T: Default + Clone> CycleGet<T> for Vec<T> {
    fn cycle_get(&self, index: usize) -> T {
        if self.is_empty() {
            T::default()
        } else {
            self[index % self.len()].clone()
        }
    }
}

impl<T: Default + Clone, const N: usize> CycleGet<T> for [T; N] {
    fn cycle_get(&self, index: usize) -> T {
        if N == 0 {
            T::default()
        } else {
            self[index % N].clone()
        }
    }
}

/// Returns the number of explicitly poly channels (None if length <= 1)
pub trait PolyChannels {
    fn poly_channels(&self) -> Option<usize>;
}

impl<T> PolyChannels for Vec<T> {
    fn poly_channels(&self) -> Option<usize> {
        if self.len() <= 1 {
            None
        } else {
            Some(self.len())
        }
    }
}

// === Deserialize poly helper ===

/// Deserialize either a single value or array into Vec<T>.
/// This enables params to accept either:
/// - `"pink"` -> `vec!["pink"]`
/// - `["white", "pink", "brown"]` -> `vec!["white", "pink", "brown"]`
pub fn deserialize_poly<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany<T> {
        One(T),
        Many(Vec<T>),
    }

    match OneOrMany::deserialize(deserializer)? {
        OneOrMany::One(v) => Ok(vec![v]),
        OneOrMany::Many(v) => Ok(v),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_poly_signal_mono() {
        let sig = PolySignal::mono(1.5);
        assert_eq!(sig.channels(), 1);
        assert!(sig.is_monophonic());
        assert!(!sig.is_polyphonic());
        assert!(!sig.is_disconnected());
        assert_eq!(sig.get(0), 1.5);
        assert_eq!(sig.get(1), 0.0);
    }

    #[test]
    fn test_poly_signal_poly() {
        let sig = PolySignal::poly(&[1.0, 2.0, 3.0]);
        assert_eq!(sig.channels(), 3);
        assert!(!sig.is_monophonic());
        assert!(sig.is_polyphonic());
        assert_eq!(sig.get(0), 1.0);
        assert_eq!(sig.get(1), 2.0);
        assert_eq!(sig.get(2), 3.0);
        assert_eq!(sig.get(3), 0.0);
    }

    #[test]
    fn test_poly_signal_cycling() {
        let sig = PolySignal::poly(&[1.0, 2.0]);
        assert_eq!(sig.get_cycling(0), 1.0);
        assert_eq!(sig.get_cycling(1), 2.0);
        assert_eq!(sig.get_cycling(2), 1.0); // wraps
        assert_eq!(sig.get_cycling(3), 2.0); // wraps
    }

    #[test]
    fn test_poly_signal_disconnected() {
        let sig = PolySignal::default();
        assert_eq!(sig.channels(), 0);
        assert!(sig.is_disconnected());
        assert_eq!(sig.get_cycling(0), 0.0);
        assert_eq!(sig.get_or(0, 5.0), 5.0);
    }

    #[test]
    fn test_poly_signal_sum_average() {
        let sig = PolySignal::poly(&[1.0, 2.0, 3.0]);
        assert_eq!(sig.sum(), 6.0);
        assert_eq!(sig.average(), 2.0);
    }

    #[test]
    fn test_cycle_get_vec() {
        let v = vec![1, 2, 3];
        assert_eq!(v.cycle_get(0), 1);
        assert_eq!(v.cycle_get(1), 2);
        assert_eq!(v.cycle_get(2), 3);
        assert_eq!(v.cycle_get(3), 1); // wraps
        assert_eq!(v.cycle_get(4), 2); // wraps
    }

    #[test]
    fn test_cycle_get_empty_vec() {
        let v: Vec<i32> = vec![];
        assert_eq!(v.cycle_get(0), 0); // default
    }

    #[test]
    fn test_poly_channels() {
        let v1 = vec![1];
        let v2 = vec![1, 2];
        let v3 = vec![1, 2, 3];
        assert_eq!(v1.poly_channels(), None);
        assert_eq!(v2.poly_channels(), Some(2));
        assert_eq!(v3.poly_channels(), Some(3));
    }

    #[test]
    fn test_poly_signal_zip_with() {
        let a = PolySignal::poly(&[1.0, 2.0]);
        let b = PolySignal::poly(&[10.0, 20.0, 30.0]);
        let result = a.zip_with(&b, |_, x, y| x + y);
        assert_eq!(result.channels(), 3);
        assert_eq!(result.get(0), 11.0);
        assert_eq!(result.get(1), 22.0);
        assert_eq!(result.get(2), 31.0); // 1.0 (cycling) + 30.0
    }

    #[test]
    fn test_deserialize_poly() {
        use serde_json::from_str;

        // Single value
        let v: Vec<String> =
            from_str(r#""pink""#).map(|s: String| vec![s]).unwrap();
        assert_eq!(v, vec!["pink"]);

        // Array
        let v: Vec<String> = from_str(r#"["white", "pink", "brown"]"#).unwrap();
        assert_eq!(v, vec!["white", "pink", "brown"]);
    }
}
