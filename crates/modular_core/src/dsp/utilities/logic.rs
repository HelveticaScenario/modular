use crate::types::Signal;
use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default)]
struct RisingEdgeDetectorParams {
    input: Signal,
}

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default)]
struct FallingEdgeDetectorParams {
    input: Signal,
}

#[derive(Outputs, JsonSchema)]
struct EdgeDetectorOutputs {
    #[output("output", "gate", default)]
    output: f32,
}

#[derive(Module)]
#[module("rising", "Rising Edge Detector")]
#[args(input)]
pub struct RisingEdgeDetector {
    outputs: EdgeDetectorOutputs,
    params: RisingEdgeDetectorParams,
    last_input: f32,
}

impl Default for RisingEdgeDetector {
    fn default() -> Self {
        Self {
            outputs: EdgeDetectorOutputs { output: 0.0 },
            params: Default::default(),
            last_input: 0.0,
        }
    }
}

impl RisingEdgeDetector {
    pub fn update(&mut self, _sample_rate: f32) {
        let input = self.params.input.get_value();

        let output = if input > self.last_input { 5.0 } else { 0.0 };

        self.last_input = input;
        self.outputs.output = output;
    }
}

message_handlers!(impl RisingEdgeDetector {});

#[derive(Module)]
#[module("falling", "Falling Edge Detector")]
#[args(input)]
pub struct FallingEdgeDetector {
    outputs: EdgeDetectorOutputs,
    params: FallingEdgeDetectorParams,
    last_input: f32,
}

impl Default for FallingEdgeDetector {
    fn default() -> Self {
        Self {
            outputs: EdgeDetectorOutputs { output: 0.0 },
            params: Default::default(),
            last_input: 0.0,
        }
    }
}

impl FallingEdgeDetector {
    pub fn update(&mut self, _sample_rate: f32) {
        let input = self.params.input.get_value();

        let output = if input < self.last_input { 5.0 } else { 0.0 };

        self.last_input = input;
        self.outputs.output = output;
    }
}

message_handlers!(impl FallingEdgeDetector {});
