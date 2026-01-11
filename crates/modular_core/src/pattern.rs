use std::sync::Weak;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use pest::Parser;
use pest::iterators::Pair;

use crate::types::{Connect, Signal};

#[derive(pest_derive::Parser)]
#[grammar = "pattern.pest"]
struct PatternDslParser;

/// Main AST node enum representing all possible elements in the Musical DSL
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema)]
pub enum ASTNode {
    Leaf {
        value: Value,
        idx: usize,
        span: (usize, usize),
    },
    FastSubsequence {
        elements: Vec<ASTNode>,
    },
    SlowSubsequence {
        elements: Vec<ASTNode>,
    },
    RandomChoice {
        choices: Vec<ASTNode>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct PatternParseError {
    pub message: String,
}

impl std::fmt::Display for PatternParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for PatternParseError {}

/// Root pattern node containing all top-level elements
#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize, JsonSchema)]
pub struct PatternProgram {
    pub elements: Vec<ASTNode>,
    /// Optional patternable scale modifier for runtime scale selection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale_pattern: Option<ScalePatternProgram>,
    /// Optional add modifier with type-specific addition
    #[serde(skip_serializing_if = "Option::is_none")]
    pub add_pattern: Option<AddPatternProgram>,
}

impl PatternProgram {
    pub fn new(elements: Vec<ASTNode>) -> Self {
        Self {
            elements,
            scale_pattern: None,
            add_pattern: None,
        }
    }

    pub fn with_scale_pattern(mut self, scale_pattern: ScalePatternProgram) -> Self {
        self.scale_pattern = Some(scale_pattern);
        self
    }

    pub fn with_add_pattern(mut self, add_pattern: AddPatternProgram) -> Self {
        self.add_pattern = Some(add_pattern);
        self
    }
}

impl Connect for PatternProgram {
    fn connect(&mut self, patch: &crate::Patch) {
        for element in &mut self.elements {
            Connect::connect(element, patch);
        }
    }
}

impl Connect for ASTNode {
    fn connect(&mut self, patch: &crate::Patch) {
        match self {
            ASTNode::Leaf {
                value: Value::ModuleRef { signal, .. },
                ..
            } => {
                Connect::connect(signal, patch);
            }
            ASTNode::Leaf { .. } => {
                // No connections needed for leaf nodes
            }
            ASTNode::FastSubsequence { elements } | ASTNode::SlowSubsequence { elements } => {
                for element in elements {
                    Connect::connect(element, patch);
                }
            }
            ASTNode::RandomChoice { choices } => {
                for choice in choices {
                    Connect::connect(choice, patch);
                }
            }
        }
    }
}

fn parse_ast(pair: Pair<Rule>, idx: &mut usize) -> Result<ASTNode, PatternParseError> {
    match pair.as_rule() {
        Rule::Element | Rule::NonRandomElement | Rule::Value => {
            let inner = pair.into_inner().next().ok_or_else(|| PatternParseError {
                message: "Parse error: empty node".to_string(),
            })?;
            parse_ast(inner, idx)
        }

        Rule::FastSubsequence => {
            let mut elements = Vec::new();
            for child in pair.into_inner() {
                if child.as_rule() == Rule::Element {
                    elements.push(parse_ast(child, idx)?);
                }
            }
            Ok(ASTNode::FastSubsequence { elements })
        }

        Rule::SlowSubsequence => {
            let mut elements = Vec::new();
            for child in pair.into_inner() {
                if child.as_rule() == Rule::Element {
                    elements.push(parse_ast(child, idx)?);
                }
            }
            Ok(ASTNode::SlowSubsequence { elements })
        }

        Rule::RandomChoice => {
            let mut choices = Vec::new();
            for child in pair.into_inner() {
                if child.as_rule() == Rule::NonRandomElement {
                    choices.push(parse_ast(child, idx)?);
                }
            }
            Ok(ASTNode::RandomChoice { choices })
        }

        Rule::Rest => {
            let i = *idx;
            *idx += 1;
            let span = pair.as_span();
            Ok(ASTNode::Leaf {
                value: Value::Rest,
                idx: i,
                span: (span.start(), span.end()),
            })
        }

        Rule::NumericLiteral => {
            let span = pair.as_span();
            let num_str = pair
                .into_inner()
                .next()
                .ok_or_else(|| PatternParseError {
                    message: "Parse error: missing numeric literal".to_string(),
                })?
                .as_str();
            let value = num_str.parse::<f64>().unwrap_or(0.0);

            let i = *idx;
            *idx += 1;
            // Bare numbers need context to resolve - will be MIDI or scale interval
            Ok(ASTNode::Leaf {
                value: Value::UnresolvedNumeric(value),
                idx: i,
                span: (span.start(), span.end()),
            })
        }

        Rule::HzValue => {
            let span = pair.as_span();
            let mut inner = pair.into_inner();
            let num_str = inner
                .next()
                .ok_or_else(|| PatternParseError {
                    message: "Parse error: missing hz number".to_string(),
                })?
                .as_str();
            let suffix = inner
                .next()
                .ok_or_else(|| PatternParseError {
                    message: "Parse error: missing hz suffix".to_string(),
                })?
                .as_str();

            let mut value = num_str.parse::<f64>().unwrap_or(0.0);
            if suffix.eq_ignore_ascii_case("khz") {
                value *= 1000.0;
            }

            // Store as Hz - will be converted to V/Oct at runtime
            let i = *idx;
            *idx += 1;
            Ok(ASTNode::Leaf {
                value: Value::Pitch(PitchValue::Hz(value.max(0.0))),
                idx: i,
                span: (span.start(), span.end()),
            })
        }

        Rule::NoteName => {
            let mut inner = pair.into_inner();
            let letter_pair = inner.next().ok_or_else(|| PatternParseError {
                message: "Parse error: missing note letter".to_string(),
            })?;
            let span_start = letter_pair.as_span().start();
            let mut span_end = letter_pair.as_span().end();
            let letter = letter_pair
                .as_str()
                .chars()
                .next()
                .ok_or_else(|| PatternParseError {
                    message: "Parse error: invalid note letter".to_string(),
                })?;

            let next = inner.next();

            let (accidental, octave) = match next {
                Some(p) if p.as_rule() == Rule::Accidental => {
                    span_end = p.as_span().end();
                    let acc = p.as_str().chars().next().ok_or_else(|| PatternParseError {
                        message: "Parse error: invalid accidental".to_string(),
                    })?;
                    let octave = inner
                        .next()
                        .map(|oct_p| {
                            span_end = oct_p.as_span().end();
                            oct_p.as_str().parse::<i32>().unwrap_or(3)
                        })
                        .unwrap_or(3);
                    (Some(acc), octave)
                }
                Some(p) if p.as_rule() == Rule::Octave => {
                    span_end = p.as_span().end();
                    let octave = p.as_str().parse::<i32>().unwrap_or(3);
                    (None, octave)
                }
                _ => (None, 3),
            };
            let voct = note_name_to_voct(letter, accidental, octave);
            let i = *idx;
            *idx += 1;
            Ok(ASTNode::Leaf {
                value: Value::Numeric(voct),
                idx: i,
                span: (span_start, span_end),
            })
        }

        Rule::VoltsValue => {
            let span = pair.as_span();
            let num_str = pair
                .into_inner()
                .next()
                .ok_or_else(|| PatternParseError {
                    message: "Parse error: missing volts number".to_string(),
                })?
                .as_str();
            let value = num_str.parse::<f64>().unwrap_or(0.0);

            let i = *idx;
            *idx += 1;
            Ok(ASTNode::Leaf {
                value: Value::Pitch(PitchValue::Volts(value)),
                idx: i,
                span: (span.start(), span.end()),
            })
        }

        Rule::ModuleRef => {
            let span = pair.as_span();
            let mut inner = pair.into_inner();
            let module_id = inner
                .next()
                .ok_or_else(|| PatternParseError {
                    message: "Parse error: missing module id".to_string(),
                })?
                .as_str()
                .to_string();

            let port_name = inner
                .next()
                .ok_or_else(|| PatternParseError {
                    message: "Parse error: missing port name".to_string(),
                })?
                .as_str()
                .to_string();

            let i = *idx;
            *idx += 1;
            Ok(ASTNode::Leaf {
                value: Value::ModuleRef {
                    signal: Signal::Cable {
                        module: module_id,
                        module_ptr: Weak::default(),
                        port: port_name,
                    },
                    sample_and_hold: false,
                },
                idx: i,
                span: (span.start(), span.end()),
            })
        }

        other => Err(PatternParseError {
            message: format!("Parse error: unexpected rule {other:?}"),
        }),
    }
}

fn hz_to_voct(frequency_hz: f64) -> f64 {
    // Matches src/dsl/factories.ts hz(): log2(f / 27.5)
    (frequency_hz / 27.5).log2()
}

/// A scale definition (root note + scale name) that can be used at runtime
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema)]
pub struct ScaleDefinition {
    pub root_letter: char,
    pub root_accidental: Option<char>,
    pub root_octave: i32,
    pub scale_name: String,
}

/// AST node for scale patterns (mirrors main pattern structure but only contains ScaleDefinitions)
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema)]
pub enum ScalePatternNode {
    Leaf {
        definition: ScaleDefinition,
        idx: usize,
        span: (usize, usize),
    },
    FastSubsequence {
        elements: Vec<ScalePatternNode>,
    },
    SlowSubsequence {
        elements: Vec<ScalePatternNode>,
    },
    RandomChoice {
        choices: Vec<ScalePatternNode>,
    },
}

