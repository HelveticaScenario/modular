use schemars::JsonSchema;
use serde::Deserialize;

use crate::poly::{PORT_MAX_CHANNELS, PolyOutput, PolySignal};
use crate::types::Clickless;

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
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
    scale: [f32; PORT_MAX_CHANNELS],
    shift: [f32; PORT_MAX_CHANNELS],
    params: ScaleAndShiftParams,
}

impl ScaleAndShift {
    fn update(&mut self, _sample_rate: f32) {
        let channels = self.channel_count() as u8;

        self.outputs.sample.set_channels(channels);

        for i in 0..channels as usize {
            let input_val = self.params.input.get_value(i);
            let scale_val = self.params.scale.get_value_or(i, 5.0);
            let shift_val = self.params.shift.get_value_or(i, 0.0);

            self.scale[i] = scale_val;
            self.shift[i] = shift_val;

            self.outputs
                .sample
                .set(i, input_val * (self.scale[i] / 5.0) + self.shift[i]);
        }
    }
}

message_handlers!(impl ScaleAndShift {});
