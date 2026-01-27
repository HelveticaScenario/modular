use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::poly::{PolyOutput, PolySignal};

#[derive(Deserialize, Default, JsonSchema, Connect)]
#[serde(default)]
struct SignalParams {
    /// signal input (polyphonic)
    source: PolySignal,
}

#[derive(Outputs, JsonSchema)]
struct SignalOutputs {
    #[output("output", "signal output", default)]
    sample: PolyOutput,
}

#[derive(Default, Module)]
#[module("signal", "a polyphonic signal passthrough")]
#[args(source)]
pub struct Signal {
    outputs: SignalOutputs,
    params: SignalParams,
}

impl Signal {
    fn update(&mut self, _sample_rate: f32) -> () {
        let input = &self.params.source;
        let channels = input.channels();
        self.outputs.sample.set_channels(channels);
        for i in 0..channels as usize {
            self.outputs.sample.set(i, input.get_value(i));
        }
    }
}

message_handlers!(impl Signal {});
