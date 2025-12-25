use anyhow::{anyhow, Result};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::types::Signal;

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
    smoothed_scale: f32,
    smoothed_shift: f32,
    params: ScaleAndShiftParams,
}

impl ScaleAndShift {
    fn update(&mut self, _sample_rate: f32) -> () {
        let input = self.params.input.get_value();
        let target_scale = self.params.scale.get_value_or(5.0);
        let target_shift = self.params.shift.get_value();
        self.smoothed_scale = crate::types::smooth_value(self.smoothed_scale, target_scale);
        self.smoothed_shift = crate::types::smooth_value(self.smoothed_shift, target_shift);
        self.outputs.sample = input * (self.smoothed_scale / 5.0) + self.smoothed_shift
    }
}
