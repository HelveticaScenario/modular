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
}

impl PatternProgram {
    pub fn new(elements: Vec<ASTNode>) -> Self {
        Self { elements }
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
            Ok(ASTNode::Leaf {
                value: Value::Numeric(value),
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

            // TS version rejects non-positive frequencies; here we just map <=0 to 0.0.
            let voct = if value > 0.0 { hz_to_voct(value) } else { 0.0 };
            let i = *idx;
            *idx += 1;
            Ok(ASTNode::Leaf {
                value: Value::Numeric(voct),
                idx: i,
                span: (span.start(), span.end()),
            })
        }

        Rule::NoteName => {
            let span = pair.as_span();
            let mut inner = pair.into_inner();
            let letter_pair = inner.next().ok_or_else(|| PatternParseError {
                message: "Parse error: missing note letter".to_string(),
            })?;
            let letter = letter_pair
                .as_str()
                .chars()
                .next()
                .ok_or_else(|| PatternParseError {
                    message: "Parse error: invalid note letter".to_string(),
                })?;

            let next = inner.next().ok_or_else(|| PatternParseError {
                message: "Parse error: missing octave".to_string(),
            })?;

            let (accidental, octave_pair) = if next.as_rule() == Rule::Accidental {
                let acc = next
                    .as_str()
                    .chars()
                    .next()
                    .ok_or_else(|| PatternParseError {
                        message: "Parse error: invalid accidental".to_string(),
                    })?;
                let octave_pair = inner.next().ok_or_else(|| PatternParseError {
                    message: "Parse error: missing octave".to_string(),
                })?;
                (Some(acc), octave_pair)
            } else {
                (None, next)
            };

            let octave = octave_pair.as_str().parse::<i32>().unwrap_or(0);
            let voct = note_name_to_voct(letter, accidental, octave);
            let i = *idx;
            *idx += 1;
            Ok(ASTNode::Leaf {
                value: Value::Numeric(voct),
                idx: i,
                span: (span.start(), span.end()),
            })
        }

        Rule::MidiValue => {
            let span = pair.as_span();
            let midi_pair = pair.into_inner().next().ok_or_else(|| PatternParseError {
                message: "Parse error: missing midi number".to_string(),
            })?;
            let midi_note = midi_pair.as_str().parse::<i32>().unwrap_or(0);
            // Matches src/dsl/parser.ts: (midi - 69) / 12
            let voct = (midi_note as f64 - 69.0) / 12.0;
            let i = *idx;
            *idx += 1;
            Ok(ASTNode::Leaf {
                value: Value::Numeric(voct),
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

        Rule::ScaleInterval => {
            let span = pair.as_span();
            let num_str = pair
                .into_inner()
                .next()
                .ok_or_else(|| PatternParseError {
                    message: "Parse error: missing scale interval number".to_string(),
                })?
                .as_str();
            let interval = num_str.parse::<f64>().unwrap_or(0.0);

            let i = *idx;
            *idx += 1;
            Ok(ASTNode::Leaf {
                value: Value::ScaleInterval(interval),
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

/// Parsed scale modifier from the pattern
#[derive(Debug, Clone)]
struct ScaleModifier {
    root_letter: char,
    root_accidental: Option<char>,
    root_octave: i32,
    scale_name: String,
}

/// Convert a scale interval to V/Oct using the given scale modifier
fn scale_interval_to_voct(interval: f64, scale_mod: &ScaleModifier) -> Result<f64, PatternParseError> {
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

    // Handle intervals: 1-indexed, wrap around octaves
    let target_idx_total = interval_idx - 1;

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

/// Parse a ScaleModifier from a Pair
fn parse_scale_modifier(pair: Pair<Rule>) -> Result<ScaleModifier, PatternParseError> {
    let mut inner = pair.into_inner();

    // Parse NoteName (root note)
    let note_pair = inner.next().ok_or_else(|| PatternParseError {
        message: "Parse error: missing root note in scale modifier".to_string(),
    })?;

    let mut note_inner = note_pair.into_inner();
    let letter_pair = note_inner.next().ok_or_else(|| PatternParseError {
        message: "Parse error: missing note letter in scale modifier".to_string(),
    })?;
    let letter = letter_pair.as_str().chars().next().ok_or_else(|| PatternParseError {
        message: "Parse error: invalid note letter in scale modifier".to_string(),
    })?;

    let next = note_inner.next().ok_or_else(|| PatternParseError {
        message: "Parse error: missing octave in scale modifier".to_string(),
    })?;

    let (accidental, octave_pair) = if next.as_rule() == Rule::Accidental {
        let acc = next.as_str().chars().next();
        let octave_pair = note_inner.next().ok_or_else(|| PatternParseError {
            message: "Parse error: missing octave in scale modifier".to_string(),
        })?;
        (acc, octave_pair)
    } else {
        (None, next)
    };

    let octave = octave_pair.as_str().parse::<i32>().map_err(|_| PatternParseError {
        message: "Parse error: invalid octave in scale modifier".to_string(),
    })?;

    // Parse scale name
    let scale_name_pair = inner.next().ok_or_else(|| PatternParseError {
        message: "Parse error: missing scale name in scale modifier".to_string(),
    })?;
    let scale_name = scale_name_pair.as_str().to_string();

    Ok(ScaleModifier {
        root_letter: letter,
        root_accidental: accidental,
        root_octave: octave,
        scale_name,
    })
}

/// Resolve all ScaleInterval values in the AST to Numeric values
fn resolve_scale_intervals(
    elements: &mut Vec<ASTNode>,
    scale_mod: &ScaleModifier,
) -> Result<(), PatternParseError> {
    for element in elements {
        resolve_scale_intervals_in_node(element, scale_mod)?;
    }
    Ok(())
}

fn resolve_scale_intervals_in_node(
    node: &mut ASTNode,
    scale_mod: &ScaleModifier,
) -> Result<(), PatternParseError> {
    match node {
        ASTNode::Leaf { value, .. } => {
            if let Value::ScaleInterval(interval) = value {
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

/// Check if any ScaleInterval values remain unresolved in the AST
fn has_unresolved_scale_intervals(elements: &[ASTNode]) -> bool {
    elements.iter().any(|e| has_unresolved_in_node(e))
}

fn has_unresolved_in_node(node: &ASTNode) -> bool {
    match node {
        ASTNode::Leaf { value, .. } => matches!(value, Value::ScaleInterval(_)),
        ASTNode::FastSubsequence { elements } | ASTNode::SlowSubsequence { elements } => {
            elements.iter().any(|e| has_unresolved_in_node(e))
        }
        ASTNode::RandomChoice { choices } => choices.iter().any(|c| has_unresolved_in_node(c)),
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

/// Parse the Musical DSL pattern source into AST nodes.
///
/// This mirrors the existing Ohm grammar in `src/dsl/mini.ohm` and the conversions
/// done in the TS parser (`hz()`/`note()`/MIDI mapping).
///
/// If the pattern contains scale interval values (e.g., `1s`, `2s`), it must end with
/// a scale modifier (e.g., `$ scale(c4:major)`). The scale intervals will be resolved
/// to V/Oct values based on the specified scale.
pub fn parse_pattern_elements(source: &str) -> Result<Vec<ASTNode>, PatternParseError> {
    let mut pairs =
        PatternDslParser::parse(Rule::Program, source).map_err(|err| PatternParseError {
            message: format!("Parse error: {err}"),
        })?;

    let program = pairs.next().ok_or_else(|| PatternParseError {
        message: "Parse error: missing program".to_string(),
    })?;

    let mut elements = Vec::new();
    let mut scale_modifier: Option<ScaleModifier> = None;
    let mut idx: usize = 0;

    for pair in program.into_inner() {
        match pair.as_rule() {
            Rule::Element => {
                elements.push(parse_ast(pair, &mut idx)?);
            }
            Rule::ScaleModifier => {
                scale_modifier = Some(parse_scale_modifier(pair)?);
            }
            _ => {}
        }
    }

    // If we have scale intervals but no scale modifier, that's an error
    if has_unresolved_scale_intervals(&elements) {
        match &scale_modifier {
            Some(scale_mod) => {
                resolve_scale_intervals(&mut elements, scale_mod)?;
            }
            None => {
                return Err(PatternParseError {
                    message: "Pattern contains scale intervals (e.g., 1s, 2s) but no scale modifier. Add '$ scale(note:scale)' at the end.".to_string(),
                });
            }
        }
    }

    Ok(elements)
}

/// Represents the output value from the runner
#[derive(Debug, Clone, PartialEq, JsonSchema, Deserialize, Serialize)]
pub enum Value {
    Numeric(f64),
    Rest,
    ModuleRef {
        #[serde(skip)]
        signal: Signal,
        sample_and_hold: bool,
    },
    /// Unresolved scale interval - will be converted to Numeric after parsing
    /// when the scale modifier is processed
    ScaleInterval(f64),
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
        let ast = normalize_nodes_spans(parse_pattern_elements("1 2 3").unwrap());
        assert_eq!(
            ast,
            vec![num(1.0, 0), num(2.0, 1), num(3.0, 2)]
        );
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
        let ast = normalize_nodes_spans(parse_pattern_elements("[1 2] <3 4> 5|6|7 ~").unwrap());
        assert_eq!(
            ast,
            vec![
                ASTNode::FastSubsequence {
                    elements: vec![
                        num(1.0, 0),
                        num(2.0, 1),
                    ]
                },
                ASTNode::SlowSubsequence {
                    elements: vec![
                        num(3.0, 2),
                        num(4.0, 3),
                    ]
                },
                ASTNode::RandomChoice {
                    choices: vec![
                        num(5.0, 4),
                        num(6.0, 5),
                        num(7.0, 6),
                    ]
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

        let pattern = PatternProgram { elements: ast };
        // let compiled = CompiledPattern::compile(&pattern);

        // 2) Runtime behavior: verify each part occurs at the right time.
        // Loop 0: first half => c4, second half => g4
        assert_eq!(pattern.run(0.10, 0), Some((Value::Numeric(c4), 0.0, 0.5, 0)));
        assert_eq!(pattern.run(0.60, 0), Some((Value::Numeric(g4), 0.5, 0.5, 2)));

        // Loop 1: first half => g4 (slow advances), second half => [e4 d4]
        // Within the second half, the fast subsequence splits time again.
        assert_eq!(pattern.run(1.10, 0), Some((Value::Numeric(g4), 1.0, 0.5, 1)));
        assert_eq!(pattern.run(1.60, 0), Some((Value::Numeric(e4), 1.5, 0.25, 3)));
        assert_eq!(pattern.run(1.90, 0), Some((Value::Numeric(d4), 1.75, 0.25, 4)));

        // Loop 2: back to loop-0 selection for both slow subsequences
        assert_eq!(pattern.run(2.10, 0), Some((Value::Numeric(c4), 2.0, 0.5, 0)));
        assert_eq!(pattern.run(2.60, 0), Some((Value::Numeric(g4), 2.5, 0.5, 2)));
    }

    fn num(value: f64, idx: usize) -> ASTNode {
        ASTNode::Leaf {
            value: Value::Numeric(value),
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
            Some((Value::Numeric(2.0), 0.3333333333333333, 0.3333333333333333, 1))
        );
        assert_eq!(
            pattern.run(0.7, 0),
            Some((Value::Numeric(3.0), 0.6666666666666666, 0.3333333333333333, 2))
        );
    }

    #[test]
    fn test_looping() {
        let pattern = PatternProgram::new(vec![num(1.0, 0), num(2.0, 1)]);

        assert_eq!(pattern.run(0.0, 0), Some((Value::Numeric(1.0), 0.0, 0.5, 0)));
        assert_eq!(pattern.run(1.0, 0), Some((Value::Numeric(1.0), 1.0, 0.5, 0)));
        assert_eq!(pattern.run(2.5, 0), Some((Value::Numeric(2.0), 2.5, 0.5, 1)));
    }

    #[test]
    fn test_fast_subsequence() {
        let pattern = PatternProgram::new(vec![
            num(1.0, 0),
            ASTNode::FastSubsequence {
                elements: vec![num(2.0, 1), num(3.0, 2)],
            },
        ]);

        assert_eq!(pattern.run(0.25, 0), Some((Value::Numeric(1.0), 0.0, 0.5, 0)));
        assert_eq!(pattern.run(0.55, 0), Some((Value::Numeric(2.0), 0.5, 0.25, 1)));
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

        assert_eq!(pattern.run(0.5, 0), Some((Value::Numeric(1.0), 0.0, 1.0, 0)));
        assert_eq!(pattern.run(1.5, 0), Some((Value::Numeric(2.0), 1.0, 1.0, 1)));
        assert_eq!(pattern.run(2.5, 0), Some((Value::Numeric(3.0), 2.0, 1.0, 2)));
        assert_eq!(pattern.run(3.5, 0), Some((Value::Numeric(1.0), 3.0, 1.0, 0)));
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
        assert_eq!(pattern.run(0.5, 0), Some((Value::Numeric(1.0), 0.0, 1.0, 0)));
        assert_eq!(pattern.run(1.5, 0), Some((Value::Numeric(3.0), 1.0, 1.0, 2)));
        assert_eq!(pattern.run(2.5, 0), Some((Value::Numeric(2.0), 2.0, 1.0, 1)));
        assert_eq!(pattern.run(3.5, 0), Some((Value::Numeric(4.0), 3.0, 1.0, 3)));
        assert_eq!(pattern.run(4.5, 0), Some((Value::Numeric(1.0), 4.0, 1.0, 0)));
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
        assert_eq!(pattern.run(3.5, 0), Some((Value::Numeric(2.0), 3.0, 1.0, 1)));
        assert_eq!(pattern.run(0.5, 0), Some((Value::Numeric(1.0), 0.0, 1.0, 0)));
        assert_eq!(pattern.run(2.5, 0), Some((Value::Numeric(1.0), 2.0, 1.0, 0)));
        assert_eq!(pattern.run(1.5, 0), Some((Value::Numeric(2.0), 1.0, 1.0, 1)));
    }

    #[test]
    fn test_scale_interval_basic() {
        // 1s(A0:Major) -> A0 (root) -> 0V
        let ast = parse_pattern_elements("1s $ scale(A0:Major)").unwrap();
        assert_eq!(ast.len(), 1);
        if let ASTNode::Leaf { value: Value::Numeric(v), .. } = &ast[0] {
            assert!((*v - 0.0).abs() < 1e-6, "Expected ~0V for 1s(A0:Major), got {}", v);
        } else {
            panic!("Expected numeric leaf");
        }
    }

    #[test]
    fn test_scale_interval_second_degree() {
        // 2s(A0:Major) -> B0 (2nd in A Major) -> 2 semitones -> 2/12 V
        let ast = parse_pattern_elements("2s $ scale(A0:Major)").unwrap();
        assert_eq!(ast.len(), 1);
        if let ASTNode::Leaf { value: Value::Numeric(v), .. } = &ast[0] {
            let expected = 2.0 / 12.0;
            assert!((*v - expected).abs() < 1e-6, "Expected ~{}V for 2s(A0:Major), got {}", expected, v);
        } else {
            panic!("Expected numeric leaf");
        }
    }

    #[test]
    fn test_scale_interval_octave_wrap() {
        // 8s(A0:Major) -> A1 (octave up) -> 1V
        let ast = parse_pattern_elements("8s $ scale(A0:Major)").unwrap();
        assert_eq!(ast.len(), 1);
        if let ASTNode::Leaf { value: Value::Numeric(v), .. } = &ast[0] {
            assert!((*v - 1.0).abs() < 1e-6, "Expected ~1V for 8s(A0:Major), got {}", v);
        } else {
            panic!("Expected numeric leaf");
        }
    }

    #[test]
    fn test_scale_interval_with_cents() {
        // 1.5s(A0:Major) -> A0 + 50 cents -> 0.5/12 V
        let ast = parse_pattern_elements("1.5s $ scale(A0:Major)").unwrap();
        assert_eq!(ast.len(), 1);
        if let ASTNode::Leaf { value: Value::Numeric(v), .. } = &ast[0] {
            let expected = 0.5 / 12.0;
            assert!((*v - expected).abs() < 1e-6, "Expected ~{}V for 1.5s(A0:Major), got {}", expected, v);
        } else {
            panic!("Expected numeric leaf");
        }
    }

    #[test]
    fn test_scale_interval_sequence() {
        // Test a sequence of scale intervals
        let ast = parse_pattern_elements("1s 3s 5s $ scale(C4:Major)").unwrap();
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
        // Test scale intervals inside subsequences
        let ast = parse_pattern_elements("[1s 2s] <3s 4s> $ scale(A0:Major)").unwrap();
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
    fn test_scale_interval_missing_modifier_error() {
        // Scale intervals without modifier should error
        let result = parse_pattern_elements("1s 2s 3s");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("scale modifier"), "Error should mention scale modifier: {}", err.message);
    }

    #[test]
    fn test_scale_interval_with_negative_octave() {
        // Test with negative octave in scale modifier
        let ast = parse_pattern_elements("1s $ scale(A-1:Major)").unwrap();
        assert_eq!(ast.len(), 1);
        if let ASTNode::Leaf { value: Value::Numeric(v), .. } = &ast[0] {
            // A-1 is one octave below A0, so -1V
            assert!((*v - (-1.0)).abs() < 1e-6, "Expected ~-1V for 1s(A-1:Major), got {}", v);
        } else {
            panic!("Expected numeric leaf");
        }
    }

    #[test]
    fn test_scale_interval_mixed_with_notes() {
        // Mix scale intervals with regular notes and numbers
        let ast = parse_pattern_elements("1s c4 2s 440hz $ scale(A0:Major)").unwrap();
        assert_eq!(ast.len(), 4);

        // All should be resolved to numeric
        for node in &ast {
            if let ASTNode::Leaf { value, .. } = node {
                assert!(matches!(value, Value::Numeric(_)));
            } else {
                panic!("Expected leaf nodes");
            }
        }
    }

    #[test]
    fn test_negative_octave_in_note_name() {
        // Test that note names now support negative octaves
        let ast = parse_pattern_elements("A-1").unwrap();
        assert_eq!(ast.len(), 1);
        if let ASTNode::Leaf { value: Value::Numeric(v), .. } = &ast[0] {
            // A-1 is one octave below A0, so -1V
            assert!((*v - (-1.0)).abs() < 1e-6, "Expected ~-1V for A-1, got {}", v);
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
        let ast = parse_pattern_elements("1s 2s 3s $ scale(A0:Major)").unwrap();
        assert_eq!(ast.len(), 3);

        // Check spans - they should match the positions in the original string
        if let ASTNode::Leaf { span, .. } = &ast[0] {
            assert_eq!(*span, (0, 2), "1s should be at span (0, 2)");
        }
        if let ASTNode::Leaf { span, .. } = &ast[1] {
            assert_eq!(*span, (3, 5), "2s should be at span (3, 5)");
        }
        if let ASTNode::Leaf { span, .. } = &ast[2] {
            assert_eq!(*span, (6, 8), "3s should be at span (6, 8)");
        }
    }

    #[test]
    fn test_spans_mixed_elements_with_scale() {
        // Test spans with mixed elements and scale modifier
        let ast = parse_pattern_elements("1s c4 2s 440hz $ scale(A0:Major)").unwrap();
        assert_eq!(ast.len(), 4);

        // Check spans
        if let ASTNode::Leaf { span, .. } = &ast[0] {
            assert_eq!(*span, (0, 2), "1s should be at span (0, 2)");
        }
        if let ASTNode::Leaf { span, .. } = &ast[1] {
            assert_eq!(*span, (3, 5), "c4 should be at span (3, 5)");
        }
        if let ASTNode::Leaf { span, .. } = &ast[2] {
            assert_eq!(*span, (6, 8), "2s should be at span (6, 8)");
        }
        if let ASTNode::Leaf { span, .. } = &ast[3] {
            assert_eq!(*span, (9, 14), "440hz should be at span (9, 14)");
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
}
