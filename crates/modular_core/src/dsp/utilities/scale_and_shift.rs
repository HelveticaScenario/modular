use schemars::JsonSchema;
use serde::Deserialize;

use crate::poly::{PolyOutput, PolySignal, PORT_MAX_CHANNELS};
use crate::types::Clickless;

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default, rename_all = "camelCase")]
struct ScaleAndShiftParams {
    /// signal to scale and shift
    input: PolySignal,
    /// scale factor (0–10V range; 5V = unity gain, 0V = silence, -5V = inverted, 10V = 2x)
    scale: PolySignal,
    /// DC offset added to the scaled signal (in volts)
    shift: PolySignal,
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ScaleAndShiftOutputs {
    #[output("output", "signal output", default)]
    sample: PolyOutput,
}

/// Scales and offsets a signal — the classic attenuverter + DC offset.
///
/// - **scale** — gain factor (0–10 V; 5 V = unity, 0 V = silence,
///   values above 5 V amplify, negative values invert).
/// - **shift** — DC offset added after scaling (in volts).
///
/// ```js
/// // invert a slow sine and shift it into 0–5 V range
/// $scaleAndShift($sine('1hz'), -5, 2.5)
/// ```
#[module(name = "$scaleAndShift", args(input, scale?, shift?))]
#[derive(Default)]
pub struct ScaleAndShift {
    outputs: ScaleAndShiftOutputs,
    scale: [f32; PORT_MAX_CHANNELS],
    shift: [f32; PORT_MAX_CHANNELS],
    params: ScaleAndShiftParams,
}

impl ScaleAndShift {
    fn update(&mut self, _sample_rate: f32) {
        let channels = self.channel_count();

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
