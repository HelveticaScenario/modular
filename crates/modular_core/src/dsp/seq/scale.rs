//! Scale snapping and scale root handling for the Seq module.
//!
//! This module provides:
//! - `ScaleSnapper`: A lookup table for snapping MIDI notes to a scale
//! - `ScaleRoot`: Represents either a fixed root note or a pattern of roots
//! - `FixedRoot`: A fixed root note for scale construction

use rust_music_theory::note::{Note, Notes, Pitch};
use rust_music_theory::scale::Scale;

use crate::pattern_system::Pattern;

/// A fixed scale root (note letter + optional accidental).
#[derive(Clone, Debug, PartialEq)]
pub struct FixedRoot {
    pub letter: char,
    pub accidental: Option<char>,
}

impl FixedRoot {
    /// Create a new fixed root.
    pub fn new(letter: char, accidental: Option<char>) -> Self {
        Self { letter, accidental }
    }

    /// Parse from a string like "c", "c#", "bb".
    pub fn parse(s: &str) -> Option<Self> {
        let chars: Vec<char> = s.chars().collect();
        if chars.is_empty() {
            return None;
        }

        let letter = chars[0].to_ascii_lowercase();
        if !('a'..='g').contains(&letter) {
            return None;
        }

        let accidental = if chars.len() > 1 {
            match chars[1] {
                '#' | 's' => Some('#'),
                'b' | 'f' => Some('b'),
                _ => None,
            }
        } else {
            None
        };

        Some(Self { letter, accidental })
    }

    /// Get the pitch class (0-11, C=0).
    pub fn pitch_class(&self) -> i8 {
        let base = match self.letter {
            'c' => 0,
            'd' => 2,
            'e' => 4,
            'f' => 5,
            'g' => 7,
            'a' => 9,
            'b' => 11,
            _ => 0,
        };

        let acc = match self.accidental {
            Some('#') => 1,
            Some('b') => -1,
            _ => 0,
        };

        ((base + acc) % 12 + 12) as i8 % 12
    }

    /// Convert to rust_music_theory Pitch.
    pub fn to_pitch(&self) -> Option<Pitch> {
        let pitch_str = match self.accidental {
            Some(acc) => format!("{}{}", self.letter.to_ascii_uppercase(), acc),
            None => self.letter.to_ascii_uppercase().to_string(),
        };
        Pitch::from_str(&pitch_str)
    }
}

/// Scale root - either fixed or dynamic (pattern-based).
#[derive(Clone)]
pub enum ScaleRoot {
    /// A fixed root note.
    Fixed(FixedRoot),

    /// A pattern of root notes (queried at hap.whole_begin).
    Pattern(Pattern<FixedRoot>),
}

impl ScaleRoot {
    /// Get the root at a specific time.
    /// For Fixed, returns the fixed root.
    /// For Pattern, queries the pattern at the given time.
    pub fn root_at(&self, time: f64) -> Option<FixedRoot> {
        match self {
            ScaleRoot::Fixed(root) => Some(root.clone()),
            ScaleRoot::Pattern(pat) => {
                pat.query_at_first(time).map(|hap| hap.value.clone())
            }
        }
    }
}

impl std::fmt::Debug for ScaleRoot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScaleRoot::Fixed(root) => write!(f, "ScaleRoot::Fixed({:?})", root),
            ScaleRoot::Pattern(_) => write!(f, "ScaleRoot::Pattern(...)"),
        }
    }
}

/// A scale snapper with precomputed lookup table for fast MIDI→scale snapping.
///
/// The `snap_table` contains 13 entries (0-12 inclusive, where 12 wraps to next octave):
/// - Index 0 = offset for pitch class at root
/// - Index 1 = offset for pitch class 1 semitone above root
/// - ...up to index 12 = octave boundary handling
///
/// Each table entry is the signed offset to the nearest scale degree.
/// When equidistant, prefers the lower pitch.
#[derive(Clone, Debug)]
pub struct ScaleSnapper {
    /// Snap offsets for each chromatic pitch relative to root (0-12).
    /// Value is the signed semitone offset to snap to the nearest scale tone.
    snap_table: [i8; 13],

