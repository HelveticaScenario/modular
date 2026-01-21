//! SeqValue enum and SeqPatternParam for the new pattern-based sequencer.
//!
//! SeqValue represents the different value types that can appear in a sequence:
//! - MIDI note numbers (f64 for cents precision)
//! - Musical notes (letter + accidental + optional octave)
//! - Signals from other modules (with optional sample-and-hold)
//! - Rests

use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    dsp::utils::midi_to_voct_f64,
    pattern_system::{
        mini::{ast::AtomValue, convert::{ConvertError, HasRest}, FromMiniAtom},
        Pattern,
    },
    types::{Connect, Signal},
    Patch,
};

use super::seq_operators::{seq_value_registry, CachedOperator};

/// A value in a sequence pattern.
///
/// Represents the different types of values that can be sequenced:
/// - MIDI notes (with cents precision via f64)
/// - Musical notes parsed from notation like "c4" or "bb"
/// - Signals from other modules, optionally sample-and-held
/// - Rests (silence/no output)
#[derive(Clone, Debug)]
pub enum SeqValue {
    /// MIDI note number (supports fractional for cents: 60.5 = C4 + 50 cents)
    Midi(f64),

    /// Musical note with optional octave (defaults to 4 if not specified)
    Note {
        letter: char,
        /// Accidental: '#' for sharp, 'b' for flat
        accidental: Option<char>,
        /// Octave number. If None, defaults to 4 during conversion.
        octave: Option<i32>,
    },

    /// Signal from another module, with optional sample-and-hold.
    /// When `sample_and_hold` is true, the signal is sampled once at hap onset.
    /// When false, the signal is read continuously.
    ///
    /// The signal field contains a raw pointer that is set during parsing
    /// and connected during the Connect phase.
    Signal {
        signal: Signal,
        sample_and_hold: bool,
    },

    /// Rest - no value output, gate goes low
    Rest,
}

impl SeqValue {
    /// Convert this value to a MIDI note number.
    /// Returns None for Rest and Signal variants.
    pub fn to_midi(&self) -> Option<f64> {
        match self {
            SeqValue::Midi(m) => Some(*m),
            SeqValue::Note {
                letter,
                accidental,
                octave,
            } => {
                let base = match letter.to_ascii_lowercase() {
                    'c' => 0,
                    'd' => 2,
                    'e' => 4,
                    'f' => 5,
                    'g' => 7,
                    'a' => 9,
                    'b' => 11,
                    _ => return None,
                };

                let acc_offset = match accidental {
                    Some('#') => 1,
                    Some('b') => -1,
                    _ => 0,
                };

                // Default octave to 4 if not specified
                let oct = octave.unwrap_or(4);
                Some(((oct + 1) * 12 + base + acc_offset) as f64)
            }
            SeqValue::Signal { .. } => None,
            SeqValue::Rest => None,
        }
    }

    /// Convert this value to V/Oct.
    /// Returns None for Rest and Signal variants.
    pub fn to_voct(&self) -> Option<f64> {
        self.to_midi().map(midi_to_voct_f64)
    }

    /// Check if this is a rest value.
    pub fn is_rest(&self) -> bool {
        matches!(self, SeqValue::Rest)
    }

    /// Check if this is a signal value.
    pub fn is_signal(&self) -> bool {
        matches!(self, SeqValue::Signal { .. })
    }

    /// Check if this is a sample-and-hold signal.
    pub fn is_sample_and_hold(&self) -> bool {
        matches!(self, SeqValue::Signal { sample_and_hold: true, .. })
    }

    /// Apply MIDI offset (for add operator on static values).
    pub fn add_midi(&self, offset: f64) -> SeqValue {
        match self {
            SeqValue::Midi(m) => SeqValue::Midi(m + offset),
            SeqValue::Note { .. } => {
                if let Some(midi) = self.to_midi() {
                    SeqValue::Midi(midi + offset)
                } else {
                    self.clone()
                }
            }
            _ => self.clone(),
        }
    }

