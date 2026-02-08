use crate::dsp::utils::{SchmittState, SchmittTrigger};
use crate::{
    PORT_MAX_CHANNELS,
    poly::{PolyOutput, PolySignal},
};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default, rename_all = "camelCase")]
struct SampleAndHoldParams {
    input: PolySignal,
    trigger: PolySignal,
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct SampleAndHoldOutputs {
    #[output("output", "output", default)]
    sample: PolyOutput,
}

#[derive(Default, Clone, Copy)]
struct SampleAndHoldChannelState {
    trigger: SchmittTrigger,
    held_value: f32,
}

#[module(
    name = "sah",
    description = "Sample and Hold",
    args(input, trigger)
)]
#[derive(Default)]
pub struct SampleAndHold {
    outputs: SampleAndHoldOutputs,
    params: SampleAndHoldParams,
    channels: [SampleAndHoldChannelState; PORT_MAX_CHANNELS],
}

impl SampleAndHold {
    pub fn update(&mut self, _sample_rate: f32) {
        let num_channels = self.channel_count();

        for ch in 0..num_channels {
            let state = &mut self.channels[ch];
            let input = self.params.input.get_value(ch);
            let trigger = self.params.trigger.get_value(ch);

            if state.trigger.state == SchmittState::Uninitialized {
                state.held_value = input;
            }

            // Make sure to initialize the held value on the first update
            if state.trigger.process(trigger) {
                state.held_value = input;
            }

            self.outputs.sample.set(ch, state.held_value);
        }
    }
}

message_handlers!(impl SampleAndHold {});

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default, rename_all = "camelCase")]
struct TrackAndHoldParams {
    input: PolySignal,
    gate: PolySignal,
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct TrackAndHoldOutputs {
    #[output("output", "output", default)]
    sample: PolyOutput,
}

#[derive(Default)]
struct TrackAndHoldChannelState {
    gate: SchmittTrigger,
}

#[module(name = "tah", description = "Track and Hold", args(input, gate))]
#[derive(Default)]
pub struct TrackAndHold {
    outputs: TrackAndHoldOutputs,
    params: TrackAndHoldParams,
    channels: [TrackAndHoldChannelState; PORT_MAX_CHANNELS],
}

impl TrackAndHold {
    pub fn update(&mut self, _sample_rate: f32) {
        let num_channels = self.channel_count();

        for ch in 0..num_channels {
            let state = &mut self.channels[ch];
            let input = self.params.input.get_value(ch);
            let gate = self.params.gate.get_value(ch);

            // Track while gate is low or on rising edge
            state.gate.process(gate);
            if state.gate.state() != crate::dsp::utils::SchmittState::High {
                self.outputs.sample.set(ch, input);
            }
        }
    }
}

message_handlers!(impl TrackAndHold {});
