use napi::{De, Result};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    dsp::utils::{TempGate, TempGateState},
    pattern::{PatternProgram, Value, parse_pattern_elements},
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
    #[allow(dead_code)]
    source: String,

    #[serde(skip, default)]
    #[schemars(skip)]
    pattern: PatternProgram,
}

impl PatternParam {
    fn parse_pattern(source: &str) -> std::result::Result<PatternProgram, String> {
        let elements = parse_pattern_elements(source).map_err(|e| e.to_string())?;
        let program = PatternProgram { elements };
        Ok(program)
    }
}

impl<'de> Deserialize<'de> for PatternParam {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let source = String::deserialize(deserializer)?;
        let pattern = Self::parse_pattern(&source).map_err(|e| serde::de::Error::custom(e))?;

        Ok(Self { source, pattern })
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

#[derive(Module)]
#[module("seq", "A 4 channel mixer")]
pub struct Seq {
    outputs: SeqOutputs,
    params: SeqParams,
    cached_node: Option<CachedNode>,
    seed: u64,
    trigger: TempGate,
    gate: TempGate,
}

impl Default for Seq {
    fn default() -> Self {
        Self {
            outputs: SeqOutputs::default(),
            params: SeqParams::default(),
            cached_node: None,
            seed: 0,
            trigger: TempGate::new(TempGateState::Low, 0.0, 1.0),
            gate: TempGate::new(TempGateState::Low, 0.0, 5.0),
        }
    }
}

impl Seq {
    fn update(&mut self, _sample_rate: f32) -> () {
        let playhead_value = f64::from(self.params.playhead.get_value());
        if let Some(cached_node) = &self.cached_node {
            if playhead_value >= cached_node.time_start && playhead_value < cached_node.time_end {
                // Use cached value
                match &cached_node.value {
                    Value::Numeric(v) => {
                        self.outputs.cv = *v as f32;
                        self.outputs.gate = self.gate.process();
                        self.outputs.trig = self.trigger.process();
                    }
                    Value::Rest => {
                        self.outputs.gate = self.gate.process();
                        self.outputs.trig = self.trigger.process();
                    }
                }
                return;
            }
        }
        let value = self.params.pattern.pattern.run(playhead_value, self.seed);
        match value {
            Some((value, start, duration)) => {
                match value {
                    Value::Numeric(v) => {
                        self.gate.set_state(TempGateState::Low, TempGateState::High);
                        self.trigger
                            .set_state(TempGateState::High, TempGateState::Low);
                        self.outputs.cv = v as f32;
                        self.outputs.gate = self.gate.process();
                        self.outputs.trig = self.trigger.process();
                    }
                    Value::Rest => {
                        self.trigger
                            .set_state(TempGateState::Low, TempGateState::Low);
                        self.gate.set_state(TempGateState::Low, TempGateState::Low);
                        self.outputs.gate = self.gate.process();
                        self.outputs.trig = self.trigger.process();
                    }
                }
                self.cached_node = Some(CachedNode {
                    value,
                    time_start: start,
                    time_end: start + duration,
                });
            }
            None => {
                self.outputs.cv = 0.0;
                self.outputs.gate = 0.0;
                self.outputs.trig = 0.0;
            }
        }
    }
}

message_handlers!(impl Seq {});
