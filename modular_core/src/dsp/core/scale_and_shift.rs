use anyhow::{Result, anyhow};
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
pub struct ScaleAndShift {
    outputs: ScaleAndShiftOutputs,
    scale: Clickless,
    shift: Clickless,
    params: ScaleAndShiftParams,
}

impl ScaleAndShift {
    fn update(&mut self, _sample_rate: f32) -> () {
        let input = self.params.input.get_value();
        self.scale.update(self.params.scale.get_value_or(5.0));
        self.shift.update(self.params.shift.get_value());
        self.outputs.sample = input * (*self.scale / 5.0) + *self.shift;
    }
}

message_handlers!(impl ScaleAndShift {});
