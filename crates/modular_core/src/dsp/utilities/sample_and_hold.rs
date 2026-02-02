use crate::{
    PORT_MAX_CHANNELS,
    poly::{PolyOutput, PolySignal},
    types::Signal,
};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default)]
struct SampleAndHoldParams {
    input: PolySignal,
    trigger: PolySignal,
}

#[derive(Outputs, JsonSchema)]
struct SampleAndHoldOutputs {
    #[output("output", "output", default)]
    sample: PolyOutput,
}

#[derive(Default, Clone, Copy)]
struct SahChannelState {
    last_trigger: f32,
    held_value: f32,
    initialized: bool,
}

#[derive(Module)]
#[module("sah", "Sample and Hold")]
#[args(input, trigger)]
pub struct SampleAndHold {
    outputs: SampleAndHoldOutputs,
    params: SampleAndHoldParams,
    channels: [SahChannelState; PORT_MAX_CHANNELS],
}

impl Default for SampleAndHold {
    fn default() -> Self {
        Self {
            outputs: Default::default(),
            params: Default::default(),
            channels: [SahChannelState::default(); PORT_MAX_CHANNELS],
        }
    }
}

impl SampleAndHold {
    pub fn update(&mut self, _sample_rate: f32) {
        let num_channels = self.channel_count();
        self.outputs.sample.set_channels(num_channels);

        for ch in 0..num_channels {
            let state = &mut self.channels[ch];
            let input = self.params.input.get_value_or(ch, 0.0);
            let trigger = self.params.trigger.get_value_or(ch, 0.0);

            if !state.initialized {
                state.held_value = input;
                state.initialized = true;
            } else if trigger > 0.1 && state.last_trigger <= 0.1 {
                state.held_value = input;
            }
            state.last_trigger = trigger;

            self.outputs.sample.set(ch, state.held_value);
        }
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