/// Compiled scale pattern program for runtime scale selection
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema)]
pub struct ScalePatternProgram {
    pub elements: Vec<ScalePatternNode>,
}

/// Type of values in an add pattern (must be consistent)
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize, JsonSchema)]
pub enum AddPatternType {
    /// All bare numbers (no suffix) - added as MIDI notes or scale intervals
    BareNumber,
    /// All have hz suffix - added as frequencies
    Hz,
    /// All have v suffix - added as volts
    Volts,
}

/// Add pattern program with its determined value type
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, JsonSchema)]
pub struct AddPatternProgram {
    pub elements: Vec<ASTNode>,
    pub value_type: AddPatternType,
}

/// Convert a scale interval to V/Oct using the given scale modifier
fn scale_interval_to_voct(
    interval: f64,
    scale_mod: &ScaleDefinition,
) -> Result<f64, PatternParseError> {
    use rust_music_theory::note::{Note, Notes, Pitch};
    use rust_music_theory::scale::Scale;

    // Build pitch string
    let pitch_str = match scale_mod.root_accidental {
        Some(acc) => format!("{}{}", scale_mod.root_letter, acc),
        None => scale_mod.root_letter.to_string(),
    };

    let pitch = Pitch::from_str(&pitch_str).ok_or_else(|| PatternParseError {
        message: format!("Invalid pitch: {}", pitch_str),
    })?;

    let root_note = Note::new(pitch, scale_mod.root_octave as u8);

    // Build scale definition
    let scale_def = format!("{} {}", root_note.pitch, scale_mod.scale_name);
    let scale = Scale::from_regex(&scale_def).map_err(|_| PatternParseError {
        message: format!("Invalid scale definition: {}", scale_def),
    })?;

    let interval_idx = interval.floor() as i64;
    let cents = (interval - interval.floor()) * 100.0;

    // Handle intervals: 0-indexed, wrap around octaves
    let target_idx_total = interval_idx;

    let notes = scale.notes();
    let len = notes.len() as i64;
    if len == 0 {
        return Err(PatternParseError {
            message: "Scale has no notes".to_string(),
        });
    }

    let scale_root_octave = notes[0].octave as i64;

    // Handle both positive and negative intervals with proper wrapping
    let (octave_shift, note_idx) = if target_idx_total >= 0 {
        ((target_idx_total / len), (target_idx_total % len) as usize)
    } else {
        // For negative intervals, we need to wrap backwards
        let abs_idx = (-target_idx_total - 1) as i64;
        let octave_down = (abs_idx / len) + 1;
        let note_from_end = (abs_idx % len) as usize;
        (-octave_down, len as usize - 1 - note_from_end)
    };

    let base_note = &notes[note_idx];
    let relative_octave = (base_note.octave as i64) - scale_root_octave;
    let target_octave = (scale_mod.root_octave as i64) + relative_octave + octave_shift;

    let mut target_note = base_note.clone();
    target_note.octave = target_octave as u8;

    let pc_val = target_note.pitch.into_u8();

    let midi = (target_octave as f64 + 1.0) * 12.0 + (pc_val as f64);
    let midi_with_cents = midi + (cents / 100.0);

    // Convert to V/Oct (A0 = 0V, MIDI 21)
    let volts = (midi_with_cents - 21.0) / 12.0;
    Ok(volts)
}

/// Parse a ScaleDefinition from a Pair
fn parse_scale_definition(pair: Pair<Rule>) -> Result<ScaleDefinition, PatternParseError> {
    let mut inner = pair.into_inner();

    // Parse NoteName (root note)
    let note_pair = inner.next().ok_or_else(|| PatternParseError {
        message: "Parse error: missing root note in scale modifier".to_string(),
    })?;

    let mut note_inner = note_pair.into_inner();
    let letter_pair = note_inner.next().ok_or_else(|| PatternParseError {
        message: "Parse error: missing note letter in scale modifier".to_string(),
    })?;
    let letter = letter_pair
        .as_str()
        .chars()
        .next()
        .ok_or_else(|| PatternParseError {
            message: "Parse error: invalid note letter in scale modifier".to_string(),
        })?;

    let next = note_inner.next();

    let (accidental, octave) = match next {
        Some(p) if p.as_rule() == Rule::Accidental => {
            let acc = p.as_str().chars().next();
            let octave = note_inner
                .next()
                .map(|oct_p| oct_p.as_str().parse::<i32>().unwrap_or(3))
                .unwrap_or(3);
            (acc, octave)
        }
        Some(p) if p.as_rule() == Rule::Octave => {
            let octave = p.as_str().parse::<i32>().unwrap_or(3);
            (None, octave)
        }
        _ => (None, 3),
    };

    // Parse scale name
    let scale_name_pair = inner.next().ok_or_else(|| PatternParseError {
        message: "Parse error: missing scale name in scale modifier".to_string(),
    })?;
    let scale_name = scale_name_pair.as_str().to_string();

    Ok(ScaleDefinition {
        root_letter: letter,
        root_accidental: accidental,
        root_octave: octave,
        scale_name,
    })
}

/// Resolve all ScaleInterval values in the AST to Numeric values
fn resolve_scale_intervals(
    elements: &mut Vec<ASTNode>,
    scale_mod: &ScaleDefinition,
) -> Result<(), PatternParseError> {
    for element in elements {
        resolve_scale_intervals_in_node(element, scale_mod)?;
    }
    Ok(())
}

fn resolve_scale_intervals_in_node(
    node: &mut ASTNode,
    scale_mod: &ScaleDefinition,
) -> Result<(), PatternParseError> {
    match node {
        ASTNode::Leaf { value, .. } => {
            if let Value::Pitch(PitchValue::ScaleInterval(interval)) = value {
                let voct = scale_interval_to_voct(*interval, scale_mod)?;
                *value = Value::Numeric(voct);
            }
        }
        ASTNode::FastSubsequence { elements } | ASTNode::SlowSubsequence { elements } => {
            for element in elements {
                resolve_scale_intervals_in_node(element, scale_mod)?;
            }
        }
        ASTNode::RandomChoice { choices } => {
            for choice in choices {
                resolve_scale_intervals_in_node(choice, scale_mod)?;
            }
        }
    }
    Ok(())
}

/// Resolve UnresolvedNumeric values based on whether a scale modifier is present
fn resolve_unresolved_numerics(elements: &mut Vec<ASTNode>, has_scale: bool) {
    for element in elements {
        resolve_unresolved_numerics_in_node(element, has_scale);
    }
}

fn resolve_unresolved_numerics_in_node(node: &mut ASTNode, has_scale: bool) {
    match node {
        ASTNode::Leaf { value, .. } => {
            if let Value::UnresolvedNumeric(n) = value {
                *value = if has_scale {
                    // With scale modifier: bare numbers are scale intervals
                    Value::Pitch(PitchValue::ScaleInterval(*n))
                } else {
                    // Without scale modifier: bare numbers are MIDI notes
                    Value::Pitch(PitchValue::Midi(*n))
                };
            }
        }
        ASTNode::FastSubsequence { elements } | ASTNode::SlowSubsequence { elements } => {
            for element in elements {
                resolve_unresolved_numerics_in_node(element, has_scale);
            }
        }
        ASTNode::RandomChoice { choices } => {
            for choice in choices {
                resolve_unresolved_numerics_in_node(choice, has_scale);
            }
        }
    }
}

fn note_name_to_voct(letter: char, accidental: Option<char>, octave: i32) -> f64 {
    // Matches src/dsl/factories.ts note() implementation.
    let base = match letter.to_ascii_lowercase() {
        'c' => 0,
        'd' => 2,
        'e' => 4,
        'f' => 5,
        'g' => 7,
        'a' => 9,
        'b' => 11,
        _ => 0,
    };

    let mut semitone = base;
    if accidental == Some('#') {
        semitone += 1;
    } else if accidental == Some('b') {
        semitone -= 1;
    }

    let semitones_from_c4 = (octave - 4) * 12 + semitone;
    let frequency = 440.0 * 2.0_f64.powf((semitones_from_c4 as f64 - 9.0) / 12.0);
    hz_to_voct(frequency)
}

