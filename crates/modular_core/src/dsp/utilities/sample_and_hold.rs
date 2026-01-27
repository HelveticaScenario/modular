use crate::{dsp::utils::SchmittTrigger, types::Signal};
use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default)]
struct SampleAndHoldParams {
    input: Signal,
    trigger: Signal,
}

#[derive(Outputs, JsonSchema)]
struct SampleAndHoldOutputs {
    #[output("output", "output", default)]
    sample: f32,
}

#[derive(Module)]
#[module("sah", "Sample and Hold")]
#[args(input, trigger)]
pub struct SampleAndHold {
    outputs: SampleAndHoldOutputs,
    params: SampleAndHoldParams,
    last_trigger: f32,
    held_value: f32,
}

impl Default for SampleAndHold {
    fn default() -> Self {
        Self {
            outputs: SampleAndHoldOutputs { sample: 0.0 },
            params: Default::default(),
            last_trigger: 0.0,
            held_value: 0.0,
        }
    }
}

impl SampleAndHold {
    pub fn update(&mut self, _sample_rate: f32) {
        let input = self.params.input.get_value();
        let trigger = self.params.trigger.get_value();

        if trigger > 0.1 && self.last_trigger <= 0.1 {
            self.held_value = input;
        }
        self.last_trigger = trigger;

        self.outputs.sample = self.held_value;
    }
}

message_handlers!(impl SampleAndHold {});

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default)]
struct TrackAndHoldParams {
    input: Signal,
    gate: Signal,
}

#[derive(Outputs, JsonSchema)]
struct TrackAndHoldOutputs {
    #[output("output", "output", default)]
    sample: f32,
}

#[derive(Module)]
#[module("tah", "Track and Hold")]
#[args(input, gate)]
pub struct TrackAndHold {
    outputs: TrackAndHoldOutputs,
    params: TrackAndHoldParams,
    last_gate: f32,
    held_value: f32,
}

impl Default for TrackAndHold {
    fn default() -> Self {
        Self {
            outputs: TrackAndHoldOutputs { sample: 0.0 },
            params: Default::default(),
            last_gate: 0.0,
            held_value: 0.0,
        }
    }
}

impl TrackAndHold {
    pub fn update(&mut self, _sample_rate: f32) {
        let input = self.params.input.get_value();
        let gate = self.params.gate.get_value();

        if gate > 2.5 {
            if self.last_gate <= 2.5 {
                // Just opened gate
                self.held_value = input;
            }
            self.outputs.sample = self.held_value;
        } else {
            // Gate is low, track input
            self.held_value = input;
        }
    }
}

message_handlers!(impl TrackAndHold {});
