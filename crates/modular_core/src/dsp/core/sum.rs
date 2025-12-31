use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::types::Signal;

#[derive(Deserialize, Default, JsonSchema, Connect)]
#[serde(default)]
struct SumParams {
    /// signals to sum
    signals: Vec<Signal>,
}

#[derive(Outputs, JsonSchema)]
struct SumOutputs {
    #[output("output", "signal output", default)]
    sample: f32,
}

#[derive(Default, Module)]
#[module("sum", "A signal adder")]
pub struct Sum {
    outputs: SumOutputs,
    params: SumParams,
}

impl Sum {
    fn update(&mut self, _sample_rate: f32) -> () {
        self.outputs.sample = self
            .params
            .signals
            .iter()
            .fold(0.0, |acc, x| acc + x.get_value())
    }
}

message_handlers!(impl Sum {});