    /// Apply MIDI multiplication (for mul operator on static values).
    pub fn mul_midi(&self, factor: f64) -> SeqValue {
        match self {
            SeqValue::Midi(m) => SeqValue::Midi(m * factor),
            SeqValue::Note { .. } => {
                if let Some(midi) = self.to_midi() {
                    SeqValue::Midi(midi * factor)
                } else {
                    self.clone()
                }
            }
            _ => self.clone(),
        }
    }
}

impl FromMiniAtom for SeqValue {
    fn from_atom(atom: &AtomValue) -> Result<Self, ConvertError> {
        match atom {
            AtomValue::Number(n) => Ok(SeqValue::Midi(*n)),
            AtomValue::Midi(m) => Ok(SeqValue::Midi(*m as f64)),
            AtomValue::Hz(hz) => {
                // Convert Hz to MIDI: MIDI = 12 * log2(f / 440) + 69
                let midi = 12.0 * (hz / 440.0).log2() + 69.0;
                Ok(SeqValue::Midi(midi))
            }
            AtomValue::Volts(v) => {
                // Convert V/Oct to MIDI: MIDI = voct * 12 + 33
                let midi = v * 12.0 + 33.0;
                Ok(SeqValue::Midi(midi))
            }
            AtomValue::Note {
                letter,
                accidental,
                octave,
            } => Ok(SeqValue::Note {
                letter: *letter,
                accidental: accidental.clone(),
                octave: *octave,
            }),
            AtomValue::Identifier(s) => {
                // Check for module reference syntax: module(id:port) or module(id:port)=
                if let Some(parsed) = parse_module_ref(s) {
                    return Ok(parsed);
                }
                // Otherwise treat as note without octave if single letter
                if s.len() == 1 {
                    let c = s.chars().next().unwrap().to_ascii_lowercase();
                    if ('a'..='g').contains(&c) {
                        return Ok(SeqValue::Note {
                            letter: c,
                            accidental: None,
                            octave: None,
                        });
                    }
                }
                Err(ConvertError::InvalidAtom(format!(
                    "Cannot convert '{}' to SeqValue",
                    s
                )))
            }
            AtomValue::String(s) => {
                // Check for module reference in string
                if let Some(parsed) = parse_module_ref(s) {
                    return Ok(parsed);
                }
                Err(ConvertError::InvalidAtom(format!(
                    "Cannot convert string '{}' to SeqValue",
                    s
                )))
            }
        }
    }

    fn from_list(atoms: &[AtomValue]) -> Result<Self, ConvertError> {
        // Lists are handled by the scale operator, not here
        if atoms.len() == 1 {
            Self::from_atom(&atoms[0])
        } else {
            Err(ConvertError::ListNotSupported)
        }
    }

    fn combine_with_head(_head_atoms: &[AtomValue], _tail: &Self) -> Result<Self, ConvertError> {
        // SeqValue doesn't support head:tail combination directly.
        // Use operators like scale() for combining notes with scale patterns.
        Err(ConvertError::ListNotSupported)
    }

    fn rest_value() -> Option<Self> {
        Some(SeqValue::Rest)
    }

    fn supports_rest() -> bool {
        true
    }
}

impl HasRest for SeqValue {
    fn rest_value() -> Self {
        SeqValue::Rest
    }
}

/// Parse a module reference string like "module(id:port)" or "module(id:port)=".
/// The `=` suffix indicates sample-and-hold mode.
fn parse_module_ref(s: &str) -> Option<SeqValue> {
    // Check for module( prefix
    if !s.starts_with("module(") {
        return None;
    }

    let sample_and_hold = s.ends_with("=");
    let trimmed = if sample_and_hold {
        &s[7..s.len() - 2] // Remove "module(" and ")="
    } else if s.ends_with(')') {
        &s[7..s.len() - 1] // Remove "module(" and ")"
    } else {
        return None;
    };

    // Parse id:port
    let parts: Vec<&str> = trimmed.splitn(2, ':').collect();
    if parts.len() != 2 {
        return None;
    }

    let module_id = parts[0].to_string();
    let port = parts[1].to_string();

    Some(SeqValue::Signal {
        signal: Signal::Cable {
            module: module_id,
            module_ptr: std::sync::Weak::new(),
            port,
        },
        sample_and_hold,
    })
}

