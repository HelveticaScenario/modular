use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    dsp::utils::{TempGate, TempGateState, hz_to_voct_f64, midi_to_voct_f64},
    pattern::{AddPatternType, PatternProgram, PitchValue, Value, apply_add, parse_pattern},
    types::{Connect, Signal},
};

#[derive(Deserialize, Default, JsonSchema, Connect)]
#[serde(default)]
struct SeqParams {
    /// Strudel/tidalcycles style pattern string
    pattern: (),
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
#[module("seq", "A strudel/tidalcycles style sequencer")]
#[args(pattern, playhead?)]
// #[stateful]
pub struct Seq {
    outputs: SeqOutputs,
    params: SeqParams,
    trigger: TempGate,
    gate: TempGate,
}

impl Default for Seq {
    fn default() -> Self {
        Self {
            outputs: SeqOutputs::default(),
            params: SeqParams::default(),
            trigger: TempGate::new(TempGateState::Low, 0.0, 1.0),
            gate: TempGate::new(TempGateState::Low, 0.0, 5.0),
        }
    }
}

impl Seq {
    fn update(&mut self, _sample_rate: f32) -> () {}
}

// impl crate::types::StatefulModule for Seq {
//     fn get_state(&self) -> Option<serde_json::Value> {
//         self.cached_node.as_ref().map(|cached| {
//             serde_json::json!({
//                 "active_step": cached.idx,
//                 "active_scale_step": cached.scale_modifier.as_ref().map(|(_, idx)| idx),
//                 "active_add_step": cached.add_modifier.as_ref().map(|(_, idx, _)| idx)
//             })
//         })
//     }
// }

message_handlers!(impl Seq {});
