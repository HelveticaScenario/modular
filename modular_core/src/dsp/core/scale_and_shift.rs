use anyhow::{Result, anyhow};

use crate::types::{ChannelBuffer, InternalParam, NUM_CHANNELS};

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
#[module("scaleAndShift", "attenuate, invert, offset")]
pub struct ScaleAndShift {
    #[output("output", "signal output", default)]
    sample: ChannelBuffer,
    smoothed_scale: ChannelBuffer,
    smoothed_shift: ChannelBuffer,
    params: ScaleAndShiftParams,
}

impl ScaleAndShift {
    fn update(&mut self, _sample_rate: f32) -> () {
        let mut input = ChannelBuffer::default();
        let mut target_scale = [5.0; NUM_CHANNELS];
        let mut target_shift = ChannelBuffer::default();

        self.params.input.get_value(&mut input);
        self.params.scale.get_value_or(&mut target_scale, &[5.0; NUM_CHANNELS]);
        self.params.shift.get_value(&mut target_shift);

        crate::types::smooth_buffer(&mut self.smoothed_scale, &target_scale);
        crate::types::smooth_buffer(&mut self.smoothed_shift, &target_shift);

        for i in 0..NUM_CHANNELS {
            self.sample[i] = input[i] * (self.smoothed_scale[i] / 5.0) + self.smoothed_shift[i];
        }
    }
}
