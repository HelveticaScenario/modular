use schemars::JsonSchema;
use serde::Deserialize;

use crate::poly::{PolyOutput, PolySignal};

#[derive(Deserialize, Default, JsonSchema, ChannelCount)]
#[serde(default)]
struct SignalParams {
    /// signal input (polyphonic)
    source: PolySignal,
}

impl crate::types::Connect for SignalParams {
    fn connect(&mut self, patch: &crate::Patch) {
        self.source.connect(patch);
    }
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
    fn update(&mut self, _sample_rate: f32) {
        let channels = self.channel_count();
        self.outputs.sample.set_channels(channels);
        for i in 0..channels as usize {
            let val = self.params.source.get_value(i);
            self.outputs.sample.set(i, val);
        }
    }
}

message_handlers!(impl Signal {});
