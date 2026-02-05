use crate::{
    dsp::{
        consts::{LUT_SINE, LUT_SINE_SIZE},
        utils::{interpolate, wrap},
    },
    poly::{PolyOutput, PolySignal},
};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default)]
struct DSineOscillatorParams {
    /// phase input (0-1, will be wrapped)
    phase: PolySignal,
}

#[derive(Outputs, JsonSchema)]
struct DSineOscillatorOutputs {
    #[output("output", "signal output", default, range = (-5.0, 5.0))]
    sample: PolyOutput,
}

#[derive(Module)]
#[module("osc.dSine", "A phase-driven sine wave oscillator")]
#[args(phase)]
#[derive(Default)]
pub struct DSineOscillator {
    outputs: DSineOscillatorOutputs,
    params: DSineOscillatorParams,
}


impl DSineOscillator {
    fn update(&mut self, _sample_rate: f32) {
        let num_channels = self.channel_count();

        self.outputs.sample.set_channels(num_channels);

        for ch in 0..num_channels {
            let phase = wrap(0.0..1.0, self.params.phase.get_value(ch));
            let sine = interpolate(LUT_SINE, phase, LUT_SINE_SIZE);
            self.outputs.sample.set(ch, sine * 5.0);
        }
    }
}

message_handlers!(impl DSineOscillator {});