    /// Root offset in semitones (C=0, C#=1, ..., B=11).
    root_offset: i8,

    /// The scale type name (for dynamic root rebuilding).
    scale_name: String,
}

impl ScaleSnapper {
    /// Build a ScaleSnapper from a scale type name and root.
    ///
    /// # Arguments
    /// * `root` - The root note of the scale
    /// * `scale_name` - The scale type (e.g., "major", "minor", "dorian")
    ///
    /// # Returns
    /// `Some(ScaleSnapper)` if the scale is valid, `None` otherwise.
    pub fn new(root: &FixedRoot, scale_name: &str) -> Option<Self> {
        let pitch = root.to_pitch()?;
        let root_note = Note::new(pitch, 4); // Octave doesn't matter for interval calculation

        // Build scale definition string
        let scale_def = format!("{} {}", root_note.pitch, scale_name);
        let scale = Scale::from_regex(&scale_def).ok()?;

        let notes = scale.notes();
        if notes.is_empty() {
            return None;
        }

        // Build set of scale degrees (pitch classes relative to root)
        let root_pc = root.pitch_class();
        let mut scale_degrees: Vec<i8> = notes
            .iter()
            .map(|n| {
                let pc = n.pitch.into_u8() as i8;
                ((pc - root_pc) % 12 + 12) % 12
            })
            .collect();

        // Remove duplicates and sort
        scale_degrees.sort();
        scale_degrees.dedup();

        // Include octave (12) for boundary handling
        let mut degrees_with_octave = scale_degrees.clone();
        degrees_with_octave.push(12);

        // Also include -12 (previous octave) for downward snapping
        let mut degrees_extended: Vec<i8> = degrees_with_octave.clone();
        for &d in &scale_degrees {
            degrees_extended.push(d - 12);
        }
        degrees_extended.sort();

        // Build snap table: for each chromatic pitch (0-12), find nearest scale degree
        let mut snap_table = [0i8; 13];
        for chromatic in 0..=12 {
            let mut best_offset = 0i8;
            let mut best_dist = i8::MAX;

            for &degree in &degrees_extended {
                let offset = degree - chromatic;
                let dist = offset.abs();

                if dist < best_dist || (dist == best_dist && offset < 0) {
                    best_dist = dist;
                    best_offset = offset;
                }
            }

            snap_table[chromatic as usize] = best_offset;
        }

        // Root offset: semitones from A to root
        let root_offset = root.pitch_class();

        Some(Self {
            snap_table,
            root_offset,
            scale_name: scale_name.to_string(),
        })
    }

    /// Snap a MIDI note to the nearest scale degree.
    ///
    /// # Arguments
    /// * `midi` - The MIDI note number (can be fractional for cents)
    ///
    /// # Returns
    /// The snapped MIDI note number.
    pub fn snap_midi(&self, midi: f64) -> f64 {
        // Split into integer and fractional parts
        let midi_int = midi.floor() as i32;
        let cents = midi - midi_int as f64;

        // Convert MIDI to pitch class (C=0, C#=1, ..., B=11)
        // MIDI 60 = C4, so midi % 12 gives pitch class with C=0
        let midi_pc = ((midi_int % 12) + 12) % 12;

        // Convert to position relative to scale root
        // If root is C (pitch_class=0), then C maps to 0, D to 2, etc.
        // If root is D (pitch_class=2), then D maps to 0, E to 2, C to -2 (or 10)
        let pc_in_scale = ((midi_pc - self.root_offset as i32) % 12 + 12) % 12;

        // Look up snap offset
        let snap_offset = self.snap_table[pc_in_scale as usize];

        // Apply snap
        let snapped = midi_int + snap_offset as i32;

        // Add back cents (microtuning preserved)
        snapped as f64 + cents
    }

