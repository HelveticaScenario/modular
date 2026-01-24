use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::types::{Clickless, Signal};

#[derive(Deserialize, Default, JsonSchema, Connect)]
#[serde(default)]
struct ScaleAndShiftParams {
    /// signal input
    input: Signal,
    /// scale factor
    scale: Signal,
    /// shift amount
    shift: Signal,
}

#[derive(Outputs, JsonSchema)]
struct ScaleAndShiftOutputs {
    #[output("output", "signal output", default)]
    sample: f32,
}

#[derive(Default, Module)]
#[module("scaleAndShift", "attenuate, invert, offset")]
#[args(input, scale?, shift?)]
pub struct ScaleAndShift {
    outputs: ScaleAndShiftOutputs,
    scale: Clickless,
    shift: Clickless,
    params: ScaleAndShiftParams,
}

impl ScaleAndShift {
    fn update(&mut self, _sample_rate: f32) -> () {
        let input = self.params.input.get_poly_signal().get(0);
        self.scale.update(self.params.scale.get_poly_signal().get_or(0, 5.0));
        self.shift.update(self.params.shift.get_poly_signal().get(0));
        self.outputs.sample = input * (*self.scale / 5.0) + *self.shift;
    }
}

message_handlers!(impl ScaleAndShift {});