/// A pattern parameter that wraps a pattern string and its parsed components.
///
/// This struct is serialized as a simple string but contains the parsed pattern
/// and collected signal pointers for connection.
///
/// # Safety
///
/// The `signals` field contains raw pointers to Signal fields within the Pattern.
/// These pointers remain valid as long as the Pattern is not dropped or reallocated.
/// Since SeqPatternParam owns both the pattern and signals, and patterns are
/// immutable after parsing, this is safe.
#[derive(Default, JsonSchema)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct SeqPatternParam {
    /// The source pattern string (used for serialization)
    #[allow(dead_code)]
    source: String,

    /// The parsed pattern (skipped in serialization)
    #[serde(skip, default)]
    #[schemars(skip)]
    pub(crate) pattern: Option<Pattern<SeqValue>>,

    /// Pointers to Signal fields within the pattern for connection.
    /// SAFETY: These are valid as long as the pattern is not dropped.
    #[serde(skip, default)]
    #[schemars(skip)]
    pub(crate) signals: Vec<*mut Signal>,

    /// Cached operators to apply at runtime for signal values.
    #[serde(skip, default)]
    #[schemars(skip)]
    pub(crate) operators: Vec<CachedOperator>,
}

// SAFETY: SeqPatternParam is Send because:
// - The Signal pointers are only used on the audio thread after connection
// - The pattern is immutable after parsing
// - All access to signals goes through the owning pattern
unsafe impl Send for SeqPatternParam {}
unsafe impl Sync for SeqPatternParam {}

impl SeqPatternParam {
    /// Parse a pattern string and collect signals.
    fn parse(source: &str) -> Result<Self, String> {
        let registry = seq_value_registry();

        // Parse with operators - we'll need to track operators applied
        let pattern = crate::pattern_system::mini::parse_with_operators::<SeqValue>(source, &registry)
            .map_err(|e| e.to_string())?;

        // TODO: Collect signals from pattern - this requires walking the pattern
        // For now, signals will be connected via the Connect trait on individual values

        Ok(Self {
            source: source.to_string(),
            pattern: Some(pattern),
            signals: Vec::new(),
            operators: Vec::new(),
        })
    }

    /// Get the parsed pattern.
    pub fn pattern(&self) -> Option<&Pattern<SeqValue>> {
        self.pattern.as_ref()
    }
}

impl<'de> Deserialize<'de> for SeqPatternParam {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let source = String::deserialize(deserializer)?;
        if source.is_empty() {
            return Ok(Self::default());
        }
        Self::parse(&source).map_err(|e| serde::de::Error::custom(e))
    }
}

