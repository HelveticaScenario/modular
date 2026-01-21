//! Abstract Syntax Tree types for mini notation.
//!
//! These types represent the parsed structure of a mini notation pattern,
//! including source span information for editor highlighting.

use crate::pattern_system::SourceSpan;

/// Source location in the original pattern string.
#[derive(Clone, Debug, PartialEq)]
pub struct Located<T> {
    pub node: T,
    pub span: SourceSpan,
}

impl<T> Located<T> {
    pub fn new(node: T, start: usize, end: usize) -> Self {
        Self {
            node,
            span: SourceSpan::new(start, end),
        }
    }
}

/// The main AST node type.
#[derive(Clone, Debug, PartialEq)]
pub enum MiniAST {
    /// A single value atom.
    Pure(Located<AtomValue>),

    /// Rest/silence.
    Rest(SourceSpan),

    /// A list of patterns (from tail syntax: c:e:g or c:[e f]).
    /// Elements can be atoms or subpatterns.
    List(Located<Vec<MiniAST>>),

    /// Sequence of patterns (space-separated, played in order).
    Sequence(Vec<(MiniAST, Option<f64>)>), // (pattern, optional weight)

    /// Stack of patterns (comma-separated, played simultaneously).
    Stack(Vec<MiniAST>),

    /// Slow subsequence (one item per cycle).
    SlowCat(Vec<MiniAST>),

    /// Random choice between values.
    RandomChoice(Vec<MiniAST>),

    /// Integer range (0..4 → [0, 1, 2, 3, 4]).
    Range(i64, i64),

    /// Polymeter: different length sequences played simultaneously.
    PolyMeter(Vec<MiniAST>),

    /// Fast modifier: pattern * factor.
    Fast(Box<MiniAST>, Box<MiniAST>),

    /// Slow modifier: pattern / factor.
    Slow(Box<MiniAST>, Box<MiniAST>),

    /// Replicate: pattern ! n (repeat n times).
    Replicate(Box<MiniAST>, u32),

    /// Degrade: pattern ? prob (randomly drop with probability).
    Degrade(Box<MiniAST>, Option<f64>),

    /// Euclidean rhythm: pattern(pulses, steps, rotation?).
    Euclidean {
        pattern: Box<MiniAST>,
        pulses: u32,
        steps: u32,
        rotation: Option<u32>,
    },

    /// Pattern with operator chain.
    WithOperators {
        base: Box<MiniAST>,
        operators: Vec<OperatorCall>,
    },
}

/// Atomic value types.
#[derive(Clone, Debug, PartialEq)]
pub enum AtomValue {
    /// Numeric value.
    Number(f64),

    /// MIDI note number.
    Midi(i32),

    /// Frequency in Hz.
    Hz(f64),

    /// Voltage (for modular synth).
    Volts(f64),

    /// Musical note (e.g., c4, a#3, bb5).
    Note {
        letter: char,
        /// Accidental: '#' for sharp, 'b' for flat
        accidental: Option<char>,
        octave: Option<i32>,
    },

    /// String identifier (for scale names, sample names, etc.).
    Identifier(String),

    /// Quoted string.
    String(String),
}

impl AtomValue {
    /// Convert to f64 if possible.
    pub fn to_f64(&self) -> Option<f64> {
        match self {
            AtomValue::Number(n) => Some(*n),
            AtomValue::Midi(m) => Some(*m as f64),
            AtomValue::Hz(h) => Some(*h),
            AtomValue::Volts(v) => Some(*v),
            AtomValue::Note { letter, accidental, octave } => {
                // Convert note to MIDI number
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
                    Some('#') | Some('s') => 1,
                    Some('b') | Some('f') => -1,
                    _ => 0,
                };

                let oct = octave.unwrap_or(4);
                Some(((oct + 1) * 12 + base + acc_offset) as f64)
            }
            AtomValue::Identifier(_) => None,
            AtomValue::String(_) => None,
        }
    }

    /// Try to parse a string as an AtomValue.
    pub fn parse(s: &str) -> AtomValue {
        // Try parsing as number first
        if let Ok(n) = s.parse::<f64>() {
            return AtomValue::Number(n);
        }

        // Check for Hz suffix
        if s.ends_with("hz") || s.ends_with("Hz") {
            if let Ok(n) = s[..s.len()-2].parse::<f64>() {
                return AtomValue::Hz(n);
            }
        }

        // Check for voltage suffix
        if s.ends_with('v') || s.ends_with('V') {
            if let Ok(n) = s[..s.len()-1].parse::<f64>() {
                return AtomValue::Volts(n);
            }
        }

        // Check for MIDI prefix
        if s.starts_with('m') || s.starts_with('M') {
            if let Ok(n) = s[1..].parse::<i32>() {
                return AtomValue::Midi(n);
            }
        }

        // Try parsing as note
        if let Some(note) = parse_note(s) {
            return note;
        }

        // Default to identifier
        AtomValue::Identifier(s.to_string())
    }
}

