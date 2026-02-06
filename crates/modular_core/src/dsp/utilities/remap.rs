use schemars::JsonSchema;
use serde::Deserialize;

use crate::poly::{PORT_MAX_CHANNELS, PolyOutput, PolySignal};
use crate::types::Clickless;

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default, rename_all = "camelCase")]
struct RemapParams {
    /// signal input to remap
    input: PolySignal,
    /// minimum of input range
    in_min: PolySignal,
    /// maximum of input range
    in_max: PolySignal,
    /// minimum of output range
    out_min: PolySignal,
    /// maximum of output range
    out_max: PolySignal,
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct RemapOutputs {
    #[output("output", "remapped signal output", default)]
    sample: PolyOutput,
}

#[derive(Default)]
#[module(name = "util.remap", description = "remap a signal from one range to another", args(input, inMin?, inMax?, outMin?, outMax?))]
pub struct Remap {
    outputs: RemapOutputs,
    in_min: [f32; PORT_MAX_CHANNELS],
    in_max: [f32; PORT_MAX_CHANNELS],
    out_min: [f32; PORT_MAX_CHANNELS],
    out_max: [f32; PORT_MAX_CHANNELS],
    params: RemapParams,
}

impl Remap {
    fn update(&mut self, _sample_rate: f32) {
        let channels = self.channel_count();

        self.outputs.sample.set_channels(channels);

        for i in 0..channels as usize {
            let input_val = self.params.input.get_value(i);

            // Get range parameters with defaults
            let in_min_val = self.params.in_min.get_value_or(i, -1.0);
            let in_max_val = self.params.in_max.get_value_or(i, 1.0);
            let out_min_val = self.params.out_min.get_value_or(i, -5.0);
            let out_max_val = self.params.out_max.get_value_or(i, 5.0);

            // Smooth parameters to avoid clicks
            self.in_min[i] = in_min_val;
            self.in_max[i] = in_max_val;
            self.out_min[i] = out_min_val;
            self.out_max[i] = out_max_val;

            // Apply remapping using map_range utility
            let output = crate::dsp::utils::map_range(
                input_val,
                self.in_min[i],
                self.in_max[i],
                self.out_min[i],
                self.out_max[i],
            );

            self.outputs.sample.set(i, output);
        }
    }
}

message_handlers!(impl Remap {});
