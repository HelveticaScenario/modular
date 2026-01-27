use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::poly::{PORT_MAX_CHANNELS, PolyOutput, PolySignal};
use crate::types::Clickless;

#[derive(Deserialize, Default, JsonSchema, Connect)]
#[serde(default)]
struct ScaleAndShiftParams {
    /// signal input
    input: PolySignal,
    /// scale factor
    scale: PolySignal,
    /// shift amount
    shift: PolySignal,
}

#[derive(Outputs, JsonSchema)]
struct ScaleAndShiftOutputs {
    #[output("output", "signal output", default)]
    sample: PolyOutput,
}

#[derive(Default, Module)]
#[module("scaleAndShift", "attenuate, invert, offset")]
#[args(input, scale?, shift?)]
pub struct ScaleAndShift {
    outputs: ScaleAndShiftOutputs,
    scale: [Clickless; PORT_MAX_CHANNELS],
    shift: [Clickless; PORT_MAX_CHANNELS],
    params: ScaleAndShiftParams,
}

impl ScaleAndShift {
    fn update(&mut self, _sample_rate: f32) -> () {
        let input = &self.params.input;
        let channels = PolySignal::max_channels(&[input, &self.params.scale, &self.params.shift]);

        self.outputs.sample.set_channels(channels);

        for i in 0..channels as usize {
            let input_val = input.get_value(i);
            let scale_val = self.params.scale.get_value_or(i, 5.0);
            let shift_val = self.params.shift.get_value_or(i, 0.0);

            self.scale[i].update(scale_val);
            self.shift[i].update(shift_val);

            self.outputs
                .sample
                .set(i, input_val * (*self.scale[i] / 5.0) + *self.shift[i]);
        }
    }
}

message_handlers!(impl ScaleAndShift {});
