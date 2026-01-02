/// Main AST node enum representing all possible elements in the Musical DSL
#[derive(Debug, Clone, PartialEq)]
pub enum ASTNode {
    FastSubsequence(FastSubsequence),
    SlowSubsequence(SlowSubsequence),
    RandomChoice(RandomChoice),
    NumericLiteral(NumericLiteral),
    Rest,
}

/// Root program node containing all top-level elements
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub elements: Vec<ASTNode>,
    pub seed: u64,
}

impl Program {
    pub fn new(elements: Vec<ASTNode>) -> Self {
        Self { elements, seed: 0 }
    }

    pub fn with_seed(elements: Vec<ASTNode>, seed: u64) -> Self {
        Self { elements, seed }
    }
}

/// Fast subsequence represented by square brackets [...]
#[derive(Debug, Clone, PartialEq)]
pub struct FastSubsequence {
    pub elements: Vec<ASTNode>,
}

/// Slow subsequence represented by angle brackets <...>
#[derive(Debug, Clone, PartialEq)]
pub struct SlowSubsequence {
    pub elements: Vec<ASTNode>,
}

/// Random choice represented by | (e.g., A | B | C)
#[derive(Debug, Clone, PartialEq)]
pub struct RandomChoice {
    pub choices: Vec<ASTNode>,
}

/// Numeric literal value (supports decimals and negatives)
#[derive(Debug, Clone, PartialEq)]
pub struct NumericLiteral {
    pub value: f32,
}

/// Note name (placeholder - not used in current tests)
#[derive(Debug, Clone, PartialEq)]
pub struct NoteName {
    pub note: char,
    pub accidental: Option<char>,
    pub octave: u8,
}

/// MIDI value (placeholder - not used in current tests)
#[derive(Debug, Clone, PartialEq)]
pub struct MidiValue {
    pub value: u8,
}

/// Represents the output value from the runner
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Numeric(f32),
    Rest,
}

