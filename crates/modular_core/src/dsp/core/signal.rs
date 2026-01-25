use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::PolySignal;

#[derive(Deserialize, Default, JsonSchema, Connect)]
#[serde(default)]
struct SignalParams {
    /// signal input
    source: crate::types::Signal,
}

#[derive(Outputs, JsonSchema)]
struct SignalOutputs {
    #[output("output", "signal output", default)]
    sample: PolySignal,
}

#[derive(Default, Module)]
#[module("signal", "a signal")]
#[args(source?)]
pub struct Signal {
    outputs: SignalOutputs,
    params: SignalParams,
}

impl Signal {
    fn update(&mut self, _sample_rate: f32) -> () {
        self.outputs.sample = self.params.source.get_poly_signal()
    }
}

message_handlers!(impl Signal {});
