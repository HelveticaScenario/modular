use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    pattern::{
        CompiledNode, CompiledPattern, PatternProgram, Rng, Value, hash_components,
        parse_pattern_elements,
    },
    types::Signal,
};

#[derive(Debug, Clone)]
struct CachedNode {
    value: Value,
    time_start: f64,
    time_end: f64,
}

#[derive(Default, JsonSchema)]
#[serde(transparent)]
#[schemars(transparent)]
struct PatternParam {
    source: String,

    #[serde(skip, default)]
    #[schemars(skip)]
    compiled_pattern: CompiledPattern,
}

impl PatternParam {
    fn compile_pattern(source: &str) -> std::result::Result<CompiledPattern, String> {
        let elements = parse_pattern_elements(source).map_err(|e| e.to_string())?;

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::hash::Hash::hash(source, &mut hasher);
        let seed = (std::hash::Hasher::finish(&hasher) & 0xFFFF_FFFF) as u32;

        let program = PatternProgram {
            id: "seq".to_string(),
            elements,
            seed,
        };

        Ok(CompiledPattern::compile(&program))
    }
}

impl<'de> Deserialize<'de> for PatternParam {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let source = String::deserialize(deserializer)?;
        let compiled_pattern =
            Self::compile_pattern(&source).map_err(|e| serde::de::Error::custom(e))?;

        Ok(Self {
            source,
            compiled_pattern,
        })
    }
}

#[derive(Deserialize, Default, JsonSchema, Connect)]
#[serde(default)]
struct SeqParams {
    /// Musical DSL pattern source string (parsed/compiled in Rust)
    pattern: PatternParam,
    /// playhead control signal
    playhead: Signal,
}

#[derive(Outputs, JsonSchema)]
struct SeqOutputs {
    #[output("cv", "control voltage output", default)]
    cv: f32,
    #[output("gate", "gate output")]
    gate: f32,
    #[output("trig", "trigger output")]
    trig: f32,
}

#[derive(Default, Module)]
#[module("seq", "A 4 channel mixer")]
pub struct Seq {
    outputs: SeqOutputs,
    params: SeqParams,
    cached_node: Option<CachedNode>,
    seed: u64,
}

impl Seq {
    fn update(&mut self, _sample_rate: f32) -> () {
        let playhead_value = self.params.playhead.get_value();

        let value = self.run(playhead_value as f64);
        match value {
            Some(Value::Numeric(v)) => {
                self.outputs.cv = v as f32;
                self.outputs.gate = 5.0;
                self.outputs.trig = 1.0;
            }
            Some(Value::Rest) | None => {
                // self.outputs.cv = 0.0;
                self.outputs.gate = 0.0;
                self.outputs.trig = 0.0;
            }
        }
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

        let rng = Rng::new(self.seed);

        self.find_node_in_range(
            &self.params.pattern.compiled_pattern.root,
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
                let hash = hash_components(self.seed, time_bits, choice_id);

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

message_handlers!(impl Seq {});
