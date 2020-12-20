use anyhow::{anyhow, Result};

use crate::types::InternalParam;

#[derive(Default, Params)]
struct ScaleAndShiftParams {
    #[param("input", "signal input")]
    input: InternalParam,
    #[param("scale", "scale factor")]
    scale: InternalParam,
    #[param("shift", "shift amount")]
    shift: InternalParam,
}

#[derive(Default, Module)]
#[module("scale-and-shift", "attenuate, invert, offset")]
pub struct ScaleAndShift {
    #[output("output", "signal output")]
    sample: f32,
    params: ScaleAndShiftParams,
}

impl ScaleAndShift {
    fn update(&mut self, _sample_rate: f32) -> () {
        let input = self.params.input.get_value();
        let scale = self.params.scale.get_value_or(5.0);
        let shift = self.params.shift.get_value();
        self.sample = input * (scale / 5.0) + shift
    }
}
