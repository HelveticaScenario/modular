use napi_derive::napi;
use schemars::JsonSchema;
use serde::Deserialize;

use pest::Parser;
use pest::iterators::Pair;

#[derive(pest_derive::Parser)]
#[grammar = "pattern.pest"]
struct PatternDslParser;

/// Main AST node enum representing all possible elements in the Musical DSL
#[derive(Debug, Clone, PartialEq, Deserialize, JsonSchema)]
pub enum ASTNode {
    Leaf { value: Value, idx: usize },
    FastSubsequence { elements: Vec<ASTNode> },
    SlowSubsequence { elements: Vec<ASTNode> },
    RandomChoice { choices: Vec<ASTNode> },
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
#[derive(Debug, Default, Clone, PartialEq, Deserialize, JsonSchema)]
pub struct PatternProgram {
    pub elements: Vec<ASTNode>,
}

impl PatternProgram {
    pub fn new(elements: Vec<ASTNode>) -> Self {
        Self { elements }
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
            Ok(ASTNode::Leaf {
                value: Value::Rest,
                idx: i,
            })
        }

        Rule::NumericLiteral => {
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
            })
        }

        Rule::HzValue => {
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
            })
        }

        Rule::NoteName => {
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
            })
        }

        Rule::MidiValue => {
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
pub fn parse_pattern_elements(source: &str) -> Result<Vec<ASTNode>, PatternParseError> {
    let mut pairs =
        PatternDslParser::parse(Rule::Program, source).map_err(|err| PatternParseError {
            message: format!("Parse error: {err}"),
        })?;

    let program = pairs.next().ok_or_else(|| PatternParseError {
        message: "Parse error: missing program".to_string(),
    })?;

    let mut elements = Vec::new();
    let mut idx: usize = 0;
    for pair in program.into_inner() {
        if pair.as_rule() == Rule::Element {
            elements.push(parse_ast(pair, &mut idx)?);
        }
    }

    Ok(elements)
}

/// Represents the output value from the runner
#[derive(Debug, Clone, PartialEq, JsonSchema, Deserialize)]
#[napi]
pub enum Value {
    Numeric(f64),
    Rest,
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
    pub fn run(&self, time: f64, seed: u64) -> Option<(Value, f64, f64)> {
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
    ) -> Option<(Value, f64, f64)> {
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
    ) -> Option<(Value, f64, f64)> {
        match node {
            ASTNode::Leaf { value, .. } => {
                Some((value.clone(), start + loop_index as f64, duration))
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

    #[test]
    fn test_parse_pattern_elements_basic() {
        let ast = parse_pattern_elements("1 2 3").unwrap();
        assert_eq!(
            ast,
            vec![
                ASTNode::Leaf {
                    value: Value::Numeric(1.0),
                    idx: 0
                },
                ASTNode::Leaf {
                    value: Value::Numeric(2.0),
                    idx: 1
                },
                ASTNode::Leaf {
                    value: Value::Numeric(3.0),
                    idx: 2
                },
            ]
        );
    }

    #[test]
    fn test_parse_pattern_elements_subsequences_and_random() {
        let ast = parse_pattern_elements("[1 2] <3 4> 5|6|7 ~").unwrap();
        assert_eq!(
            ast,
            vec![
                ASTNode::FastSubsequence {
                    elements: vec![
                        ASTNode::Leaf {
                            value: Value::Numeric(1.0),
                            idx: 0
                        },
                        ASTNode::Leaf {
                            value: Value::Numeric(2.0),
                            idx: 1
                        },
                    ]
                },
                ASTNode::SlowSubsequence {
                    elements: vec![
                        ASTNode::Leaf {
                            value: Value::Numeric(3.0),
                            idx: 2
                        },
                        ASTNode::Leaf {
                            value: Value::Numeric(4.0),
                            idx: 3
                        },
                    ]
                },
                ASTNode::RandomChoice {
                    choices: vec![
                        ASTNode::Leaf {
                            value: Value::Numeric(5.0),
                            idx: 4
                        },
                        ASTNode::Leaf {
                            value: Value::Numeric(6.0),
                            idx: 5
                        },
                        ASTNode::Leaf {
                            value: Value::Numeric(7.0),
                            idx: 6
                        },
                    ]
                },
                ASTNode::Leaf {
                    value: Value::Rest,
                    idx: 7
                },
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
        let ast = parse_pattern_elements(source).unwrap();

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
        assert_eq!(pattern.run(0.10, 0), Some((Value::Numeric(c4), 0.0, 0.5)));
        assert_eq!(pattern.run(0.60, 0), Some((Value::Numeric(g4), 0.5, 0.5)));

        // Loop 1: first half => g4 (slow advances), second half => [e4 d4]
        // Within the second half, the fast subsequence splits time again.
        assert_eq!(pattern.run(1.10, 0), Some((Value::Numeric(g4), 1.0, 0.5)));
        assert_eq!(pattern.run(1.60, 0), Some((Value::Numeric(e4), 1.5, 0.25)));
        assert_eq!(pattern.run(1.90, 0), Some((Value::Numeric(d4), 1.75, 0.25)));

        // Loop 2: back to loop-0 selection for both slow subsequences
        assert_eq!(pattern.run(2.10, 0), Some((Value::Numeric(c4), 2.0, 0.5)));
        assert_eq!(pattern.run(2.60, 0), Some((Value::Numeric(g4), 2.5, 0.5)));
    }

    fn num(value: f64, idx: usize) -> ASTNode {
        ASTNode::Leaf {
            value: Value::Numeric(value),
            idx,
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
            Some((Value::Numeric(1.0), 0.0, 0.3333333333333333))
        );
        assert_eq!(
            pattern.run(0.4, 0),
            Some((Value::Numeric(2.0), 0.3333333333333333, 0.3333333333333333))
        );
        assert_eq!(
            pattern.run(0.7, 0),
            Some((Value::Numeric(3.0), 0.6666666666666666, 0.3333333333333333))
        );
    }

    #[test]
    fn test_looping() {
        let pattern = PatternProgram::new(vec![num(1.0, 0), num(2.0, 1)]);

        assert_eq!(pattern.run(0.0, 0), Some((Value::Numeric(1.0), 0.0, 0.5)));
        assert_eq!(pattern.run(1.0, 0), Some((Value::Numeric(1.0), 1.0, 0.5)));
        assert_eq!(pattern.run(2.5, 0), Some((Value::Numeric(2.0), 2.5, 0.5)));
    }

    #[test]
    fn test_fast_subsequence() {
        let pattern = PatternProgram::new(vec![
            num(1.0, 0),
            ASTNode::FastSubsequence {
                elements: vec![num(2.0, 1), num(3.0, 2)],
            },
        ]);

        assert_eq!(pattern.run(0.25, 0), Some((Value::Numeric(1.0), 0.0, 0.5)));
        assert_eq!(pattern.run(0.55, 0), Some((Value::Numeric(2.0), 0.5, 0.25)));
        assert_eq!(
            pattern.run(0.75, 0),
            Some((Value::Numeric(3.0), 0.75, 0.25))
        );
    }

    #[test]
    fn test_slow_subsequence() {
        let pattern = PatternProgram::new(vec![ASTNode::SlowSubsequence {
            elements: vec![num(1.0, 0), num(2.0, 1), num(3.0, 2)],
        }]);

        assert_eq!(pattern.run(0.5, 0), Some((Value::Numeric(1.0), 0.0, 1.0)));
        assert_eq!(pattern.run(1.5, 0), Some((Value::Numeric(2.0), 1.0, 1.0)));
        assert_eq!(pattern.run(2.5, 0), Some((Value::Numeric(3.0), 2.0, 1.0)));
        assert_eq!(pattern.run(3.5, 0), Some((Value::Numeric(1.0), 3.0, 1.0)));
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
        assert_eq!(pattern.run(0.5, 0), Some((Value::Numeric(1.0), 0.0, 1.0)));
        assert_eq!(pattern.run(1.5, 0), Some((Value::Numeric(3.0), 1.0, 1.0)));
        assert_eq!(pattern.run(2.5, 0), Some((Value::Numeric(2.0), 2.0, 1.0)));
        assert_eq!(pattern.run(3.5, 0), Some((Value::Numeric(4.0), 3.0, 1.0)));
        assert_eq!(pattern.run(4.5, 0), Some((Value::Numeric(1.0), 4.0, 1.0)));
    }

    #[test]
    fn test_random_choice() {
        let pattern =
            PatternProgram::new(vec![random(vec![num(1.0, 0), num(2.0, 1), num(3.0, 2)])]);
        let mut counts = HashMap::new();
        for i in 0..10000 {
            let time = i as f64;
            if let Some((Value::Numeric(val), _, _)) = pattern.run(time, 0) {
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
            if let Some((Value::Numeric(val), _, _)) = pattern.run(time, 0) {
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
            if let Some((Value::Numeric(val), _, _)) = pattern.run(time, 0) {
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
        assert_eq!(pattern.run(3.5, 0), Some((Value::Numeric(2.0), 3.0, 1.0)));
        assert_eq!(pattern.run(0.5, 0), Some((Value::Numeric(1.0), 0.0, 1.0)));
        assert_eq!(pattern.run(2.5, 0), Some((Value::Numeric(1.0), 2.0, 1.0)));
        assert_eq!(pattern.run(1.5, 0), Some((Value::Numeric(2.0), 1.0, 1.0)));
    }
}
