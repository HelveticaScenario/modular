use num::Float;

use crate::dsp::consts::{LUT_PITCH_RATIO_HIGH, LUT_PITCH_RATIO_LOW};

/// Map a value from one range to another. If the input range is degenerate, returns `y0`.
pub fn map_range(x: f32, x0: f32, x1: f32, y0: f32, y1: f32) -> f32 {
    let denom = x1 - x0;
    if denom.abs() < f32::EPSILON {
        return y0;
    }
    (x - x0) * (y1 - y0) / denom + y0
}

fn make_integral_fractional(x: f32) -> (i32, f32) {
    let integral: i32 = x as i32;
    let fractional: f32 = x - (integral as f32);
    (integral, fractional)
}

pub fn semitones_to_ratio(semitones: f32) -> f32 {
    let pitch: f32 = semitones + 128.0;
    let (pitch_integral, pitch_fractional) = make_integral_fractional(pitch);

    return LUT_PITCH_RATIO_HIGH[pitch_integral as usize]
        * LUT_PITCH_RATIO_LOW[(pitch_fractional * 256.0) as usize];
}

pub fn interpolate(table: &'static [f32], mut index: f32, size: usize) -> f32 {
    index *= size as f32;
    let (index_integral, index_fractional) = make_integral_fractional(index);
    let a: f32 = table[index_integral as usize];
    let b: f32 = table[(index_integral + 1) as usize];
    return a + (b - a) * index_fractional;
}

