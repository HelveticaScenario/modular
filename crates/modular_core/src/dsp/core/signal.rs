use schemars::JsonSchema;
use serde::Deserialize;

use crate::poly::{PolyOutput, PolySignal, PolySignalExt};

#[derive(Clone, Deserialize, Default, JsonSchema, ChannelCount, SignalParams)]
#[serde(rename_all = "camelCase")]
struct SignalParams {
    /// Input signal to forward.
    #[serde(default)]
    source: Option<PolySignal>,
}

impl crate::types::Connect for SignalParams {
    fn connect(&mut self, patch: &crate::Patch) {
        if let Some(ref mut s) = self.source {
            s.connect(patch);
        }
    }
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct SignalOutputs {
    #[output("output", "forwarded signal", default)]
    sample: PolyOutput,
}

/// Utility module for routing, naming, and exposing signals in a patch.
#[module(name = "$signal", args(source))]
#[derive(Default)]
pub struct Signal {
    outputs: SignalOutputs,
    params: SignalParams,
}

impl Signal {
    fn update(&mut self, _sample_rate: f32) {
        let channels = self.channel_count();
        for i in 0..channels as usize {
            let val = self.params.source.value_or_zero(i);
            self.outputs.sample.set(i, val);
        }
    }
}

message_handlers!(impl Signal {});
