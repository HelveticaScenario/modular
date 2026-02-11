//! Query context for pattern evaluation.
//!
//! The State carries the time span being queried along with any
//! additional control values needed during evaluation.

use super::TimeSpan;
use std::collections::HashMap;

/// Query context containing the time span and optional controls.
#[derive(Clone, Debug)]
pub struct State {
    /// The time span being queried.
    pub span: TimeSpan,
    /// Optional control values (e.g., random seed).
    pub controls: Controls,
}

/// Control values that can be passed through the query.
#[derive(Clone, Debug, Default)]
pub struct Controls {
    /// Random seed for deterministic randomness.
    pub rand_seed: u64,
    /// Named control values.
    values: HashMap<String, ControlValue>,
}

/// A control value that can be stored in the controls map.
#[derive(Clone, Debug)]
pub enum ControlValue {
    Float(f64),
    Int(i64),
    String(String),
}

impl Controls {
    /// Create new empty controls.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create controls with a specific random seed.
    pub fn with_seed(seed: u64) -> Self {
        Self {
            rand_seed: seed,
            values: HashMap::new(),
        }
    }

    /// Set a float control value.
    pub fn set_float(&mut self, key: &str, value: f64) {
        self.values
            .insert(key.to_string(), ControlValue::Float(value));
    }

    /// Set an integer control value.
    pub fn set_int(&mut self, key: &str, value: i64) {
        self.values
            .insert(key.to_string(), ControlValue::Int(value));
    }

    /// Set a string control value.
    pub fn set_string(&mut self, key: &str, value: String) {
        self.values
            .insert(key.to_string(), ControlValue::String(value));
    }

    /// Get a float control value.
    pub fn get_float(&self, key: &str) -> Option<f64> {
        match self.values.get(key) {
            Some(ControlValue::Float(v)) => Some(*v),
            _ => None,
        }
    }

    /// Get an integer control value.
    pub fn get_int(&self, key: &str) -> Option<i64> {
        match self.values.get(key) {
            Some(ControlValue::Int(v)) => Some(*v),
            _ => None,
        }
    }

    /// Get a string control value.
    pub fn get_string(&self, key: &str) -> Option<&str> {
        match self.values.get(key) {
            Some(ControlValue::String(v)) => Some(v),
            _ => None,
        }
    }
}

impl State {
    /// Create a new state with the given span.
    pub fn new(span: TimeSpan) -> Self {
        Self {
            span,
            controls: Controls::default(),
        }
    }

    /// Create a new state with the given span and controls.
    pub fn with_controls(span: TimeSpan, controls: Controls) -> Self {
        Self { span, controls }
    }

    /// Create a new state with the same controls but a different span.
    pub fn set_span(&self, span: TimeSpan) -> Self {
        Self {
            span,
            controls: self.controls.clone(),
        }
    }

    /// Create a new state with the same span but different controls.
    pub fn set_controls(&self, controls: Controls) -> Self {
        Self {
            span: self.span.clone(),
            controls,
        }
    }

    /// Get the random seed from controls.
    pub fn rand_seed(&self) -> u64 {
        self.controls.rand_seed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern_system::Fraction;

    #[test]
    fn test_state_creation() {
        let span = TimeSpan::new(Fraction::from_integer(0), Fraction::from_integer(1));
        let state = State::new(span.clone());

        assert_eq!(state.span, span);
        assert_eq!(state.controls.rand_seed, 0);
    }

    #[test]
    fn test_state_set_span() {
        let span1 = TimeSpan::new(Fraction::from_integer(0), Fraction::from_integer(1));
        let span2 = TimeSpan::new(Fraction::from_integer(1), Fraction::from_integer(2));

        let state1 = State::new(span1);
        let state2 = state1.set_span(span2.clone());

        assert_eq!(state2.span, span2);
    }

    #[test]
    fn test_controls() {
        let mut controls = Controls::new();
        controls.set_float("freq", 440.0);
        controls.set_int("octave", 4);
        controls.set_string("name", "test".to_string());

        assert_eq!(controls.get_float("freq"), Some(440.0));
        assert_eq!(controls.get_int("octave"), Some(4));
        assert_eq!(controls.get_string("name"), Some("test"));
        assert_eq!(controls.get_float("missing"), None);
    }
}