impl Connect for SeqPatternParam {
    fn connect(&mut self, patch: &Patch) {
        // Connect all collected signals
        for signal_ptr in &mut self.signals {
            // SAFETY: Pointers are valid as long as pattern exists
            unsafe {
                (**signal_ptr).connect(patch);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seq_value_to_midi() {
        assert_eq!(SeqValue::Midi(60.0).to_midi(), Some(60.0));
        assert_eq!(SeqValue::Midi(60.5).to_midi(), Some(60.5));

        let c4 = SeqValue::Note {
            letter: 'c',
            accidental: None,
            octave: Some(4),
        };
        assert_eq!(c4.to_midi(), Some(60.0));

        let c_no_oct = SeqValue::Note {
            letter: 'c',
            accidental: None,
            octave: None,
        };
        assert_eq!(c_no_oct.to_midi(), Some(60.0)); // Defaults to octave 4

        let cs4 = SeqValue::Note {
            letter: 'c',
            accidental: Some('#'),
            octave: Some(4),
        };
        assert_eq!(cs4.to_midi(), Some(61.0));

        assert_eq!(SeqValue::Rest.to_midi(), None);
    }

    #[test]
    fn test_parse_module_ref() {
        let sig = parse_module_ref("module(osc1:output)").unwrap();
        assert!(matches!(sig, SeqValue::Signal { sample_and_hold: false, .. }));

        let sh_sig = parse_module_ref("module(osc1:output)=").unwrap();
        assert!(matches!(sh_sig, SeqValue::Signal { sample_and_hold: true, .. }));

        assert!(parse_module_ref("not_a_module").is_none());
    }

    #[test]
    fn test_from_atom() {
        let n = SeqValue::from_atom(&AtomValue::Number(60.0)).unwrap();
        assert!(matches!(n, SeqValue::Midi(m) if m == 60.0));

        let note = SeqValue::from_atom(&AtomValue::Note {
            letter: 'a',
            accidental: None,
            octave: Some(4),
        })
        .unwrap();
        assert!(matches!(
            note,
            SeqValue::Note {
                letter: 'a',
                octave: Some(4),
                ..
            }
        ));
    }

    #[test]
    fn test_note_octaves_different() {
        use crate::pattern_system::mini::parse;
        use crate::pattern_system::Fraction;

        // Parse "a1 a2 a3 a4" and check each note has different octave
        let pattern: crate::pattern_system::Pattern<SeqValue> =
            parse("a1 a2 a3 a4").expect("Should parse");

        let haps = pattern.query_arc(Fraction::from(0), Fraction::from(1));

        assert_eq!(haps.len(), 4, "Should have 4 haps");

        let midis: Vec<f64> = haps
            .iter()
            .filter_map(|h| h.value.to_midi())
            .collect();

        // a1 = 33, a2 = 45, a3 = 57, a4 = 69
        assert_eq!(midis[0], 33.0, "a1 should be MIDI 33");
        assert_eq!(midis[1], 45.0, "a2 should be MIDI 45");
        assert_eq!(midis[2], 57.0, "a3 should be MIDI 57");
        assert_eq!(midis[3], 69.0, "a4 should be MIDI 69");
    }

    #[test]
    fn test_seq_value_supports_rest() {
        use crate::pattern_system::mini::convert::FromMiniAtom;
        // SeqValue should support rests
        assert!(SeqValue::supports_rest());
        assert!(<SeqValue as FromMiniAtom>::rest_value().is_some());
        assert!(matches!(<SeqValue as FromMiniAtom>::rest_value(), Some(SeqValue::Rest)));
    }

    #[test]
    fn test_seq_value_has_rest_trait() {
        // Test HasRest trait implementation
        use crate::pattern_system::HasRest;
        let rest = <SeqValue as HasRest>::rest_value();
        assert!(matches!(rest, SeqValue::Rest));
    }

    #[test]
    fn test_seq_value_rest_in_pattern() {
        use crate::pattern_system::mini::parse;
        use crate::pattern_system::Fraction;

        // SeqValue should allow rest (~) in patterns
        let pattern: crate::pattern_system::Pattern<SeqValue> =
            parse("c4 ~ e4").expect("Should parse with rest");

        let haps = pattern.query_arc(Fraction::from(0), Fraction::from(1));
        assert_eq!(haps.len(), 3, "Should have 3 haps including rest");
        
        // Second hap should be a rest
        assert!(haps[1].value.is_rest(), "Second hap should be a rest");
    }

    #[test]
    fn test_seq_value_degrade_in_pattern() {
        use crate::pattern_system::mini::parse;
        use crate::pattern_system::Fraction;

        // SeqValue should allow degrade (?) in patterns
        let pattern: crate::pattern_system::Pattern<SeqValue> =
            parse("c4?").expect("Should parse with degrade");

        // Query multiple times - should always get a hap (either note or rest)
        for i in 0..10 {
            let haps = pattern.query_arc(Fraction::from(i), Fraction::from(i + 1));
            assert_eq!(haps.len(), 1, "Should always have exactly 1 hap at cycle {}", i);
        }
    }

    #[test]
    fn test_seq_value_euclidean_in_pattern() {
        use crate::pattern_system::mini::parse;
        use crate::pattern_system::Fraction;

        // SeqValue should allow euclidean in patterns
        let pattern: crate::pattern_system::Pattern<SeqValue> =
            parse("c4(3,8)").expect("Should parse with euclidean");

        let haps = pattern.query_arc(Fraction::from(0), Fraction::from(1));
        
        // Should have 8 haps (3 notes + 5 rests)
        assert_eq!(haps.len(), 8, "Should have 8 haps (euclidean 3,8)");
        
        // Count pulses (non-rests)
        let pulse_count = haps.iter().filter(|h| !h.value.is_rest()).count();
        assert_eq!(pulse_count, 3, "Should have 3 pulses");
    }
}