    /// Check if a MIDI note is in the scale.
    pub fn is_in_scale(&self, midi: f64) -> bool {
        let midi_int = midi.floor() as i32;
        let midi_pc = ((midi_int % 12) + 12) % 12;
        let pc_in_scale = ((midi_pc - self.root_offset as i32) % 12 + 12) % 12;
        self.snap_table[pc_in_scale as usize] == 0
    }

    /// Get the scale type name.
    pub fn scale_name(&self) -> &str {
        &self.scale_name
    }
}

/// Known scale type names for validation.
pub const KNOWN_SCALE_TYPES: &[&str] = &[
    "major",
    "minor",
    "ionian",
    "dorian",
    "phrygian",
    "lydian",
    "mixolydian",
    "aeolian",
    "locrian",
    "harmonic minor",
    "melodic minor",
    "pentatonic major",
    "pentatonic minor",
    "blues",
    "whole tone",
    "chromatic",
    "diminished",
    "augmented",
];

/// Validate that a scale type name is known.
pub fn validate_scale_type(name: &str) -> bool {
    let normalized = name.to_lowercase();
    KNOWN_SCALE_TYPES
        .iter()
        .any(|&known| known == normalized || known.replace(' ', "") == normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixed_root_parse() {
        let c = FixedRoot::parse("c").unwrap();
        assert_eq!(c.letter, 'c');
        assert_eq!(c.accidental, None);

        let cs = FixedRoot::parse("c#").unwrap();
        assert_eq!(cs.letter, 'c');
        assert_eq!(cs.accidental, Some('#'));

        let bb = FixedRoot::parse("bb").unwrap();
        assert_eq!(bb.letter, 'b');
        assert_eq!(bb.accidental, Some('b'));
    }

    #[test]
    fn test_fixed_root_pitch_class() {
        assert_eq!(FixedRoot::parse("c").unwrap().pitch_class(), 0);
        assert_eq!(FixedRoot::parse("c#").unwrap().pitch_class(), 1);
        assert_eq!(FixedRoot::parse("d").unwrap().pitch_class(), 2);
        assert_eq!(FixedRoot::parse("a").unwrap().pitch_class(), 9);
        assert_eq!(FixedRoot::parse("b").unwrap().pitch_class(), 11);
    }

    #[test]
    fn test_scale_snapper_c_major() {
        let root = FixedRoot::parse("c").unwrap();
        let snapper = ScaleSnapper::new(&root, "major").unwrap();

        // C major: C D E F G A B
        // C (60) should stay C
        assert_eq!(snapper.snap_midi(60.0), 60.0);

        // D (62) should stay D
        assert_eq!(snapper.snap_midi(62.0), 62.0);

        // C# (61) should snap to C (60) - prefer lower when equidistant
        assert_eq!(snapper.snap_midi(61.0), 60.0);

        // F# (66) should snap to F (65) or G (67)
        // F# is equidistant, should prefer lower (F)
        let snapped = snapper.snap_midi(66.0);
        assert!(snapped == 65.0 || snapped == 67.0);
    }

    #[test]
    fn test_scale_snapper_preserves_cents() {
        let root = FixedRoot::parse("c").unwrap();
        let snapper = ScaleSnapper::new(&root, "major").unwrap();

        // 60.5 (C + 50 cents) should stay around 60.5
        let snapped = snapper.snap_midi(60.5);
        assert!((snapped - 60.5).abs() < 1.0);
    }

    #[test]
    fn test_validate_scale_type() {
        assert!(validate_scale_type("major"));
        assert!(validate_scale_type("Minor"));
        assert!(validate_scale_type("dorian"));
        assert!(validate_scale_type("harmonic minor"));
        assert!(validate_scale_type("harmonicminor"));
        assert!(!validate_scale_type("unknown_scale"));
    }
}