pub fn wrap<T>(range: std::ops::Range<T>, mut val: T) -> T
where
    T: std::ops::Sub<Output = T>
        + std::ops::AddAssign
        + std::cmp::PartialOrd
        + std::ops::SubAssign
        + Copy,
{
    let span = range.end - range.start;
    while val > range.end {
        val -= span;
    }
    while val < range.start {
        val += span;
    }
    val
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for make_integral_fractional
    #[test]
    fn test_make_integral_fractional_positive() {
        let (integral, fractional) = make_integral_fractional(3.75);
        assert_eq!(integral, 3);
        assert!((fractional - 0.75).abs() < 0.0001);
    }

    #[test]
    fn test_make_integral_fractional_whole_number() {
        let (integral, fractional) = make_integral_fractional(5.0);
        assert_eq!(integral, 5);
        assert!((fractional - 0.0).abs() < 0.0001);
    }

    #[test]
    fn test_make_integral_fractional_negative() {
        let (integral, fractional) = make_integral_fractional(-2.25);
        assert_eq!(integral, -2);
        // -2.25 - (-2) = -0.25
        assert!((fractional - (-0.25)).abs() < 0.0001);
    }

    // Tests for semitones_to_ratio
    #[test]
    fn test_semitones_to_ratio_zero() {
        // 0 semitones = ratio of 1.0
        let ratio = semitones_to_ratio(0.0);
        assert!(
            (ratio - 1.0).abs() < 0.01,
            "0 semitones should be ratio 1.0, got {}",
            ratio
        );
    }

    #[test]
    fn test_semitones_to_ratio_octave_up() {
        // 12 semitones = ratio of 2.0 (one octave up)
        let ratio = semitones_to_ratio(12.0);
        assert!(
            (ratio - 2.0).abs() < 0.01,
            "12 semitones should be ratio 2.0, got {}",
            ratio
        );
    }

    #[test]
    fn test_semitones_to_ratio_octave_down() {
        // -12 semitones = ratio of 0.5 (one octave down)
        let ratio = semitones_to_ratio(-12.0);
        assert!(
            (ratio - 0.5).abs() < 0.01,
            "-12 semitones should be ratio 0.5, got {}",
            ratio
        );
    }

    #[test]
    fn test_semitones_to_ratio_perfect_fifth() {
        // 7 semitones = ratio of ~1.498 (perfect fifth)
        let ratio = semitones_to_ratio(7.0);
        assert!(
            (ratio - 1.498).abs() < 0.01,
            "7 semitones should be ~1.498, got {}",
            ratio
        );
    }

    // Tests for wrap
    #[test]
    fn test_wrap_within_range() {
        let result: f32 = wrap(0.0..1.0, 0.5);
        assert!((result - 0.5).abs() < 0.0001);
    }

    #[test]
    fn test_wrap_above_range() {
        let result: f32 = wrap(0.0..1.0, 1.5);
        assert!(
            (result - 0.5).abs() < 0.0001,
            "1.5 wrapped to 0-1 should be 0.5, got {}",
            result
        );
    }

    #[test]
    fn test_wrap_below_range() {
        let result: f32 = wrap(0.0..1.0, -0.25);
        assert!(
            (result - 0.75).abs() < 0.0001,
            "-0.25 wrapped to 0-1 should be 0.75, got {}",
            result
        );
    }

    #[test]
    fn test_wrap_multiple_times_above() {
        let result: f32 = wrap(0.0..1.0, 2.75);
        assert!(
            (result - 0.75).abs() < 0.0001,
            "2.75 wrapped to 0-1 should be 0.75, got {}",
            result
        );
    }

    #[test]
    fn test_wrap_phase_range() {
        // Common use case: wrapping phase in range 0..2*PI
        use std::f32::consts::PI;
        let result: f32 = wrap(0.0..(2.0 * PI), 3.0 * PI);
        assert!(
            (result - PI).abs() < 0.0001,
            "3*PI wrapped to 0..2*PI should be PI, got {}",
            result
        );
    }

    #[test]
    fn test_wrap_integer() {
        let result = wrap(0..10, 15);
        assert_eq!(result, 5);
    }

    #[test]
    fn test_map_range() {
        assert!((map_range(0.5, 0.0, 1.0, -1.0, 1.0) - 0.0).abs() < 1e-6);
        assert_eq!(map_range(1.0, 1.0, 1.0, 2.0, 4.0), 2.0);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SchmittState {
    Low,
    High,
    Uninitialized,
}

/// Reusable Schmitt trigger with hysteresis
#[derive(Debug, Clone, Copy)]
pub struct SchmittTrigger {
    pub state: SchmittState,
    low_threshold: f32,
    high_threshold: f32,
}

impl SchmittTrigger {
    /// Create a new Schmitt trigger with the given thresholds
    pub fn new(low_threshold: f32, high_threshold: f32) -> Self {
        Self {
            state: SchmittState::Uninitialized,
            low_threshold,
            high_threshold,
        }
    }

    /// Process a sample through the Schmitt trigger
    /// Returns true if it toggled from low to high
    pub fn process(&mut self, input: f32) -> bool {
        match self.state {
            SchmittState::Uninitialized => {
                // Initialize state based on input
                if input >= self.high_threshold {
                    self.state = SchmittState::High;
                } else {
                    self.state = SchmittState::Low;
                }
            }
            SchmittState::High => {
                // Currently high - check if we should go low
                if input < self.low_threshold {
                    self.state = SchmittState::Low;
                }
            }
            SchmittState::Low => {
                // Currently low - check if we should go high
                if input > self.high_threshold {
                    self.state = SchmittState::High;
                    return true;
                }
            }
        }

        false
    }

    /// Update thresholds
    pub fn set_thresholds(&mut self, low_threshold: f32, high_threshold: f32) {
        self.low_threshold = low_threshold;
        self.high_threshold = high_threshold;
    }

    /// Get current state
    pub fn state(&self) -> SchmittState {
        self.state
    }

    /// Reset state to Uninitialized
    pub fn reset(&mut self) {
        self.state = SchmittState::Uninitialized;
    }
}

impl Default for SchmittTrigger {
    fn default() -> Self {
        Self::new(-1.0, 1.0)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TempGateState {
    #[default]
    Low,
    High,
}

#[derive(Clone, Copy, Default, PartialEq)]
pub struct TempGate {
    target: TempGateState,
    state: TempGateState,
    low_val: f32,
    high_val: f32,
}

impl TempGate {
    pub fn new(state: TempGateState, low_val: f32, high_val: f32) -> Self {
        Self {
            target: state,
            state,
            low_val,
            high_val,
        }
    }

    pub fn set_state(&mut self, state: TempGateState, target: TempGateState) {
        self.state = state;
        self.target = target;
    }

    pub fn process(&mut self) -> f32 {
        let state = self.state;
        if self.state != self.target {
            self.state = self.target;
        }
        match state {
            TempGateState::Low => self.low_val,
            TempGateState::High => self.high_val,
        }
    }
}

// ============ Pitch Conversion Functions ============

pub fn hz_to_voct_f64(frequency_hz: f64) -> f64 {
    // Matches src/dsl/factories.ts hz(): log2(f / 27.5)
    (frequency_hz / 27.5).log2()
}

/// Convert MIDI note number to V/Oct (A0 = 0V = MIDI 21)
pub fn midi_to_voct_f64(midi: f64) -> f64 {
    (midi - 21.0) / 12.0
}

/// Convert V/Oct to MIDI note number
pub fn voct_to_midi_f64(voct: f64) -> f64 {
    voct * 12.0 + 21.0
}

/// Convert V/Oct to frequency in Hz
pub fn voct_to_hz_f64(voct: f64) -> f64 {
    27.5 * 2.0.powf(voct)
}

pub fn hz_to_voct(frequency_hz: f32) -> f32 {
    // Matches src/dsl/factories.ts hz(): log2(f / 27.5)
    (frequency_hz / 27.5).log2()
}

/// Convert MIDI note number to V/Oct (A0 = 0V = MIDI 21)
pub fn midi_to_voct(midi: f32) -> f32 {
    (midi - 21.0) / 12.0
}

/// Convert V/Oct to MIDI note number
pub fn voct_to_midi(voct: f32) -> f32 {
    voct * 12.0 + 21.0
}

/// Convert V/Oct to frequency in Hz
pub fn voct_to_hz(voct: f32) -> f32 {
    27.5 * 2.0.powf(voct)
}
