use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::types::Signal;

#[derive(Deserialize, Default, JsonSchema, Connect)]
#[serde(default)]
struct MixParams {
    /// signals to mix
    signals: Vec<Signal>,
}

#[derive(Outputs, JsonSchema)]
struct MixOutputs {
    #[output("output", "signal output", default)]
    sample: f32,
}

#[derive(Default, Module)]
#[module("mix", "A 4 channel mixer")]
pub struct Mix {
    outputs: MixOutputs,
    params: MixParams,
}

impl Mix {
    fn update(&mut self, _sample_rate: f32) -> () {
        let inputs: Vec<_> = self
            .params
            .signals
            .iter()
            .filter(|input| **input != Signal::Disconnected)
            .collect();
        let count = inputs.len();
        self.outputs.sample = if count > 0 {
            inputs.into_iter().fold(0.0, |acc, x| acc + x.get_value()) / count as f32
        } else {
            0.0
        };
    }
}

message_handlers!(impl Mix {});
