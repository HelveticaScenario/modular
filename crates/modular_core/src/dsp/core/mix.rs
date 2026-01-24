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
#[module("mix", "A mixer that sums all input signals and their channels")]
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
            .filter(|input| **input != Signal::Disconnected)
            .collect();

        if inputs.is_empty() {
            self.outputs.sample = 0.0;
            return;
        }

        // Sum all channels of all polyphonic inputs
        let mut total = 0.0f32;
        let mut channel_count = 0usize;

        for signal in inputs {
            let poly = signal.get_poly_signal();
            let channels = poly.channels() as usize;
            if channels == 0 {
                continue;
            }
            for ch in 0..channels {
                total += poly.get(ch);
                channel_count += 1;
            }
        }

        // Average all channels
        self.outputs.sample = if channel_count > 0 {
            total / channel_count as f32
        } else {
            0.0
        };
    }
}

message_handlers!(impl Mix {});