/// Parse a scale pattern node (recursive for nested patterns)
fn parse_scale_pattern_node(
    pair: Pair<Rule>,
    idx: &mut usize,
) -> Result<ScalePatternNode, PatternParseError> {
    match pair.as_rule() {
        Rule::ScaleDefinition => {
            let span = pair.as_span();
            let definition = parse_scale_definition(pair)?;
            let i = *idx;
            *idx += 1;
            Ok(ScalePatternNode::Leaf {
                definition,
                idx: i,
                span: (span.start(), span.end()),
            })
        }
        Rule::ScaleFastSubsequence => {
            let mut elements = Vec::new();
            for child in pair.into_inner() {
                if child.as_rule() == Rule::ScalePatternElement {
                    let inner = child.into_inner().next().ok_or_else(|| PatternParseError {
                        message: "Empty scale pattern element".to_string(),
                    })?;
                    elements.push(parse_scale_pattern_node(inner, idx)?);
                }
            }
            Ok(ScalePatternNode::FastSubsequence { elements })
        }
        Rule::ScaleSlowSubsequence => {
            let mut elements = Vec::new();
            for child in pair.into_inner() {
                if child.as_rule() == Rule::ScalePatternElement {
                    let inner = child.into_inner().next().ok_or_else(|| PatternParseError {
                        message: "Empty scale pattern element".to_string(),
                    })?;
                    elements.push(parse_scale_pattern_node(inner, idx)?);
                }
            }
            Ok(ScalePatternNode::SlowSubsequence { elements })
        }
        Rule::ScalePatternSequence => {
            let inner = pair.into_inner().next().ok_or_else(|| PatternParseError {
                message: "Empty scale pattern sequence".to_string(),
            })?;
            parse_scale_pattern_node(inner, idx)
        }
        Rule::ScaleRandomChoice => {
            let mut choices = Vec::new();
            for child in pair.into_inner() {
                choices.push(parse_scale_pattern_node(child, idx)?);
            }
            Ok(ScalePatternNode::RandomChoice { choices })
        }
        other => Err(PatternParseError {
            message: format!("Unexpected scale pattern rule: {:?}", other),
        }),
    }
}

/// Parse the ScalePattern rule into a ScalePatternProgram
fn parse_scale_pattern(pair: Pair<Rule>) -> Result<ScalePatternProgram, PatternParseError> {
    let inner = pair.into_inner().next().ok_or_else(|| PatternParseError {
        message: "Empty scale pattern".to_string(),
    })?;

    let mut idx = 0;
    let node = parse_scale_pattern_node(inner, &mut idx)?;
    Ok(ScalePatternProgram {
        elements: vec![node],
    })
}

/// Detect the type of values in an add pattern (must be consistent)
fn detect_add_pattern_type(elements: &[ASTNode]) -> Result<AddPatternType, PatternParseError> {
    let mut found_type: Option<AddPatternType> = None;

    for node in elements {
        detect_type_in_node(node, &mut found_type)?;
    }

    // If only module refs, default to Volts
    Ok(found_type.unwrap_or(AddPatternType::Volts))
}

fn detect_type_in_node(
    node: &ASTNode,
    found_type: &mut Option<AddPatternType>,
) -> Result<(), PatternParseError> {
    match node {
        ASTNode::Leaf { value, .. } => {
            let node_type = match value {
                Value::Pitch(PitchValue::Volts(_)) => Some(AddPatternType::Volts),
                Value::Pitch(PitchValue::Hz(_)) => Some(AddPatternType::Hz),
                Value::Pitch(PitchValue::Midi(_)) | Value::UnresolvedNumeric(_) => {
                    Some(AddPatternType::BareNumber)
                }
                Value::ModuleRef { .. } => None, // Module refs inherit type
                Value::Rest => None,             // Rests don't affect type
                Value::Numeric(_) => Some(AddPatternType::Volts), // Note names treated as volts in add context
                Value::Pitch(PitchValue::ScaleInterval(_)) => Some(AddPatternType::BareNumber),
            };

            if let Some(t) = node_type {
                match found_type {
                    None => *found_type = Some(t),
                    Some(existing) if *existing != t => {
                        return Err(PatternParseError {
                            message: format!(
                                "Add pattern has mixed types: found {:?} and {:?}. All values must be same type (bare numbers, Hz, or Volts).",
                                existing, t
                            ),
                        });
                    }
                    _ => {}
                }
            }
        }
        ASTNode::FastSubsequence { elements } | ASTNode::SlowSubsequence { elements } => {
            for e in elements {
                detect_type_in_node(e, found_type)?;
            }
        }
        ASTNode::RandomChoice { choices } => {
            for c in choices {
                detect_type_in_node(c, found_type)?;
            }
        }
    }
    Ok(())
}

