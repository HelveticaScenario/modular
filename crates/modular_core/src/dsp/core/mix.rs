use anyhow::{anyhow, Result};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::types::Signal;

#[derive(Deserialize, Default, JsonSchema, Connect)]
#[serde(default)]
struct MixParams {
    /// a signal input
    in1: Signal,
    /// a signal input
    in2: Signal,
    /// a signal input
    in3: Signal,
    /// a signal input
    in4: Signal,
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
        let inputs = [&self.params.in1, &self.params.in2, &self.params.in3, &self.params.in4];
        let count = inputs
            .iter()
            .filter(|input| ***input != Signal::Disconnected)
            .count();

        self.outputs.sample = if count > 0 {
            inputs.iter().fold(0.0, |acc, x| acc + x.get_value()) / count as f32
        } else {
            0.0
        }
    }
}

message_handlers!(impl Mix {});