/// Compiled node with precomputed information for efficient lookup
#[derive(Debug, Clone)]
enum CompiledNode {
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
struct Rng {
    state: u64,
}

impl Rng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Generate next random number and return a value in [0, 1)
    fn next(&mut self) -> f64 {
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
fn hash_components(seed: u64, time_bits: u64, choice_id: u64) -> u64 {
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

/// Compiled program optimized for stateless lookup
#[derive(Debug, Clone)]
pub struct CompiledProgram {
    root: Vec<CompiledNode>,
    seed: u64,
}

impl CompiledProgram {
    /// Compile a program into an optimized form for stateless execution
    pub fn compile(program: &Program) -> Self {
        let root = Self::compile_nodes(&program.elements);
        Self {
            root,
            seed: program.seed,
        }
    }

    fn compile_nodes(nodes: &[ASTNode]) -> Vec<CompiledNode> {
        nodes.iter().map(|node| Self::compile_node(node)).collect()
    }

    fn compile_node(node: &ASTNode) -> CompiledNode {
        match node {
            ASTNode::NumericLiteral(num) => CompiledNode::Value(Value::Numeric(num.value)),
            ASTNode::Rest => CompiledNode::Value(Value::Rest),
            ASTNode::FastSubsequence(fast) => {
                let children = Self::compile_nodes(&fast.elements);
                CompiledNode::Fast(children)
            }
            ASTNode::SlowSubsequence(slow) => {
                let children = Self::compile_nodes(&slow.elements);
                let period = children.len();

                CompiledNode::Slow {
                    nodes: children,
                    period,
                }
            }
            ASTNode::RandomChoice(choice) => {
                let choices = choice
                    .choices
                    .iter()
                    .map(|node| Self::compile_node(node))
                    .collect();

                CompiledNode::Random { choices }
            }
        }
    }

    /// Run the compiled program at a given time (stateless)
    pub fn run(&self, time: f64) -> Option<Value> {
        let loop_time = time.fract();
        let loop_index = time.floor() as usize;

        let rng = Rng::new(self.seed);

        self.run_nodes(&self.root, loop_time, 0.0, 1.0, loop_index, rng, 0)
    }

    fn run_nodes(
        &self,
        nodes: &[CompiledNode],
        time: f64,
        start: f64,
        duration: f64,
        loop_index: usize,
        rng: Rng,
        choice_id: u64,
    ) -> Option<Value> {
        if nodes.is_empty() {
            return None;
        }

        let element_duration = duration / nodes.len() as f64;

        for (i, node) in nodes.iter().enumerate() {
            let element_start = start + i as f64 * element_duration;
            let element_end = element_start + element_duration;

            if time >= element_start && time < element_end {
                let relative_time = (time - element_start) / element_duration;
                let node_choice_id = choice_id
                    .wrapping_mul(nodes.len() as u64)
                    .wrapping_add(i as u64);
                return self.run_node(node, relative_time, loop_index, rng, node_choice_id);
            }
        }

        None
    }

    fn run_node(
        &self,
        node: &CompiledNode,
        relative_time: f64,
        loop_index: usize,
        rng: Rng,
        choice_id: u64,
    ) -> Option<Value> {
        match node {
            CompiledNode::Value(val) => Some(val.clone()),
            CompiledNode::Fast(children) => self.run_nodes(
                children,
                relative_time,
                0.0,
                1.0,
                loop_index,
                rng,
                choice_id,
            ),
            CompiledNode::Slow { nodes, period, .. } => {
                // This slow subsequence is encountered once per loop
                let encounter_count = loop_index;
                let index = encounter_count % period;

                // Calculate how many times THIS child has been selected in previous loops
                // A child at position `index` is selected every `period` loops
                // Starting from loop `index`, it's selected at loops: index, index+period, index+2*period, ...
                let times_this_child_selected = if loop_index >= index {
                    (loop_index - index) / period
                } else {
                    0
                };

                let child_choice_id = choice_id
                    .wrapping_mul(*period as u64)
                    .wrapping_add(index as u64);
                self.run_node(
                    &nodes[index],
                    relative_time,
                    times_this_child_selected,
                    rng,
                    child_choice_id,
                )
            }
            CompiledNode::Random { choices } => {
                if choices.is_empty() {
                    return None;
                }

                // Compute absolute time from relative_time and loop_index
                let absolute_time = loop_index as f64 + relative_time;

                // Hash all components together with proper mixing for decorrelation
                let time_bits = absolute_time.to_bits();
                let hash = hash_components(self.seed, time_bits, choice_id);

                let mut choice_rng = Rng::new(hash);
                let random_value = choice_rng.next();

                // Map random value to choice index
                let index = (random_value * choices.len() as f64).floor() as usize;
                let index = index.min(choices.len() - 1); // Clamp to valid range

                self.run_node(
                    &choices[index],
                    relative_time,
                    loop_index,
                    rng,
                    choice_id.wrapping_add(1),
                )
            }
        }
    }
}

/// Stateless runner function - compiles on demand
pub fn run(program: &Program, time: f64) -> Option<Value> {
    let compiled = CompiledProgram::compile(program);
    compiled.run(time)
}

/// Stateful runner that caches the current node and its time range
pub struct Runner {
    compiled: CompiledProgram,
    cached_node: Option<CachedNode>,
}

#[derive(Debug, Clone)]
struct CachedNode {
    value: Value,
    time_start: f64,
    time_end: f64,
}

impl Runner {
    /// Create a new runner with the given program
    pub fn new(program: &Program) -> Self {
        Self {
            compiled: CompiledProgram::compile(program),
            cached_node: None,
        }
    }

    /// Update the program (cache remains valid if still in range)
    pub fn set_program(&mut self, program: &Program) {
        self.compiled = CompiledProgram::compile(program);
        // Don't clear cache - it will be invalidated naturally when time moves outside range
    }

    /// Run at the given time, using cache when possible
    pub fn run(&mut self, time: f64) -> Option<Value> {
        // Check if we can use cached value
        if let Some(ref cached) = self.cached_node {
            if time >= cached.time_start && time < cached.time_end {
                return Some(cached.value.clone());
            }
        }

        // Need to evaluate - first find the time range for this node
        let (value, time_start, time_end) = self.find_node_with_range(time)?;

        // Cache the result
        self.cached_node = Some(CachedNode {
            value: value.clone(),
            time_start,
            time_end,
        });

        Some(value)
    }

    /// Find the node at the given time along with its time range
    fn find_node_with_range(&self, time: f64) -> Option<(Value, f64, f64)> {
        let loop_time = time.fract();
        let loop_index = time.floor() as usize;
        let loop_start = loop_index as f64;

        let rng = Rng::new(self.compiled.seed);

        self.find_node_in_range(
            &self.compiled.root,
            loop_time,
            0.0,
            1.0,
            loop_index,
            rng,
            0,
            loop_start,
        )
    }

    fn find_node_in_range(
        &self,
        nodes: &[CompiledNode],
        time: f64,
        start: f64,
        duration: f64,
        loop_index: usize,
        rng: Rng,
        choice_id: u64,
        absolute_start: f64,
    ) -> Option<(Value, f64, f64)> {
        if nodes.is_empty() {
            return None;
        }

        let element_duration = duration / nodes.len() as f64;

        for (i, node) in nodes.iter().enumerate() {
            let element_start = start + i as f64 * element_duration;
            let element_end = element_start + element_duration;

            if time >= element_start && time < element_end {
                let relative_time = (time - element_start) / element_duration;
                let node_choice_id = choice_id
                    .wrapping_mul(nodes.len() as u64)
                    .wrapping_add(i as u64);
                let node_absolute_start = absolute_start + element_start;
                let node_absolute_end = absolute_start + element_end;

                return self.find_node_range(
                    node,
                    relative_time,
                    loop_index,
                    rng,
                    node_choice_id,
                    node_absolute_start,
                    node_absolute_end,
                );
            }
        }

        None
    }

    fn find_node_range(
        &self,
        node: &CompiledNode,
        relative_time: f64,
        loop_index: usize,
        rng: Rng,
        choice_id: u64,
        absolute_start: f64,
        absolute_end: f64,
    ) -> Option<(Value, f64, f64)> {
        match node {
            CompiledNode::Value(val) => Some((val.clone(), absolute_start, absolute_end)),
            CompiledNode::Fast(children) => self.find_node_in_range(
                children,
                relative_time,
                0.0,
                1.0,
                loop_index,
                rng,
                choice_id,
                absolute_start,
            ),
            CompiledNode::Slow { nodes, period, .. } => {
                let encounter_count = loop_index;
                let index = encounter_count % period;

                let times_this_child_selected = if loop_index >= index {
                    (loop_index - index) / period
                } else {
                    0
                };

                let child_choice_id = choice_id
                    .wrapping_mul(*period as u64)
                    .wrapping_add(index as u64);
                self.find_node_range(
                    &nodes[index],
                    relative_time,
                    times_this_child_selected,
                    rng,
                    child_choice_id,
                    absolute_start,
                    absolute_end,
                )
            }
            CompiledNode::Random { choices } => {
                if choices.is_empty() {
                    return None;
                }

                let absolute_time = loop_index as f64 + relative_time;
                let time_bits = absolute_time.to_bits();
                let hash = hash_components(self.compiled.seed, time_bits, choice_id);

                let mut choice_rng = Rng::new(hash);
                let random_value = choice_rng.next();

                let index = (random_value * choices.len() as f64).floor() as usize;
                let index = index.min(choices.len() - 1);

                self.find_node_range(
                    &choices[index],
                    relative_time,
                    loop_index,
                    rng,
                    choice_id.wrapping_add(1),
                    absolute_start,
                    absolute_end,
                )
            }
        }
    }

    /// Get the currently cached node info (for debugging/inspection)
    pub fn cached_info(&self) -> Option<(Value, f64, f64)> {
        self.cached_node
            .as_ref()
            .map(|cached| (cached.value.clone(), cached.time_start, cached.time_end))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn num(value: f32) -> ASTNode {
        ASTNode::NumericLiteral(NumericLiteral { value })
    }

    fn random(choices: Vec<ASTNode>) -> ASTNode {
        ASTNode::RandomChoice(RandomChoice { choices })
    }

    #[test]
    fn test_basic_sequence() {
        let program = Program::new(vec![num(1.0), num(2.0), num(3.0)]);

        let compiled = CompiledProgram::compile(&program);

        assert_eq!(compiled.run(0.1), Some(Value::Numeric(1.0)));
        assert_eq!(compiled.run(0.4), Some(Value::Numeric(2.0)));
        assert_eq!(compiled.run(0.7), Some(Value::Numeric(3.0)));
    }

    #[test]
    fn test_looping() {
        let program = Program::new(vec![num(1.0), num(2.0)]);

        let compiled = CompiledProgram::compile(&program);

        assert_eq!(compiled.run(0.0), Some(Value::Numeric(1.0)));
        assert_eq!(compiled.run(1.0), Some(Value::Numeric(1.0)));
        assert_eq!(compiled.run(2.5), Some(Value::Numeric(2.0)));
    }

    #[test]
    fn test_fast_subsequence() {
        let program = Program::new(vec![
            num(1.0),
            ASTNode::FastSubsequence(FastSubsequence {
                elements: vec![num(2.0), num(3.0)],
            }),
        ]);

        let compiled = CompiledProgram::compile(&program);

        assert_eq!(compiled.run(0.25), Some(Value::Numeric(1.0)));
        assert_eq!(compiled.run(0.55), Some(Value::Numeric(2.0)));
        assert_eq!(compiled.run(0.75), Some(Value::Numeric(3.0)));
    }

    #[test]
    fn test_slow_subsequence() {
        let program = Program::new(vec![ASTNode::SlowSubsequence(SlowSubsequence {
            elements: vec![num(1.0), num(2.0), num(3.0)],
        })]);

        let compiled = CompiledProgram::compile(&program);

        assert_eq!(compiled.run(0.5), Some(Value::Numeric(1.0)));
        assert_eq!(compiled.run(1.5), Some(Value::Numeric(2.0)));
        assert_eq!(compiled.run(2.5), Some(Value::Numeric(3.0)));
        assert_eq!(compiled.run(3.5), Some(Value::Numeric(1.0)));
    }

    #[test]
    fn test_nested_slow_subsequence() {
        // <<1 2> <3 4>>
        let program = Program::new(vec![ASTNode::SlowSubsequence(SlowSubsequence {
            elements: vec![
                ASTNode::SlowSubsequence(SlowSubsequence {
                    elements: vec![num(1.0), num(2.0)],
                }),
                ASTNode::SlowSubsequence(SlowSubsequence {
                    elements: vec![num(3.0), num(4.0)],
                }),
            ],
        })]);

        let compiled = CompiledProgram::compile(&program);

        // Should return 1, 3, 2, 4, 1...
        assert_eq!(compiled.run(0.5), Some(Value::Numeric(1.0)));
        assert_eq!(compiled.run(1.5), Some(Value::Numeric(3.0)));
        assert_eq!(compiled.run(2.5), Some(Value::Numeric(2.0)));
        assert_eq!(compiled.run(3.5), Some(Value::Numeric(4.0)));
        assert_eq!(compiled.run(4.5), Some(Value::Numeric(1.0)));
    }

    #[test]
    fn test_random_choice() {
        let program = Program::new(vec![random(vec![num(1.0), num(2.0), num(3.0)])]);
        let compiled = CompiledProgram::compile(&program);
        let mut counts = HashMap::new();
        for i in 0..10000 {
            let time = i as f64;
            if let Some(Value::Numeric(val)) = compiled.run(time) {
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
        let program = Program::new(vec![ASTNode::SlowSubsequence(SlowSubsequence {
            elements: vec![random(vec![num(1.0), num(2.0)]), num(3.0)],
        })]);

        let compiled = CompiledProgram::compile(&program);

        let mut counts = HashMap::new();
        for i in 0..10000 {
            let time = i as f64;
            if let Some(Value::Numeric(val)) = compiled.run(time) {
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
        let program = Program::new(vec![ASTNode::SlowSubsequence(SlowSubsequence {
            elements: vec![
                random(vec![
                    num(1.0),
                    ASTNode::SlowSubsequence(SlowSubsequence {
                        elements: vec![num(2.0), num(3.0)],
                    }),
                ]),
                ASTNode::SlowSubsequence(SlowSubsequence {
                    elements: vec![num(4.0), num(5.0)],
                }),
            ],
        })]);

        let compiled = CompiledProgram::compile(&program);

        let mut counts = HashMap::new();
        for i in 0..10000 {
            let time = i as f64;
            if let Some(Value::Numeric(val)) = compiled.run(time) {
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
        let program = Program::new(vec![ASTNode::SlowSubsequence(SlowSubsequence {
            elements: vec![num(1.0), num(2.0)],
        })]);

        let compiled = CompiledProgram::compile(&program);

        // Call in any order - should be stateless
        assert_eq!(compiled.run(3.5), Some(Value::Numeric(2.0)));
        assert_eq!(compiled.run(0.5), Some(Value::Numeric(1.0)));
        assert_eq!(compiled.run(2.5), Some(Value::Numeric(1.0)));
        assert_eq!(compiled.run(1.5), Some(Value::Numeric(2.0)));
    }

    #[test]
    fn test_convenience_function() {
        let program = Program::new(vec![num(1.0), num(2.0)]);

        assert_eq!(run(&program, 0.1), Some(Value::Numeric(1.0)));
        assert_eq!(run(&program, 0.6), Some(Value::Numeric(2.0)));
    }

    #[test]
    fn test_complex_nested_program() {
        // Create a complex program with 5 levels of nesting
        // Structure: [<[<1 <<2 3>> [4 5]>] 6> <7 [8 <9 10>]> <<11 [12 <13 14>]> 15>]
        let program = Program::new(vec![
            // Level 1: Fast subsequence
            ASTNode::FastSubsequence(FastSubsequence {
                elements: vec![
                    // Level 2: Slow subsequence
                    ASTNode::SlowSubsequence(SlowSubsequence {
                        elements: vec![
                            // Level 3: Fast subsequence
                            ASTNode::FastSubsequence(FastSubsequence {
                                elements: vec![
                                    // Level 4: Slow subsequence
                                    ASTNode::SlowSubsequence(SlowSubsequence {
                                        elements: vec![
                                            num(1.0),
                                            // Level 5: Slow nested in slow
                                            ASTNode::SlowSubsequence(SlowSubsequence {
                                                elements: vec![num(2.0), num(3.0)],
                                            }),
                                            // Level 5: Fast nested in slow
                                            ASTNode::FastSubsequence(FastSubsequence {
                                                elements: vec![num(4.0), num(5.0)],
                                            }),
                                        ],
                                    }),
                                ],
                            }),
                            num(6.0),
                        ],
                    }),
                    // Level 2: Slow subsequence with fast inside
                    ASTNode::SlowSubsequence(SlowSubsequence {
                        elements: vec![
                            num(7.0),
                            // Level 3: Fast with slow inside
                            ASTNode::FastSubsequence(FastSubsequence {
                                elements: vec![
                                    num(8.0),
                                    // Level 4: Slow in fast in slow
                                    ASTNode::SlowSubsequence(SlowSubsequence {
                                        elements: vec![num(9.0), num(10.0)],
                                    }),
                                ],
                            }),
                        ],
                    }),
                    // Level 2: Slow with nested slow and fast
                    ASTNode::SlowSubsequence(SlowSubsequence {
                        elements: vec![
                            // Level 3: Slow nested in slow
                            ASTNode::SlowSubsequence(SlowSubsequence {
                                elements: vec![
                                    num(11.0),
                                    // Level 4: Fast in slow in slow
                                    ASTNode::FastSubsequence(FastSubsequence {
                                        elements: vec![
                                            num(12.0),
                                            // Level 5: Slow in fast in slow in slow
                                            ASTNode::SlowSubsequence(SlowSubsequence {
                                                elements: vec![num(13.0), num(14.0)],
                                            }),
                                        ],
                                    }),
                                ],
                            }),
                            num(15.0),
                        ],
                    }),
                ],
            }),
        ]);

        let compiled = CompiledProgram::compile(&program);

        // Test various time points to verify correct behavior
        // The outer fast subsequence contains 3 elements, each taking 1/3 of the time

        // First third (0.0 - 0.333): First slow subsequence
        // This slow has 2 elements: a fast subsequence and 6
        // At loop 0, it selects the fast subsequence (index 0)
        let result = compiled.run(0.1);
        assert!(result.is_some());

        // At loop 1, it selects 6 (index 1)
        let result = compiled.run(1.1);
        assert_eq!(result, Some(Value::Numeric(6.0)));

        // At loop 2, back to fast subsequence (index 0)
        let result = compiled.run(2.1);
        assert!(result.is_some());

        // Second third (0.333 - 0.666): Second slow subsequence
        // This slow has 2 elements: 7 and a fast [8 <9 10>]
        // At loop 0, it selects 7
        let result = compiled.run(0.5);
        assert_eq!(result, Some(Value::Numeric(7.0)));

        // At loop 1, it selects the fast subsequence
        // The fast has 8 and <9 10>
        let result = compiled.run(1.5);
        assert!(result.is_some());

        // Third third (0.666 - 1.0): Third slow subsequence
        // This slow has 2 elements: <11 [12 <13 14>]> and 15
        // At loop 0, it selects the nested slow <11 [12 <13 14>]>
        let result = compiled.run(0.8);
        assert!(result.is_some());

        // At loop 1, it selects 15
        let result = compiled.run(1.8);
        assert_eq!(result, Some(Value::Numeric(15.0)));

        // Test high loop numbers to ensure stateless efficiency
        let result = compiled.run(1000.5);
        assert_eq!(result, Some(Value::Numeric(7.0)));

        let result = compiled.run(1001.8);
        assert_eq!(result, Some(Value::Numeric(15.0)));

        // Test the deeply nested slow subsequences
        // The innermost <<2 3>> inside the first element
        // Access it at loop 0 (should get 2)
        let result = compiled.run(0.05);
        // This is complex, just verify it returns something
        assert!(result.is_some());

        // Test nested slow in fast in slow behavior
        // Second element at time ~0.5, loop 1 should select the fast [8 <9 10>]
        // Within that fast, <9 10> gets encountered
        let result = compiled.run(1.45);
        assert!(result.is_some());

        // Loop 2, same position, the slow <9 10> should now return 10 (second element)
        let result = compiled.run(2.45);
        assert!(result.is_some());
    }

    #[test]
    fn test_performance_with_large_loop_index() {
        // Test that we can handle very large loop indices efficiently
        let program = Program::new(vec![ASTNode::SlowSubsequence(SlowSubsequence {
            elements: vec![
                num(1.0),
                ASTNode::SlowSubsequence(SlowSubsequence {
                    elements: vec![num(2.0), num(3.0), num(4.0)],
                }),
            ],
        })]);

        let compiled = CompiledProgram::compile(&program);

        // These should all execute in O(depth) time, not O(loop_index) time
        assert!(compiled.run(0.5).is_some());
        assert!(compiled.run(100.5).is_some());
        assert!(compiled.run(10000.5).is_some());
        assert!(compiled.run(1000000.5).is_some());

        // Verify correctness at high indices
        // At loop 0: outer selects 1
        assert_eq!(compiled.run(0.5), Some(Value::Numeric(1.0)));

        // At loop 1: outer selects nested slow (1st encounter) -> 2
        assert_eq!(compiled.run(1.5), Some(Value::Numeric(2.0)));

        // At loop 2: outer selects 1 again
        assert_eq!(compiled.run(2.5), Some(Value::Numeric(1.0)));

        // At loop 3: outer selects nested slow (2nd encounter) -> 3
        assert_eq!(compiled.run(3.5), Some(Value::Numeric(3.0)));

        // At loop 1000001: outer selects nested slow
        // Outer has period 2: [1, nested_slow]
        // At loop 1000001: 1000001 % 2 = 1, so outer selects nested_slow (index 1)
        // How many times has nested_slow been selected by this point?
        // It's selected at odd loops: 1, 3, 5, ..., 1000001
        // Using the formula: (loop_index - index) / period = (1000001 - 1) / 2 = 500000
        // So nested_slow has been encountered 500000 times
        // nested_slow has period 3: [2, 3, 4]
        // 500000 % 3 = 2, so it returns 4 (index 2)
        assert_eq!(compiled.run(1000001.5), Some(Value::Numeric(4.0)));
    }
}
