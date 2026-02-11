use crate::{
    PORT_MAX_CHANNELS,
    poly::{PolyOutput, PolySignal},
};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default, rename_all = "camelCase")]
struct RisingEdgeDetectorParams {
    input: PolySignal,
}

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default, rename_all = "camelCase")]
struct FallingEdgeDetectorParams {
    input: PolySignal,
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct EdgeDetectorOutputs {
    #[output("output", "gate", default)]
    output: PolyOutput,
}

#[derive(Default, Clone, Copy)]
struct EdgeChannelState {
    last_input: f32,
}

#[module(name = "$rising", description = "Rising Edge Detector", args(input))]
#[derive(Default)]
pub struct RisingEdgeDetector {
    outputs: EdgeDetectorOutputs,
    params: RisingEdgeDetectorParams,
    channels: [EdgeChannelState; PORT_MAX_CHANNELS],
}

impl RisingEdgeDetector {
    pub fn update(&mut self, _sample_rate: f32) {
        let num_channels = self.channel_count();

        for ch in 0..num_channels {
            let state = &mut self.channels[ch];
            let input = self.params.input.get_value_or(ch, 0.0);

            let output = if input > state.last_input { 5.0 } else { 0.0 };

            state.last_input = input;
            self.outputs.output.set(ch, output);
        }
    }
}

message_handlers!(impl RisingEdgeDetector {});

#[module(name = "$falling", description = "Falling Edge Detector", args(input))]
#[derive(Default)]
pub struct FallingEdgeDetector {
    outputs: EdgeDetectorOutputs,
    params: FallingEdgeDetectorParams,
    channels: [EdgeChannelState; PORT_MAX_CHANNELS],
}

impl FallingEdgeDetector {
    pub fn update(&mut self, _sample_rate: f32) {
        let num_channels = self.channel_count();

        for ch in 0..num_channels {
            let state = &mut self.channels[ch];
            let input = self.params.input.get_value_or(ch, 0.0);

            let output = if input < state.last_input { 5.0 } else { 0.0 };

            state.last_input = input;
            self.outputs.output.set(ch, output);
        }
    }
}

message_handlers!(impl FallingEdgeDetector {});
