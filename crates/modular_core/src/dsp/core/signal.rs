use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, Default, JsonSchema, Connect)]
#[serde(default)]
struct SignalParams {
    /// signal input
    source: crate::types::Signal,
}

#[derive(Outputs, JsonSchema)]
struct SignalOutputs {
    #[output("output", "signal output", default)]
    sample: f32,
}

#[derive(Default, Module)]
#[module("signal", "a signal")]
pub struct Signal {
    outputs: SignalOutputs,
    params: SignalParams,
}

impl Signal {
    fn update(&mut self, _sample_rate: f32) -> () {
        self.outputs.sample = self.params.source.get_value();
    }
}

message_handlers!(impl Signal {});
