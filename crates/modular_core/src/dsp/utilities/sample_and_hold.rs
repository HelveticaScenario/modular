use crate::{
    PORT_MAX_CHANNELS,
    poly::{MonoSignal, PolyOutput, PolySignal},
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
struct SampleAndHoldChannelState {
    last_trigger: f32,
    held_value: f32,
    initialized: bool,
}

#[derive(Module, Default)]
#[module("util.sah", "Sample and Hold")]
#[args(input, trigger)]
pub struct SampleAndHold {
    outputs: SampleAndHoldOutputs,
    params: SampleAndHoldParams,
    channels: [SampleAndHoldChannelState; PORT_MAX_CHANNELS],
}

impl SampleAndHold {
    pub fn update(&mut self, _sample_rate: f32) {
        let num_channels = self.channel_count();
        self.outputs.sample.set_channels(num_channels);

        for ch in 0..num_channels {
            let state = &mut self.channels[ch];
            let input = self.params.input.get_value(ch);
            let trigger = self.params.trigger.get_value(ch);

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
    input: PolySignal,
    gate: PolySignal,
}

#[derive(Outputs, JsonSchema)]
struct TrackAndHoldOutputs {
    #[output("output", "output", default)]
    sample: PolyOutput,
}

#[derive(Default)]
struct TrackAndHoldChannelState {
    last_gate: f32,
}

#[derive(Module, Default)]
#[module("util.tah", "Track and Hold")]
#[args(input, gate)]
pub struct TrackAndHold {
    outputs: TrackAndHoldOutputs,
    params: TrackAndHoldParams,
    channels: [TrackAndHoldChannelState; PORT_MAX_CHANNELS],
}

impl TrackAndHold {
    pub fn update(&mut self, _sample_rate: f32) {
        let num_channels = self.channel_count();
        self.outputs.sample.set_channels(num_channels);

        for ch in 0..num_channels {
            let state = &mut self.channels[ch];
            let input = self.params.input.get_value(ch);
            let gate = self.params.gate.get_value(ch);

            if (gate > 2.5 && state.last_gate <= 2.5) || gate <= 2.5 {
                self.outputs.sample.set(ch, input);
            }
        }
    }
}

message_handlers!(impl TrackAndHold {});
