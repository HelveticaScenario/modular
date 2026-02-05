use crate::{
    dsp::{
        consts::{LUT_SINE, LUT_SINE_SIZE},
        utils::{interpolate, voct_to_hz},
    },
    poly::{PORT_MAX_CHANNELS, PolyOutput, PolySignal},
};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default)]
struct SineOscillatorParams {
    /// frequency in v/oct
    freq: PolySignal,
}

#[derive(Outputs, JsonSchema)]
struct SineOscillatorOutputs {
    #[output("output", "signal output", default, range = (-5.0, 5.0))]
    sample: PolyOutput,
}

/// Per-channel oscillator state
#[derive(Default, Clone, Copy)]
struct ChannelState {
    phase: f32,
}

#[derive(Module)]
#[module("osc.sine", "A sine wave oscillator")]
#[args(freq)]
pub struct SineOscillator {
    outputs: SineOscillatorOutputs,
    channels: [ChannelState; PORT_MAX_CHANNELS],
    params: SineOscillatorParams,
}

impl Default for SineOscillator {
    fn default() -> Self {
        Self {
            outputs: SineOscillatorOutputs::default(),
            channels: [ChannelState::default(); PORT_MAX_CHANNELS],
            params: SineOscillatorParams::default(),
        }
    }
}

impl SineOscillator {
    fn update(&mut self, sample_rate: f32) {
        let num_channels = self.channel_count();

        self.outputs.sample.set_channels(num_channels);

        for ch in 0..num_channels {
            let state = &mut self.channels[ch];

            let frequency = voct_to_hz(self.params.freq.get_value_or(ch, 0.0)) / sample_rate;
            state.phase += frequency;
            while state.phase >= 1.0 {
                state.phase -= 1.0;
            }
            let sine = interpolate(LUT_SINE, state.phase, LUT_SINE_SIZE);
            self.outputs.sample.set(ch, sine * 5.0);
        }
    }
}

message_handlers!(impl SineOscillator {});