fn parse_note(s: &str) -> Option<AtomValue> {
    let chars: Vec<char> = s.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let letter = chars[0].to_ascii_lowercase();
    if !('a'..='g').contains(&letter) {
        return None;
    }

    // A single letter like "c" might be a note, but multi-char without
    // accidental or octave is likely an identifier
    if chars.len() == 1 {
        // Single letter could be a note without octave
        return Some(AtomValue::Note {
            letter,
            accidental: None,
            octave: None,
        });
    }

    let mut idx = 1;
    let mut accidental = None;

    // Check for single accidental
    if idx < chars.len() {
        match chars[idx] {
            '#' | 's' => {
                accidental = Some('#');
                idx += 1;
            }
            'b' | 'f' => {
                // 'b' or 'f' as accidental only if followed by digit, or at end
                // and not part of a word like "bd" (bass drum)
                if idx + 1 >= chars.len() {
                    // At end - this is ambiguous, treat as accidental for single letter note
                    accidental = Some('b');
                    idx += 1;
                } else if chars[idx + 1].is_ascii_digit() || chars[idx + 1] == '-' {
                    accidental = Some('b');
                    idx += 1;
                } else {
                    // Followed by non-digit like "bd" - this is an identifier
                    return None;
                }
            }
            c if c.is_ascii_digit() || c == '-' => {
                // Octave follows directly
            }
            _ => {
                // Invalid note format (e.g., "bd", "sn")
                return None;
            }
        }
    }

    // Parse octave
    let octave = if idx < chars.len() {
        let octave_str: String = chars[idx..].iter().collect();
        octave_str.parse::<i32>().ok()
    } else {
        None
    };

    Some(AtomValue::Note {
        letter,
        accidental,
        octave,
    })
}

/// A single operator call: $ name.variant(argument)
#[derive(Clone, Debug, PartialEq)]
pub struct OperatorCall {
    /// Operator name (e.g., "fast", "add", "scale").
    pub name: String,

    /// Optional variant (e.g., "squeeze", "in", "out").
    pub variant: Option<String>,

    /// Operator argument (can be a pattern).
    pub argument: Option<Box<MiniAST>>,

    /// Source span of the entire operator call.
    pub span: SourceSpan,
}

impl OperatorCall {
    pub fn new(name: String, variant: Option<String>, argument: Option<MiniAST>, start: usize, end: usize) -> Self {
        Self {
            name,
            variant,
            argument: argument.map(Box::new),
            span: SourceSpan::new(start, end),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atom_parse_number() {
        assert_eq!(AtomValue::parse("42"), AtomValue::Number(42.0));
        assert_eq!(AtomValue::parse("3.14"), AtomValue::Number(3.14));
        assert_eq!(AtomValue::parse("-1.5"), AtomValue::Number(-1.5));
    }

    #[test]
    fn test_atom_parse_hz() {
        assert_eq!(AtomValue::parse("440hz"), AtomValue::Hz(440.0));
        assert_eq!(AtomValue::parse("880Hz"), AtomValue::Hz(880.0));
    }

    #[test]
    fn test_atom_parse_volts() {
        assert_eq!(AtomValue::parse("5v"), AtomValue::Volts(5.0));
        assert_eq!(AtomValue::parse("1.5V"), AtomValue::Volts(1.5));
    }

    #[test]
    fn test_atom_parse_midi() {
        assert_eq!(AtomValue::parse("m60"), AtomValue::Midi(60));
        assert_eq!(AtomValue::parse("M127"), AtomValue::Midi(127));
    }

    #[test]
    fn test_atom_parse_note() {
        assert_eq!(
            AtomValue::parse("c4"),
            AtomValue::Note {
                letter: 'c',
                accidental: None,
                octave: Some(4)
            }
        );
        assert_eq!(
            AtomValue::parse("a#3"),
            AtomValue::Note {
                letter: 'a',
                accidental: Some('#'),
                octave: Some(3)
            }
        );
        assert_eq!(
            AtomValue::parse("bb5"),
            AtomValue::Note {
                letter: 'b',
                accidental: Some('b'),
                octave: Some(5)
            }
        );
    }

    #[test]
    fn test_note_to_f64() {
        let c4 = AtomValue::Note {
            letter: 'c',
            accidental: None,
            octave: Some(4),
        };
        assert_eq!(c4.to_f64(), Some(60.0)); // Middle C

        let a4 = AtomValue::Note {
            letter: 'a',
            accidental: None,
            octave: Some(4),
        };
        assert_eq!(a4.to_f64(), Some(69.0)); // A440
    }

    #[test]
    fn test_atom_parse_identifier() {
        assert_eq!(
            AtomValue::parse("maj"),
            AtomValue::Identifier("maj".to_string())
        );
        assert_eq!(
            AtomValue::parse("bd"),
            AtomValue::Identifier("bd".to_string())
        );
    }
}