/// Parse the Musical DSL pattern source into a full PatternProgram with modifiers.
///
/// This mirrors the existing Ohm grammar in `src/dsl/mini.ohm` and the conversions
/// done in the TS parser (`hz()`/`note()`/MIDI mapping).
///
/// Returns a PatternProgram containing:
/// - Main pattern elements
/// - Optional scale pattern (for runtime scale selection)
/// - Optional add pattern (with type-specific addition)
pub fn parse_pattern(source: &str) -> Result<PatternProgram, PatternParseError> {
    let mut pairs =
        PatternDslParser::parse(Rule::Program, source).map_err(|err| PatternParseError {
            message: format!("Parse error: {err}"),
        })?;

    let program = pairs.next().ok_or_else(|| PatternParseError {
        message: "Parse error: missing program".to_string(),
    })?;

    let mut elements = Vec::new();
    let mut scale_pattern: Option<ScalePatternProgram> = None;
    let mut add_elements: Vec<ASTNode> = Vec::new();
    let mut idx: usize = 0;
    let mut add_idx: usize = 0;

    for pair in program.into_inner() {
        match pair.as_rule() {
            Rule::Element => {
                elements.push(parse_ast(pair, &mut idx)?);
            }
            Rule::Modifier => {
                let modifier_inner = pair.into_inner().next();
                if let Some(mod_pair) = modifier_inner {
                    match mod_pair.as_rule() {
                        Rule::ScaleModifier => {
                            let scale_pattern_pair = mod_pair.into_inner().next();
                            if let Some(sp) = scale_pattern_pair {
                                scale_pattern = Some(parse_scale_pattern(sp)?);
                            }
                        }
                        Rule::AddModifier => {
                            for add_child in mod_pair.into_inner() {
                                if add_child.as_rule() == Rule::Element {
                                    add_elements.push(parse_ast(add_child, &mut add_idx)?);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    let has_scale = scale_pattern.is_some();

    // Resolve UnresolvedNumeric values based on scale presence
    resolve_unresolved_numerics(&mut elements, has_scale);
    // Add pattern elements should also be resolved based on main scale presence
    resolve_unresolved_numerics(&mut add_elements, has_scale);

    // Build add pattern if we have add elements
    let add_pattern = if !add_elements.is_empty() {
        let value_type = detect_add_pattern_type(&add_elements)?;
        Some(AddPatternProgram {
            elements: add_elements,
            value_type,
        })
    } else {
        None
    };

    // For simple (non-patternable) scales, resolve scale intervals at parse time
    // For patternable scales, leave as ScaleInterval for runtime resolution
    if has_scale {
        if let Some(ref sp) = scale_pattern {
            if let Some(simple_scale) = get_simple_scale(sp) {
                resolve_scale_intervals(&mut elements, &simple_scale)?;
                return Ok(PatternProgram {
                    elements,
                    scale_pattern: None, // Already resolved
                    add_pattern,
                });
            }
        }
    }

    Ok(PatternProgram {
        elements,
        scale_pattern,
        add_pattern,
    })
}

/// Extract a simple (non-patternable) scale from a ScalePatternProgram
fn get_simple_scale(sp: &ScalePatternProgram) -> Option<ScaleDefinition> {
    if sp.elements.len() == 1 {
        if let ScalePatternNode::Leaf { definition, .. } = &sp.elements[0] {
            return Some(definition.clone());
        }
    }
    None
}

/// Parse the Musical DSL pattern source into AST nodes (backward compatible).
///
/// For full support of modifiers, use `parse_pattern` instead.
pub fn parse_pattern_elements(source: &str) -> Result<Vec<ASTNode>, PatternParseError> {
    let program = parse_pattern(source)?;
    Ok(program.elements)
}

/// Typed pitch value that preserves semantic information for runtime operations
#[derive(Debug, Clone, PartialEq, JsonSchema, Deserialize, Serialize)]
pub enum PitchValue {
    /// Explicit volts (from Xv suffix)
    Volts(f64),
    /// Frequency in Hz (from Xhz suffix)
    Hz(f64),
    /// MIDI note number with optional cents as decimal (e.g., 60.5 = middle C + 50 cents)
    Midi(f64),
    /// Scale interval - will be resolved at runtime using current scale
    /// Integer part = scale degree (1-indexed), decimal = cents offset
    ScaleInterval(f64),
}

/// Represents the output value from the runner
#[derive(Debug, Clone, PartialEq, JsonSchema, Deserialize, Serialize)]
pub enum Value {
    /// Final resolved V/Oct value
    Numeric(f64),
    /// Typed pitch that preserves semantics for runtime operations
    Pitch(PitchValue),
    /// Rest (no output)
    Rest,
    /// Reference to another module's output
    ModuleRef {
        #[serde(skip)]
        signal: Signal,
        sample_and_hold: bool,
    },
    /// Temporary: bare number that needs context to resolve (MIDI or scale interval)
    UnresolvedNumeric(f64),
}

/// Compiled node with precomputed information for efficient lookup
#[derive(Debug, Clone, PartialEq, JsonSchema, Deserialize)]
pub enum CompiledNode {
    /// A leaf value
    Value(Value),
    /// Fast subsequence with child nodes
    Fast(Vec<CompiledNode>),
    /// Slow subsequence with child nodes, period, and path info
    Slow {
        nodes: Vec<CompiledNode>,
        period: usize,
    },
    /// Random choice between two nodes
    Random { choices: Vec<CompiledNode> },
}

/// Simple PCG-based random number generator for deterministic randomness
#[derive(Debug, Clone, Copy)]
pub struct Rng {
    state: u64,
    pub seed: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed, seed }
    }

    /// Generate next random number and return a value in [0, 1)
    pub fn next(&mut self) -> f64 {
        // PCG algorithm
        const MULTIPLIER: u64 = 6364136223846793005;
        const INCREMENT: u64 = 1442695040888963407;

        self.state = self.state.wrapping_mul(MULTIPLIER).wrapping_add(INCREMENT);
        let xorshifted = (((self.state >> 18) ^ self.state) >> 27) as u32;
        let rot = (self.state >> 59) as u32;
        let result = xorshifted.rotate_right(rot);

        result as f64 / u32::MAX as f64
    }
}

/// Hash multiple components together with proper mixing to decorrelate inputs
pub fn hash_components(seed: u64, time_bits: u64, choice_id: u64) -> u64 {
    // Use different mixing constants for each component to ensure decorrelation
    // These are large primes chosen to have good bit distribution
    const SEED_MIX: u64 = 0x517cc1b727220a95;
    const TIME_MIX: u64 = 0x9e3779b97f4a7c15;
    const CHOICE_MIX: u64 = 0x85ebca6b0b7e3a85;

    let mut hash = seed.wrapping_mul(SEED_MIX);
    hash ^= hash >> 32;

    hash = hash.wrapping_add(time_bits.wrapping_mul(TIME_MIX));
    hash ^= hash >> 31;

    hash = hash.wrapping_add(choice_id.wrapping_mul(CHOICE_MIX));
    hash ^= hash >> 30;

    // Final avalanche mixing
    hash = hash.wrapping_mul(0xbf58476d1ce4e5b9);
    hash ^= hash >> 32;

    hash
}

// ============ Pitch Conversion Functions ============

/// A0 frequency in Hz (reference for V/Oct)
const A0_HZ: f64 = 27.5;

/// Convert MIDI note number to V/Oct (A0 = 0V = MIDI 21)
fn midi_to_voct(midi: f64) -> f64 {
    (midi - 21.0) / 12.0
}

/// Convert V/Oct to MIDI note number
fn voct_to_midi(voct: f64) -> f64 {
    voct * 12.0 + 21.0
}

/// Convert V/Oct to frequency in Hz
fn voct_to_hz(voct: f64) -> f64 {
    A0_HZ * 2.0_f64.powf(voct)
}

/// Resolve a Value to V/Oct, optionally using a scale for ScaleInterval values
fn resolve_value_to_voct(value: &Value, scale: Option<&ScaleDefinition>) -> Option<f64> {
    match value {
        Value::Numeric(v) => Some(*v),
        Value::Pitch(pv) => match pv {
            PitchValue::Volts(v) => Some(*v),
            PitchValue::Hz(hz) => Some(hz_to_voct(*hz)),
            PitchValue::Midi(m) => Some(midi_to_voct(*m)),
            PitchValue::ScaleInterval(interval) => {
                if let Some(s) = scale {
                    scale_interval_to_voct(*interval, s).ok()
                } else {
                    // Without scale, treat as MIDI (shouldn't happen in well-formed patterns)
                    Some(midi_to_voct(*interval))
                }
            }
        },
        Value::Rest => None,
        Value::ModuleRef { .. } => None, // Module refs handled separately
        Value::UnresolvedNumeric(_) => None, // Should be resolved before runtime
    }
}

/// Extract numeric value from a Value for add operations
fn extract_add_value(
    value: &Value,
    add_type: AddPatternType,
    scale: Option<&ScaleDefinition>,
) -> Option<f64> {
    match value {
        Value::Numeric(v) => Some(*v),
        Value::Pitch(pv) => match pv {
            PitchValue::Volts(v) => Some(*v),
            PitchValue::Hz(hz) => Some(*hz),
            PitchValue::Midi(m) => Some(*m),
            PitchValue::ScaleInterval(i) => Some(*i),
        },
        Value::ModuleRef { .. } => None, // Would need to sample the module
        Value::Rest => None,
        Value::UnresolvedNumeric(n) => Some(*n),
    }
}

/// Apply add value to main value based on add type
pub fn apply_add(
    main_voct: f64,
    main_value: &Value,
    add_modifier: &Option<(f64, usize, AddPatternType)>,
    scale_modifier: &Option<(crate::pattern::ScaleDefinition, usize)>,
) -> f64 {
    match add_modifier {
        Some((value, _, add_type)) => {
            match add_type {
                AddPatternType::BareNumber => {
                    // Special case: if main is scale interval AND we have a scale,
                    // add intervals together before scale resolution
                    if let Value::Pitch(PitchValue::ScaleInterval(interval)) = main_value {
                        if let Some((s, _)) = scale_modifier {
                            let combined = interval + value;
                            return scale_interval_to_voct(combined, &s).unwrap_or(main_voct);
                        }
                    }
                    // Otherwise: convert main to MIDI, add, convert back to V/Oct
                    let main_midi = voct_to_midi(main_voct);
                    let result_midi = main_midi + value;
                    midi_to_voct(result_midi)
                }
                AddPatternType::Hz => {
                    // Convert main to Hz, add Hz, convert back to V/Oct
                    let main_hz = voct_to_hz(main_voct);
                    let result_hz = main_hz + value;
                    hz_to_voct(result_hz.max(1.0)) // Clamp to avoid log of 0
                }
                AddPatternType::Volts => {
                    // Add directly to V/Oct output
                    main_voct + value
                }
            }
        }
        None => {
            // No add modifier, but still need to resolve ScaleInterval if present
            if let Value::Pitch(PitchValue::ScaleInterval(interval)) = main_value {
                if let Some((s, _)) = scale_modifier {
                    return scale_interval_to_voct(*interval, s).unwrap_or(main_voct);
                }
            }
            main_voct
        }
    }
}

// ============ Scale Pattern Runtime ============

impl ScalePatternProgram {
    /// Run the scale pattern at a given time to get the current scale
    pub fn run(&self, time: f64, seed: u64) -> Option<(ScaleDefinition, usize)> {
        if self.elements.is_empty() {
            return None;
        }

        let loop_time = time.fract();
        let loop_index = time.floor() as usize;

        self.run_nodes(&self.elements, loop_time, 0.0, 1.0, loop_index, 0, seed, 0)
    }

    fn run_nodes(
        &self,
        nodes: &[ScalePatternNode],
        time: f64,
        start: f64,
        duration: f64,
        loop_index: usize,
        depth: usize,
        seed: u64,
        choice_id: u64,
    ) -> Option<(ScaleDefinition, usize)> {
        if nodes.is_empty() {
            return None;
        }

        let element_duration = duration / nodes.len() as f64;

        for (i, node) in nodes.iter().enumerate() {
            let element_start = start + i as f64 * element_duration;
            let element_end = element_start + element_duration;

            if time >= element_start && time < element_end {
                let node_choice_id = choice_id
                    .wrapping_mul(nodes.len() as u64)
                    .wrapping_add(i as u64);
                return self.run_node(node, time, loop_index, depth, seed, node_choice_id);
            }
        }

        None
    }

    fn run_node(
        &self,
        node: &ScalePatternNode,
        time: f64,
        loop_index: usize,
        depth: usize,
        seed: u64,
        choice_id: u64,
    ) -> Option<(ScaleDefinition, usize)> {
        match node {
            ScalePatternNode::Leaf {
                definition, idx, ..
            } => Some((definition.clone(), *idx)),
            ScalePatternNode::FastSubsequence { elements } => {
                self.run_nodes(elements, time, 0.0, 1.0, loop_index, depth, seed, choice_id)
            }
            ScalePatternNode::SlowSubsequence { elements } => {
                if elements.is_empty() {
                    return None;
                }
                let depth = depth + 1;
                let period = elements.len();
                let encounter_count = loop_index / depth;
                let index = encounter_count % period;

                let child_choice_id = choice_id
                    .wrapping_mul(period as u64)
                    .wrapping_add(index as u64);
                self.run_node(
                    &elements[index],
                    time,
                    loop_index,
                    depth,
                    seed,
                    child_choice_id,
                )
            }
            ScalePatternNode::RandomChoice { choices } => {
                if choices.is_empty() {
                    return None;
                }

                let absolute_time = loop_index as f64 + time;
                let time_bits = absolute_time.to_bits();
                let hash = hash_components(seed, time_bits, choice_id);

                let mut choice_rng = Rng::new(hash);
                let random_value = choice_rng.next();

                let index = (random_value * choices.len() as f64).floor() as usize;
                let index = index.min(choices.len() - 1);

                self.run_node(
                    &choices[index],
                    time,
                    loop_index,
                    depth,
                    hash,
                    choice_id.wrapping_add(1),
                )
            }
        }
    }
}

// ============ Add Pattern Runtime ============

impl AddPatternProgram {
    /// Run the add pattern at a given time to get the current add value
    pub fn run(&self, time: f64, seed: u64) -> Option<(f64, usize)> {
        if self.elements.is_empty() {
            return None;
        }

        let loop_time = time.fract();
        let loop_index = time.floor() as usize;

        self.run_nodes(&self.elements, loop_time, 0.0, 1.0, loop_index, 0, seed, 0)
    }

    fn run_nodes(
        &self,
        nodes: &[ASTNode],
        time: f64,
        start: f64,
        duration: f64,
        loop_index: usize,
        depth: usize,
        seed: u64,
        choice_id: u64,
    ) -> Option<(f64, usize)> {
        if nodes.is_empty() {
            return None;
        }

        let element_duration = duration / nodes.len() as f64;

        for (i, node) in nodes.iter().enumerate() {
            let element_start = start + i as f64 * element_duration;
            let element_end = element_start + element_duration;

            if time >= element_start && time < element_end {
                let node_choice_id = choice_id
                    .wrapping_mul(nodes.len() as u64)
                    .wrapping_add(i as u64);
                return self.run_node(node, time, loop_index, depth, seed, node_choice_id);
            }
        }

        None
    }

    fn run_node(
        &self,
        node: &ASTNode,
        time: f64,
        loop_index: usize,
        depth: usize,
        seed: u64,
        choice_id: u64,
    ) -> Option<(f64, usize)> {
        match node {
            ASTNode::Leaf { value, idx, .. } => {
                let num = extract_add_value(value, self.value_type, None)?;
                Some((num, *idx))
            }
            ASTNode::FastSubsequence { elements } => {
                self.run_nodes(elements, time, 0.0, 1.0, loop_index, depth, seed, choice_id)
            }
            ASTNode::SlowSubsequence { elements } => {
                if elements.is_empty() {
                    return None;
                }
                let depth = depth + 1;
                let period = elements.len();
                let encounter_count = loop_index / depth;
                let index = encounter_count % period;

                let child_choice_id = choice_id
                    .wrapping_mul(period as u64)
                    .wrapping_add(index as u64);
                self.run_node(
                    &elements[index],
                    time,
                    loop_index,
                    depth,
                    seed,
                    child_choice_id,
                )
            }
            ASTNode::RandomChoice { choices } => {
                if choices.is_empty() {
                    return None;
                }

                let absolute_time = loop_index as f64 + time;
                let time_bits = absolute_time.to_bits();
                let hash = hash_components(seed, time_bits, choice_id);

                let mut choice_rng = Rng::new(hash);
                let random_value = choice_rng.next();

                let index = (random_value * choices.len() as f64).floor() as usize;
                let index = index.min(choices.len() - 1);

                self.run_node(
                    &choices[index],
                    time,
                    loop_index,
                    depth,
                    hash,
                    choice_id.wrapping_add(1),
                )
            }
        }
    }
}

impl PatternProgram {
    /// Run the compiled pattern at a given time (stateless)
    pub fn run(&self, time: f64, seed: u64) -> Option<(Value, f64, f64, usize)> {
        let loop_time = time.fract();
        let loop_index = time.floor() as usize;

        self.run_nodes(&self.elements, loop_time, 0.0, 1.0, loop_index, 0, seed, 0)
    }

    fn run_nodes(
        &self,
        nodes: &[ASTNode],
        time: f64,
        start: f64,
        duration: f64,
        loop_index: usize,
        depth: usize,
        seed: u64,
        choice_id: u64,
    ) -> Option<(Value, f64, f64, usize)> {
        if nodes.is_empty() {
            return None;
        }

        let element_duration = duration / nodes.len() as f64;

        for (i, node) in nodes.iter().enumerate() {
            let element_start = start + i as f64 * element_duration;
            let element_end = element_start + element_duration;

            if time >= element_start && time < element_end {
                let node_choice_id = choice_id
                    .wrapping_mul(nodes.len() as u64)
                    .wrapping_add(i as u64);
                return self.run_node(
                    node,
                    element_start,
                    element_duration,
                    time,
                    loop_index,
                    depth,
                    seed,
                    node_choice_id,
                );
            }
        }

        None
    }

    fn run_node(
        &self,
        node: &ASTNode,
        start: f64,
        duration: f64,
        time: f64,
        loop_index: usize,
        depth: usize,
        seed: u64,
        choice_id: u64,
    ) -> Option<(Value, f64, f64, usize)> {
        match node {
            ASTNode::Leaf { value, idx, .. } => {
                Some((value.clone(), start + loop_index as f64, duration, *idx))
            }
            ASTNode::FastSubsequence { elements } => self.run_nodes(
                elements, time, start, duration, loop_index, depth, seed, choice_id,
            ),
            ASTNode::SlowSubsequence { elements, .. } => {
                let depth = depth + 1;
                let period = elements.len();

                let encounter_count = loop_index / depth;
                let index = encounter_count % period;

                let child_choice_id = choice_id
                    .wrapping_mul(period as u64)
                    .wrapping_add(index as u64);
                self.run_node(
                    &elements[index],
                    start,
                    duration,
                    time,
                    loop_index,
                    depth,
                    seed,
                    child_choice_id,
                )
            }
            ASTNode::RandomChoice { choices } => {
                if choices.is_empty() {
                    return None;
                }

                // Compute absolute time from relative_time and loop_index
                let absolute_time = loop_index as f64 + time;

                // Hash all components together with proper mixing for decorrelation
                let time_bits = absolute_time.to_bits();
                let hash = hash_components(seed, time_bits, choice_id);

                let mut choice_rng = Rng::new(hash);
                let random_value = choice_rng.next();

                // Map random value to choice index
                let index = (random_value * choices.len() as f64).floor() as usize;
                let index = index.min(choices.len() - 1); // Clamp to valid range

                self.run_node(
                    &choices[index],
                    start,
                    duration,
                    time,
                    loop_index,
                    depth,
                    hash,
                    choice_id.wrapping_add(1),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn normalize_node_spans(node: ASTNode) -> ASTNode {
        match node {
            ASTNode::Leaf { value, idx, .. } => ASTNode::Leaf {
                value,
                idx,
                span: (0, 0),
            },
            ASTNode::FastSubsequence { elements } => ASTNode::FastSubsequence {
                elements: normalize_nodes_spans(elements),
            },
            ASTNode::SlowSubsequence { elements } => ASTNode::SlowSubsequence {
                elements: normalize_nodes_spans(elements),
            },
            ASTNode::RandomChoice { choices } => ASTNode::RandomChoice {
                choices: normalize_nodes_spans(choices),
            },
        }
    }

    fn normalize_nodes_spans(nodes: Vec<ASTNode>) -> Vec<ASTNode> {
        nodes
            .into_iter()
            .map(normalize_node_spans)
            .collect::<Vec<_>>()
    }

    fn leaf(value: Value, idx: usize) -> ASTNode {
        ASTNode::Leaf {
            value,
            idx,
            span: (0, 0),
        }
    }

    #[test]
    fn test_parse_pattern_elements_basic() {
        // Bare numbers without scale modifier become MIDI notes
        let ast = normalize_nodes_spans(parse_pattern_elements("1 2 3").unwrap());
        assert_eq!(ast, vec![midi(1.0, 0), midi(2.0, 1), midi(3.0, 2)]);
    }

    #[test]
    fn test_parse_module_ref() {
        let ast = normalize_nodes_spans(parse_pattern_elements("module(foo:bar)").unwrap());
        assert_eq!(
            ast,
            vec![leaf(
                Value::ModuleRef {
                    signal: Signal::Cable {
                        module: "foo".to_string(),
                        module_ptr: Weak::default(),
                        port: "bar".to_string()
                    },
                    sample_and_hold: false,
                },
                0
            )]
        );
    }

    #[test]
    fn test_parse_pattern_elements_subsequences_and_random() {
        // Bare numbers without scale modifier become MIDI notes
        let ast = normalize_nodes_spans(parse_pattern_elements("[1 2] <3 4> 5|6|7 ~").unwrap());
        assert_eq!(
            ast,
            vec![
                ASTNode::FastSubsequence {
                    elements: vec![midi(1.0, 0), midi(2.0, 1),]
                },
                ASTNode::SlowSubsequence {
                    elements: vec![midi(3.0, 2), midi(4.0, 3),]
                },
                ASTNode::RandomChoice {
                    choices: vec![midi(5.0, 4), midi(6.0, 5), midi(7.0, 6),]
                },
                leaf(Value::Rest, 7),
            ]
        );
    }

    #[test]
    fn test_pattern_fast_in_slow() {
        // Pattern under test:
        //   <c4 g4> <g4 [e4 d4]>
        // Semantics:
        // - Top level has 2 elements (each takes half the loop).
        // - Each `<...>` is a slow subsequence: it advances once per loop.
        // - `[e4 d4]` is a fast subsequence: it subdivides time within its slot.

        let source = "<c4 g4> <g4 [e4 d4]>";
        let ast = normalize_nodes_spans(parse_pattern_elements(source).unwrap());

        let c4 = note_name_to_voct('c', None, 4);
        let d4 = note_name_to_voct('d', None, 4);
        let e4 = note_name_to_voct('e', None, 4);
        let g4 = note_name_to_voct('g', None, 4);

        // 1) Parse structure is exactly what we expect.
        assert_eq!(
            ast,
            vec![
                ASTNode::SlowSubsequence {
                    elements: vec![num(c4, 0), num(g4, 1)],
                },
                ASTNode::SlowSubsequence {
                    elements: vec![
                        num(g4, 2),
                        ASTNode::FastSubsequence {
                            elements: vec![num(e4, 3), num(d4, 4)],
                        },
                    ],
                },
            ]
        );

        let pattern = PatternProgram::new(ast);
        // let compiled = CompiledPattern::compile(&pattern);

        // 2) Runtime behavior: verify each part occurs at the right time.
        // Loop 0: first half => c4, second half => g4
        assert_eq!(
            pattern.run(0.10, 0),
            Some((Value::Numeric(c4), 0.0, 0.5, 0))
        );
        assert_eq!(
            pattern.run(0.60, 0),
            Some((Value::Numeric(g4), 0.5, 0.5, 2))
        );

        // Loop 1: first half => g4 (slow advances), second half => [e4 d4]
        // Within the second half, the fast subsequence splits time again.
        assert_eq!(
            pattern.run(1.10, 0),
            Some((Value::Numeric(g4), 1.0, 0.5, 1))
        );
        assert_eq!(
            pattern.run(1.60, 0),
            Some((Value::Numeric(e4), 1.5, 0.25, 3))
        );
        assert_eq!(
            pattern.run(1.90, 0),
            Some((Value::Numeric(d4), 1.75, 0.25, 4))
        );

        // Loop 2: back to loop-0 selection for both slow subsequences
        assert_eq!(
            pattern.run(2.10, 0),
            Some((Value::Numeric(c4), 2.0, 0.5, 0))
        );
        assert_eq!(
            pattern.run(2.60, 0),
            Some((Value::Numeric(g4), 2.5, 0.5, 2))
        );
    }

    fn num(value: f64, idx: usize) -> ASTNode {
        ASTNode::Leaf {
            value: Value::Numeric(value),
            idx,
            span: (0, 0),
        }
    }

    fn midi(value: f64, idx: usize) -> ASTNode {
        ASTNode::Leaf {
            value: Value::Pitch(PitchValue::Midi(value)),
            idx,
            span: (0, 0),
        }
    }

    fn random(choices: Vec<ASTNode>) -> ASTNode {
        ASTNode::RandomChoice { choices }
    }

    #[test]
    fn test_basic_sequence() {
        let pattern = PatternProgram::new(vec![num(1.0, 0), num(2.0, 1), num(3.0, 2)]);

        assert_eq!(
            pattern.run(0.1, 0),
            Some((Value::Numeric(1.0), 0.0, 0.3333333333333333, 0))
        );
        assert_eq!(
            pattern.run(0.4, 0),
            Some((
                Value::Numeric(2.0),
                0.3333333333333333,
                0.3333333333333333,
                1
            ))
        );
        assert_eq!(
            pattern.run(0.7, 0),
            Some((
                Value::Numeric(3.0),
                0.6666666666666666,
                0.3333333333333333,
                2
            ))
        );
    }

    #[test]
    fn test_looping() {
        let pattern = PatternProgram::new(vec![num(1.0, 0), num(2.0, 1)]);

        assert_eq!(
            pattern.run(0.0, 0),
            Some((Value::Numeric(1.0), 0.0, 0.5, 0))
        );
        assert_eq!(
            pattern.run(1.0, 0),
            Some((Value::Numeric(1.0), 1.0, 0.5, 0))
        );
        assert_eq!(
            pattern.run(2.5, 0),
            Some((Value::Numeric(2.0), 2.5, 0.5, 1))
        );
    }

    #[test]
    fn test_fast_subsequence() {
        let pattern = PatternProgram::new(vec![
            num(1.0, 0),
            ASTNode::FastSubsequence {
                elements: vec![num(2.0, 1), num(3.0, 2)],
            },
        ]);

        assert_eq!(
            pattern.run(0.25, 0),
            Some((Value::Numeric(1.0), 0.0, 0.5, 0))
        );
        assert_eq!(
            pattern.run(0.55, 0),
            Some((Value::Numeric(2.0), 0.5, 0.25, 1))
        );
        assert_eq!(
            pattern.run(0.75, 0),
            Some((Value::Numeric(3.0), 0.75, 0.25, 2))
        );
    }

    #[test]
    fn test_slow_subsequence() {
        let pattern = PatternProgram::new(vec![ASTNode::SlowSubsequence {
            elements: vec![num(1.0, 0), num(2.0, 1), num(3.0, 2)],
        }]);

        assert_eq!(
            pattern.run(0.5, 0),
            Some((Value::Numeric(1.0), 0.0, 1.0, 0))
        );
        assert_eq!(
            pattern.run(1.5, 0),
            Some((Value::Numeric(2.0), 1.0, 1.0, 1))
        );
        assert_eq!(
            pattern.run(2.5, 0),
            Some((Value::Numeric(3.0), 2.0, 1.0, 2))
        );
        assert_eq!(
            pattern.run(3.5, 0),
            Some((Value::Numeric(1.0), 3.0, 1.0, 0))
        );
    }

    #[test]
    fn test_nested_slow_subsequence() {
        // <<1 2> <3 4>>
        let pattern = PatternProgram::new(vec![ASTNode::SlowSubsequence {
            elements: vec![
                ASTNode::SlowSubsequence {
                    elements: vec![num(1.0, 0), num(2.0, 1)],
                },
                ASTNode::SlowSubsequence {
                    elements: vec![num(3.0, 2), num(4.0, 3)],
                },
            ],
        }]);

        // Should return 1, 3, 2, 4, 1...
        assert_eq!(
            pattern.run(0.5, 0),
            Some((Value::Numeric(1.0), 0.0, 1.0, 0))
        );
        assert_eq!(
            pattern.run(1.5, 0),
            Some((Value::Numeric(3.0), 1.0, 1.0, 2))
        );
        assert_eq!(
            pattern.run(2.5, 0),
            Some((Value::Numeric(2.0), 2.0, 1.0, 1))
        );
        assert_eq!(
            pattern.run(3.5, 0),
            Some((Value::Numeric(4.0), 3.0, 1.0, 3))
        );
        assert_eq!(
            pattern.run(4.5, 0),
            Some((Value::Numeric(1.0), 4.0, 1.0, 0))
        );
    }

    #[test]
    fn test_random_choice() {
        let pattern =
            PatternProgram::new(vec![random(vec![num(1.0, 0), num(2.0, 1), num(3.0, 2)])]);
        let mut counts = HashMap::new();
        for i in 0..10000 {
            let time = i as f64;
            if let Some((Value::Numeric(val), _, _, _)) = pattern.run(time, 0) {
                *counts.entry(val as i32).or_insert(0) += 1;
            }
        }

        // All three values should appear roughly equally
        let count_1 = *counts.get(&1).unwrap_or(&0);
        let count_2 = *counts.get(&2).unwrap_or(&0);
        let count_3 = *counts.get(&3).unwrap_or(&0);
        assert!(count_1 > 3000);
        assert!(count_2 > 3000);
        assert!(count_3 > 3000);
    }

    #[test]
    fn test_random_with_slow_subsequence() {
        let pattern = PatternProgram::new(vec![ASTNode::SlowSubsequence {
            elements: vec![random(vec![num(1.0, 0), num(2.0, 1)]), num(3.0, 2)],
        }]);

        let mut counts = HashMap::new();
        for i in 0..10000 {
            let time = i as f64;
            if let Some((Value::Numeric(val), _, _, _)) = pattern.run(time, 0) {
                *counts.entry(val as i32).or_insert(0) += 1;
            }
        }

        let count_1 = *counts.get(&1).unwrap_or(&0);
        let count_2 = *counts.get(&2).unwrap_or(&0);
        let count_3 = *counts.get(&3).unwrap_or(&0);
        assert!(count_1 > 2300);
        assert!(count_2 > 2300);
        assert_eq!(count_3, 5000);
    }

    #[test]
    fn test_with_nested_random_slowsequence() {
        let pattern = PatternProgram::new(vec![ASTNode::SlowSubsequence {
            elements: vec![
                random(vec![
                    num(1.0, 0),
                    ASTNode::SlowSubsequence {
                        elements: vec![num(2.0, 1), num(3.0, 2)],
                    },
                ]),
                ASTNode::SlowSubsequence {
                    elements: vec![num(4.0, 3), num(5.0, 4)],
                },
            ],
        }]);

        let mut counts = HashMap::new();
        for i in 0..10000 {
            let time = i as f64;
            if let Some((Value::Numeric(val), _, _, _)) = pattern.run(time, 0) {
                *counts.entry(val as i32).or_insert(0) += 1;
            }
        }

        let count_1 = *counts.get(&1).unwrap_or(&0);
        let count_2 = *counts.get(&2).unwrap_or(&0);
        let count_3 = *counts.get(&3).unwrap_or(&0);
        let count_4 = *counts.get(&4).unwrap_or(&0);
        let count_5 = *counts.get(&5).unwrap_or(&0);
        assert!(count_1 > 2300);
        assert!(count_2 > 1150);
        assert!(count_3 > 1150);
        assert_eq!(count_4, 2500);
        assert_eq!(count_5, 2500);
    }

    #[test]
    fn test_stateless_multiple_calls() {
        let pattern = PatternProgram::new(vec![ASTNode::SlowSubsequence {
            elements: vec![num(1.0, 0), num(2.0, 1)],
        }]);

        // Call in any order - should be stateless
        assert_eq!(
            pattern.run(3.5, 0),
            Some((Value::Numeric(2.0), 3.0, 1.0, 1))
        );
        assert_eq!(
            pattern.run(0.5, 0),
            Some((Value::Numeric(1.0), 0.0, 1.0, 0))
        );
        assert_eq!(
            pattern.run(2.5, 0),
            Some((Value::Numeric(1.0), 2.0, 1.0, 0))
        );
        assert_eq!(
            pattern.run(1.5, 0),
            Some((Value::Numeric(2.0), 1.0, 1.0, 1))
        );
    }

    #[test]
    fn test_scale_interval_basic() {
        // 0 with scale(A0:Major) -> A0 (root) -> 0V
        let ast = parse_pattern_elements("0 $ scale(A0:Major)").unwrap();
        assert_eq!(ast.len(), 1);
        if let ASTNode::Leaf {
            value: Value::Numeric(v),
            ..
        } = &ast[0]
        {
            assert!(
                (*v - 0.0).abs() < 1e-6,
                "Expected ~0V for 0(A0:Major), got {}",
                v
            );
        } else {
            panic!("Expected numeric leaf");
        }
    }

    #[test]
    fn test_scale_interval_second_degree() {
        // 1 with scale(A0:Major) -> B0 (2nd in A Major) -> 2 semitones -> 2/12 V
        let ast = parse_pattern_elements("1 $ scale(A0:Major)").unwrap();
        assert_eq!(ast.len(), 1);
        if let ASTNode::Leaf {
            value: Value::Numeric(v),
            ..
        } = &ast[0]
        {
            let expected = 2.0 / 12.0;
            assert!(
                (*v - expected).abs() < 1e-6,
                "Expected ~{}V for 1(A0:Major), got {}",
                expected,
                v
            );
        } else {
            panic!("Expected numeric leaf");
        }
    }

    #[test]
    fn test_scale_interval_octave_wrap() {
        // 7 with scale(A0:Major) -> A1 (octave up) -> 1V
        let ast = parse_pattern_elements("7 $ scale(A0:Major)").unwrap();
        assert_eq!(ast.len(), 1);
        if let ASTNode::Leaf {
            value: Value::Numeric(v),
            ..
        } = &ast[0]
        {
            assert!(
                (*v - 1.0).abs() < 1e-6,
                "Expected ~1V for 7(A0:Major), got {}",
                v
            );
        } else {
            panic!("Expected numeric leaf");
        }
    }

    #[test]
    fn test_scale_interval_with_cents() {
        // 0.5 with scale(A0:Major) -> A0 + 50 cents -> 0.5/12 V
        let ast = parse_pattern_elements("0.5 $ scale(A0:Major)").unwrap();
        assert_eq!(ast.len(), 1);
        if let ASTNode::Leaf {
            value: Value::Numeric(v),
            ..
        } = &ast[0]
        {
            let expected = 0.5 / 12.0;
            assert!(
                (*v - expected).abs() < 1e-6,
                "Expected ~{}V for 0.5(A0:Major), got {}",
                expected,
                v
            );
        } else {
            panic!("Expected numeric leaf");
        }
    }

    #[test]
    fn test_scale_interval_sequence() {
        // Test a sequence of scale intervals using bare numbers with scale
        let ast = parse_pattern_elements("0 2 4 $ scale(C4:Major)").unwrap();
        assert_eq!(ast.len(), 3);

        // All should be numeric (resolved)
        for node in &ast {
            if let ASTNode::Leaf { value, .. } = node {
                assert!(matches!(value, Value::Numeric(_)));
            } else {
                panic!("Expected leaf nodes");
            }
        }
    }

    #[test]
    fn test_scale_interval_in_subsequence() {
        // Test scale intervals inside subsequences using bare numbers
        let ast = parse_pattern_elements("[0 1] <2 3> $ scale(A0:Major)").unwrap();
        assert_eq!(ast.len(), 2);

        // First is fast subsequence
        if let ASTNode::FastSubsequence { elements } = &ast[0] {
            assert_eq!(elements.len(), 2);
        } else {
            panic!("Expected fast subsequence");
        }

        // Second is slow subsequence
        if let ASTNode::SlowSubsequence { elements } = &ast[1] {
            assert_eq!(elements.len(), 2);
        } else {
            panic!("Expected slow subsequence");
        }
    }

    #[test]
    fn test_bare_numbers_without_scale_are_midi() {
        // Bare numbers without scale modifier become MIDI notes
        let ast = parse_pattern_elements("60 72 48").unwrap();
        assert_eq!(ast.len(), 3);

        // All should be Pitch(Midi)
        for node in &ast {
            if let ASTNode::Leaf { value, .. } = node {
                assert!(
                    matches!(value, Value::Pitch(PitchValue::Midi(_))),
                    "Expected Pitch(Midi)"
                );
            } else {
                panic!("Expected leaf nodes");
            }
        }
    }

    #[test]
    fn test_scale_interval_with_negative_octave() {
        // Test with negative octave in scale modifier
        let ast = parse_pattern_elements("0 $ scale(A-1:Major)").unwrap();
        assert_eq!(ast.len(), 1);
        if let ASTNode::Leaf {
            value: Value::Numeric(v),
            ..
        } = &ast[0]
        {
            // A-1 is one octave below A0, so -1V
            assert!(
                (*v - (-1.0)).abs() < 1e-6,
                "Expected ~-1V for 0(A-1:Major), got {}",
                v
            );
        } else {
            panic!("Expected numeric leaf");
        }
    }

    #[test]
    fn test_scale_interval_mixed_with_notes() {
        // Mix scale intervals (bare numbers) with regular notes and hz
        let ast = parse_pattern_elements("0 c4 1 440hz $ scale(A0:Major)").unwrap();
        assert_eq!(ast.len(), 4);

        // Check each value's type:
        // - 0 and 1 should be resolved to Numeric (scale intervals resolved at parse time)
        // - c4 should be Numeric (note name converted at parse time)
        // - 440hz should be Pitch(Hz) (converted to V/Oct at runtime)
        if let ASTNode::Leaf { value, .. } = &ast[0] {
            assert!(
                matches!(value, Value::Numeric(_)),
                "0 should be resolved to Numeric"
            );
        } else {
            panic!("Expected leaf node for 0");
        }

        if let ASTNode::Leaf { value, .. } = &ast[1] {
            assert!(matches!(value, Value::Numeric(_)), "c4 should be Numeric");
        } else {
            panic!("Expected leaf node for c4");
        }

        if let ASTNode::Leaf { value, .. } = &ast[2] {
            assert!(
                matches!(value, Value::Numeric(_)),
                "1 should be resolved to Numeric"
            );
        } else {
            panic!("Expected leaf node for 1");
        }

        if let ASTNode::Leaf { value, .. } = &ast[3] {
            assert!(
                matches!(value, Value::Pitch(PitchValue::Hz(_))),
                "440hz should be Pitch(Hz)"
            );
        } else {
            panic!("Expected leaf node for 440hz");
        }
    }

    #[test]
    fn test_negative_octave_in_note_name() {
        // Test that note names now support negative octaves
        let ast = parse_pattern_elements("A-1").unwrap();
        assert_eq!(ast.len(), 1);
        if let ASTNode::Leaf {
            value: Value::Numeric(v),
            ..
        } = &ast[0]
        {
            // A-1 is one octave below A0, so -1V
            assert!(
                (*v - (-1.0)).abs() < 1e-6,
                "Expected ~-1V for A-1, got {}",
                v
            );
        } else {
            panic!("Expected numeric leaf");
        }
    }

    #[test]
    fn test_spans_without_scale_modifier() {
        // Test that spans are correct for regular patterns
        let ast = parse_pattern_elements("c4 d4 e4").unwrap();
        assert_eq!(ast.len(), 3);

        // Check spans
        if let ASTNode::Leaf { span, .. } = &ast[0] {
            assert_eq!(*span, (0, 2), "c4 should be at span (0, 2)");
        }
        if let ASTNode::Leaf { span, .. } = &ast[1] {
            assert_eq!(*span, (3, 5), "d4 should be at span (3, 5)");
        }
        if let ASTNode::Leaf { span, .. } = &ast[2] {
            assert_eq!(*span, (6, 8), "e4 should be at span (6, 8)");
        }
    }

    #[test]
    fn test_spans_with_scale_modifier() {
        // Test that spans are correct when scale modifier is present
        let ast = parse_pattern_elements("1 2 3 $ scale(A0:Major)").unwrap();
        assert_eq!(ast.len(), 3);

        // Check spans - they should match the positions in the original string
        if let ASTNode::Leaf { span, .. } = &ast[0] {
            assert_eq!(*span, (0, 1), "1 should be at span (0, 1)");
        }
        if let ASTNode::Leaf { span, .. } = &ast[1] {
            assert_eq!(*span, (2, 3), "2 should be at span (2, 3)");
        }
        if let ASTNode::Leaf { span, .. } = &ast[2] {
            assert_eq!(*span, (4, 5), "3 should be at span (4, 5)");
        }
    }

    #[test]
    fn test_spans_mixed_elements_with_scale() {
        // Test spans with mixed elements and scale modifier
        let ast = parse_pattern_elements("1 c4 2 440hz $ scale(A0:Major)").unwrap();
        assert_eq!(ast.len(), 4);

        // Check spans
        if let ASTNode::Leaf { span, .. } = &ast[0] {
            assert_eq!(*span, (0, 1), "1 should be at span (0, 1)");
        }
        if let ASTNode::Leaf { span, .. } = &ast[1] {
            assert_eq!(*span, (2, 4), "c4 should be at span (2, 4)");
        }
        if let ASTNode::Leaf { span, .. } = &ast[2] {
            assert_eq!(*span, (5, 6), "2 should be at span (5, 6)");
        }
        if let ASTNode::Leaf { span, .. } = &ast[3] {
            assert_eq!(*span, (7, 12), "440hz should be at span (7, 12)");
        }
    }

    #[test]
    fn test_json_output_format() {
        // Test what the JSON output looks like for debugging
        let ast = parse_pattern_elements("c4 d4").unwrap();
        let json = serde_json::to_string_pretty(&ast).unwrap();
        println!("JSON output for 'c4 d4':\n{}", json);

        // The JSON should have Leaf nodes with span arrays
        assert!(json.contains("Leaf"));
        assert!(json.contains("span"));
        assert!(json.contains("Numeric"));
    }

    // ============ New feature tests ============

    #[test]
    fn test_volts_suffix() {
        // Explicit volts with 'v' suffix
        let ast = parse_pattern_elements("1v 2v 3.5v").unwrap();
        assert_eq!(ast.len(), 3);

        if let ASTNode::Leaf {
            value: Value::Pitch(PitchValue::Volts(v)),
            ..
        } = &ast[0]
        {
            assert!((*v - 1.0).abs() < 1e-6, "Expected 1V, got {}", v);
        } else {
            panic!("Expected Pitch(Volts) for 1v");
        }

        if let ASTNode::Leaf {
            value: Value::Pitch(PitchValue::Volts(v)),
            ..
        } = &ast[1]
        {
            assert!((*v - 2.0).abs() < 1e-6, "Expected 2V, got {}", v);
        } else {
            panic!("Expected Pitch(Volts) for 2v");
        }

        if let ASTNode::Leaf {
            value: Value::Pitch(PitchValue::Volts(v)),
            ..
        } = &ast[2]
        {
            assert!((*v - 3.5).abs() < 1e-6, "Expected 3.5V, got {}", v);
        } else {
            panic!("Expected Pitch(Volts) for 3.5v");
        }
    }

    #[test]
    fn test_bare_numbers_as_midi_without_scale() {
        // Without scale modifier, bare numbers are MIDI notes
        let ast = parse_pattern_elements("60 62 64").unwrap();
        assert_eq!(ast.len(), 3);

        // MIDI 60 = C4, MIDI 62 = D4, MIDI 64 = E4
        for (i, expected_midi) in [(0, 60.0), (1, 62.0), (2, 64.0)] {
            if let ASTNode::Leaf {
                value: Value::Pitch(PitchValue::Midi(m)),
                ..
            } = &ast[i]
            {
                assert!(
                    (*m - expected_midi).abs() < 1e-6,
                    "Expected MIDI {}, got {}",
                    expected_midi,
                    m
                );
            } else {
                panic!("Expected Pitch(Midi) for position {}", i);
            }
        }
    }

    #[test]
    fn test_bare_numbers_as_scale_intervals_with_scale() {
        // With scale modifier, bare numbers become scale intervals and get resolved
        let ast = parse_pattern_elements("0 2 4 $ scale(C4:Major)").unwrap();
        assert_eq!(ast.len(), 3);

        // 0, 2, 4 in C4 Major = C4, E4, G4
        // These should be resolved to Numeric values
        for node in &ast {
            if let ASTNode::Leaf { value, .. } = node {
                assert!(
                    matches!(value, Value::Numeric(_)),
                    "Expected Numeric after scale resolution"
                );
            } else {
                panic!("Expected leaf node");
            }
        }
    }

    #[test]
    fn test_hz_suffix_preserved() {
        // Hz values are preserved as PitchValue::Hz
        let ast = parse_pattern_elements("440hz 880hz 1khz").unwrap();
        assert_eq!(ast.len(), 3);

        if let ASTNode::Leaf {
            value: Value::Pitch(PitchValue::Hz(hz)),
            ..
        } = &ast[0]
        {
            assert!((*hz - 440.0).abs() < 1e-6, "Expected 440Hz, got {}", hz);
        } else {
            panic!("Expected Pitch(Hz) for 440hz");
        }

        if let ASTNode::Leaf {
            value: Value::Pitch(PitchValue::Hz(hz)),
            ..
        } = &ast[1]
        {
            assert!((*hz - 880.0).abs() < 1e-6, "Expected 880Hz, got {}", hz);
        } else {
            panic!("Expected Pitch(Hz) for 880hz");
        }

        if let ASTNode::Leaf {
            value: Value::Pitch(PitchValue::Hz(hz)),
            ..
        } = &ast[2]
        {
            assert!((*hz - 1000.0).abs() < 1e-6, "Expected 1000Hz, got {}", hz);
        } else {
            panic!("Expected Pitch(Hz) for 1khz");
        }
    }

    #[test]
    fn test_mixed_types_in_pattern() {
        // Mix of different value types
        let ast = parse_pattern_elements("c4 60 1v 440hz").unwrap();
        assert_eq!(ast.len(), 4);

        // c4 -> Numeric (note name converted at parse time)
        if let ASTNode::Leaf { value, .. } = &ast[0] {
            assert!(matches!(value, Value::Numeric(_)), "c4 should be Numeric");
        }

        // 60 -> Midi (bare number without scale)
        if let ASTNode::Leaf { value, .. } = &ast[1] {
            assert!(
                matches!(value, Value::Pitch(PitchValue::Midi(_))),
                "60 should be Pitch(Midi)"
            );
        }

        // 1v -> Volts
        if let ASTNode::Leaf { value, .. } = &ast[2] {
            assert!(
                matches!(value, Value::Pitch(PitchValue::Volts(_))),
                "1v should be Pitch(Volts)"
            );
        }

        // 440hz -> Hz
        if let ASTNode::Leaf { value, .. } = &ast[3] {
            assert!(
                matches!(value, Value::Pitch(PitchValue::Hz(_))),
                "440hz should be Pitch(Hz)"
            );
        }
    }

    // ============ Add Modifier Tests ============

    #[test]
    fn test_add_modifier_volts() {
        let program = parse_pattern("c4 $ add([0v 1v])").unwrap();
        assert!(program.add_pattern.is_some());
        let add = program.add_pattern.unwrap();
        assert_eq!(add.value_type, AddPatternType::Volts);
        assert_eq!(add.elements.len(), 1); // FastSubsequence
    }

    #[test]
    fn test_add_modifier_bare_numbers() {
        let program = parse_pattern("c4 $ add([0 12])").unwrap();
        assert!(program.add_pattern.is_some());
        let add = program.add_pattern.unwrap();
        assert_eq!(add.value_type, AddPatternType::BareNumber);
    }

    #[test]
    fn test_add_modifier_hz() {
        let program = parse_pattern("440hz $ add([0hz 100hz])").unwrap();
        assert!(program.add_pattern.is_some());
        let add = program.add_pattern.unwrap();
        assert_eq!(add.value_type, AddPatternType::Hz);
    }

    #[test]
    fn test_add_modifier_mixed_types_error() {
        // Mixed types should error
        let result = parse_pattern("c4 $ add([0v 1hz])");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.message.contains("mixed types"),
            "Error should mention mixed types: {}",
            err.message
        );
    }

    #[test]
    fn test_patternable_scale_fast() {
        let program = parse_pattern("1 2 3 $ scale([A0:Major C4:Minor])").unwrap();
        assert!(program.scale_pattern.is_some());
        let sp = program.scale_pattern.unwrap();
        // Should have a FastSubsequence with 2 scales
        if let ScalePatternNode::FastSubsequence { elements } = &sp.elements[0] {
            assert_eq!(elements.len(), 2);
        } else {
            panic!("Expected FastSubsequence");
        }
    }

    #[test]
    fn test_patternable_scale_slow() {
        let program = parse_pattern("1 2 3 $ scale(<A0:Major A0:Minor>)").unwrap();
        assert!(program.scale_pattern.is_some());
        let sp = program.scale_pattern.unwrap();
        // Should have a SlowSubsequence with 2 scales
        if let ScalePatternNode::SlowSubsequence { elements } = &sp.elements[0] {
            assert_eq!(elements.len(), 2);
        } else {
            panic!("Expected SlowSubsequence");
        }
    }

    #[test]
    fn test_midi_to_voct_conversion() {
        // MIDI 21 = A0 = 0V
        assert!((midi_to_voct(21.0) - 0.0).abs() < 1e-6);
        // MIDI 33 = A1 = 1V
        assert!((midi_to_voct(33.0) - 1.0).abs() < 1e-6);
        // MIDI 60 = C4 = 3.25V (C4 is 39 semitones above A0)
        assert!((midi_to_voct(60.0) - 39.0 / 12.0).abs() < 1e-6);
    }

    #[test]
    fn test_add_pattern_runtime_volts() {
        let program = parse_pattern("c4 $ add([0v 1v])").unwrap();
        let add = program.add_pattern.as_ref().unwrap();

        // First half of loop should return 0v
        let (val, _span) = add.run(0.25, 0).unwrap();
        assert!((val - 0.0).abs() < 1e-6);

        // Second half of loop should return 1v
        let (val, _span) = add.run(0.75, 0).unwrap();
        assert!((val - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_runtime_pitch_resolution() {
        // Test that Pitch values resolve correctly at runtime
        let program = parse_pattern("60 72").unwrap(); // MIDI notes

        // Run at time 0.25 (first note)
        let (value, _, _, _) = program.run(0.25, 0).unwrap();
        if let Value::Pitch(PitchValue::Midi(m)) = value {
            assert!((m - 60.0).abs() < 1e-6);
        } else {
            panic!("Expected Pitch(Midi), got {:?}", value);
        }
    }
}
