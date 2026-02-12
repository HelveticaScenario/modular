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
#[serde(default, rename_all = "camelCase")]
struct SineOscillatorParams {
    /// frequency in v/oct
    freq: PolySignal,
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct SineOscillatorOutputs {
    #[output("output", "signal output", default, range = (-5.0, 5.0))]
    sample: PolyOutput,
}

/// Per-channel oscillator state
#[derive(Default, Clone, Copy)]
struct ChannelState {
    phase: f32,
}

/// A basic sine wave oscillator using wavetable lookup.
///
/// The `freq` input follows the **V/Oct** standard where 0V corresponds
/// to C4 (~261.63 Hz). Each additional volt doubles the frequency.
///
/// Outputs a clean sine wave in the range **±5V**.
///
/// ## Example
///
/// ```js
/// sine(note("C4")).out();
/// ```
#[module(name = "$sine", description = "A sine wave oscillator", args(freq))]
#[derive(Default)]
pub struct SineOscillator {
    outputs: SineOscillatorOutputs,
    channels: [ChannelState; PORT_MAX_CHANNELS],
    params: SineOscillatorParams,
}

impl SineOscillator {
    fn update(&mut self, sample_rate: f32) {
        let num_channels = self.channel_count();

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
