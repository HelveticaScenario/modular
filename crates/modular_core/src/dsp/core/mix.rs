use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::types::Signal;

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default)]
struct MixParams {
    /// signals to mix (each signal is mono)
    signals: Vec<Signal>,
}

#[derive(Outputs, JsonSchema)]
struct MixOutputs {
    #[output("output", "signal output", default)]
    sample: f32,
}

#[derive(Default, Module)]
#[module("mix", "A mixer that sums and averages all input signals")]
#[args(signals)]
pub struct Mix {
    outputs: MixOutputs,
    params: MixParams,
}

impl Mix {
    fn update(&mut self, _sample_rate: f32) {
        let inputs: Vec<_> = self
            .params
            .signals
            .iter()
            .filter(|input| !input.is_disconnected())
            .collect();

        if inputs.is_empty() {
            self.outputs.sample = 0.0;
            return;
        }

        // Sum all mono signals
        let mut total = 0.0f32;
        let count = inputs.len();

        for signal in inputs {
            total += signal.get_value();
        }

        // Average all signals
        self.outputs.sample = if count > 0 {
            total / count as f32
        } else {
            0.0
        };
    }
}

message_handlers!(impl Mix {});
