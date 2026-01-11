use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    dsp::utils::{TempGate, TempGateState},
    pattern::{AddPatternType, PatternProgram, PitchValue, Value, apply_add, parse_pattern},
    types::{Connect, Signal},
};

/// A0 frequency in Hz (reference for V/Oct)
const A0_HZ: f64 = 27.5;

/// Convert Hz to V/Oct
fn hz_to_voct(hz: f64) -> f64 {
    (hz / A0_HZ).log2()
}

/// Convert MIDI note to V/Oct (A0 = 0V = MIDI 21)
fn midi_to_voct(midi: f64) -> f64 {
    (midi - 21.0) / 12.0
}

/// Convert a PitchValue to V/Oct
fn pitch_to_voct(pv: &PitchValue) -> f64 {
    match pv {
        PitchValue::Volts(v) => *v,
        PitchValue::Hz(hz) => hz_to_voct(*hz),
        PitchValue::Midi(m) => midi_to_voct(*m),
        PitchValue::ScaleInterval(_) => {
            // ScaleInterval should be resolved during parsing for simple scales
            // For patternable scales, this would need runtime resolution
            // For now, treat as 0V (will be implemented with full patternable scale support)
            0.0
        }
    }
}

#[derive(Debug, Clone)]
struct CachedNode {
    value: Value,
    time_start: f64,
    time_end: f64,
    idx: usize,
    scale_modifier: Option<(crate::pattern::ScaleDefinition, usize)>,
    add_modifier: Option<(f64, usize, AddPatternType)>,
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
    fn do_parse(source: &str) -> std::result::Result<PatternProgram, String> {
        parse_pattern(source).map_err(|e| e.to_string())
    }
}

impl<'de> Deserialize<'de> for PatternParam {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let source = String::deserialize(deserializer)?;
        let pattern = Self::do_parse(&source).map_err(|e| serde::de::Error::custom(e))?;
        println!("PatternParam deserialized: {:#?}", pattern);
        Ok(Self { source, pattern })
    }
}

impl Connect for PatternParam {
    fn connect(&mut self, _patch: &crate::Patch) {
        Connect::connect(&mut self.pattern, _patch);
    }
}

#[derive(Deserialize, Default, JsonSchema)]
#[serde(default)]
struct SeqParams {
    /// Musical DSL pattern source string (parsed/compiled in Rust)
    pattern: PatternParam,
    /// playhead control signal
    playhead: Signal,
}

impl Connect for SeqParams {
    fn connect(&mut self, patch: &crate::Patch) {
        Connect::connect(&mut self.playhead, patch);
        Connect::connect(&mut self.pattern, patch);
    }
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
#[args(pattern, playhead?)]
#[stateful]
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
                        self.outputs.cv = apply_add(
                            self.outputs.cv as f64,
                            &cached_node.value,
                            &cached_node.add_modifier,
                            &cached_node.scale_modifier,
                        ) as f32;
                    }
                    Value::ModuleRef { signal, .. } => {
                        self.outputs.cv = signal.get_value();
                        self.outputs.gate = self.gate.process();
                        self.outputs.trig = self.trigger.process();
                        self.outputs.cv = apply_add(
                            self.outputs.cv as f64,
                            &cached_node.value,
                            &cached_node.add_modifier,
                            &cached_node.scale_modifier,
                        ) as f32;
                    }
                    Value::Rest => {
                        self.outputs.gate = self.gate.process();
                        self.outputs.trig = self.trigger.process();
                    }
                    Value::Pitch(pv) => {
                        let voct = pitch_to_voct(pv);
                        self.outputs.cv = voct as f32;
                        self.outputs.gate = self.gate.process();
                        self.outputs.trig = self.trigger.process();
                        self.outputs.cv = apply_add(
                            self.outputs.cv as f64,
                            &cached_node.value,
                            &cached_node.add_modifier,
                            &cached_node.scale_modifier,
                        ) as f32;
                    }
                    Value::UnresolvedNumeric(_) => {
                        unreachable!("UnresolvedNumeric should be resolved during parsing")
                    }
                }
                return;
            }
        }
        let value = self.params.pattern.pattern.run(playhead_value, self.seed);
        match value {
            Some((value, start, duration, idx)) => {
                // Run scale pattern to get current scale (for add pattern resolution)
                let scale_modifier: Option<(crate::pattern::ScaleDefinition, usize)> = self
                    .params
                    .pattern
                    .pattern
                    .scale_pattern
                    .as_ref()
                    .and_then(|sp| sp.run(playhead_value, self.seed));

                // Run add pattern and apply the add value to CV
                let add_modifier: Option<(f64, usize, AddPatternType)> = self
                    .params
                    .pattern
                    .pattern
                    .add_pattern
                    .as_ref()
                    .and_then(|ap| {
                        ap.run(playhead_value, self.seed)
                            .map(|(value, idx)| (value, idx, ap.value_type))
                    });

                match value {
                    Value::Numeric(v) => {
                        self.gate.set_state(TempGateState::Low, TempGateState::High);
                        self.trigger
                            .set_state(TempGateState::High, TempGateState::Low);
                        self.outputs.cv = v as f32;
                        self.outputs.gate = self.gate.process();
                        self.outputs.trig = self.trigger.process();
                        self.outputs.cv = apply_add(
                            self.outputs.cv as f64,
                            &value,
                            &add_modifier,
                            &scale_modifier,
                        ) as f32;
                    }
                    Value::ModuleRef { ref signal, .. } => {
                        // For now, treat module refs as rests
                        self.gate.set_state(TempGateState::Low, TempGateState::High);
                        self.trigger
                            .set_state(TempGateState::High, TempGateState::Low);
                        self.outputs.cv = signal.get_value();
                        self.outputs.gate = self.gate.process();
                        self.outputs.trig = self.trigger.process();
                        self.outputs.cv = apply_add(
                            self.outputs.cv as f64,
                            &value,
                            &add_modifier,
                            &scale_modifier,
                        ) as f32;
                    }
                    Value::Rest => {
                        self.trigger
                            .set_state(TempGateState::Low, TempGateState::Low);
                        self.gate.set_state(TempGateState::Low, TempGateState::Low);
                        self.outputs.gate = self.gate.process();
                        self.outputs.trig = self.trigger.process();
                    }
                    Value::Pitch(ref pv) => {
                        self.gate.set_state(TempGateState::Low, TempGateState::High);
                        self.trigger
                            .set_state(TempGateState::High, TempGateState::Low);
                        let voct = pitch_to_voct(pv);
                        self.outputs.cv = voct as f32;
                        self.outputs.gate = self.gate.process();
                        self.outputs.trig = self.trigger.process();
                        self.outputs.cv = apply_add(
                            self.outputs.cv as f64,
                            &value,
                            &add_modifier,
                            &scale_modifier,
                        ) as f32;
                    }
                    Value::UnresolvedNumeric(_) => {
                        unreachable!("UnresolvedNumeric should be resolved during parsing")
                    }
                }

                self.cached_node = Some(CachedNode {
                    value,
                    time_start: start,
                    time_end: start + duration,
                    idx,
                    scale_modifier,
                    add_modifier,
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

impl crate::types::StatefulModule for Seq {
    fn get_state(&self) -> Option<serde_json::Value> {
        self.cached_node.as_ref().map(|cached| {
            serde_json::json!({
                "active_step": cached.idx,
                "active_scale_step": cached.scale_modifier.as_ref().map(|(_, idx)| idx),
                "active_add_step": cached.add_modifier.as_ref().map(|(_, idx, _)| idx)
            })
        })
    }
}

message_handlers!(impl Seq {});
