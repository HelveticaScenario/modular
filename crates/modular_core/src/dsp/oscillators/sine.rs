use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    dsp::{
        consts::{LUT_SINE, LUT_SINE_SIZE},
        utils::{interpolate, wrap},
    },
    poly::{PORT_MAX_CHANNELS, PolyOutput, PolySignal},
    types::Clickless,
};

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default)]
struct SineOscillatorParams {
    /// frequency in v/oct
    freq: PolySignal,
    /// the phase of the oscillator, overrides freq if present
    phase: PolySignal,
    /// sync input (expects >0V to trigger)
    sync: PolySignal,
}

#[derive(Outputs, JsonSchema)]
struct SineOscillatorOutputs {
    #[output("output", "signal output", range = (-1.0, 1.0))]
    sample: PolyOutput,
    #[output("phaseOut", "current phase output", range = (0.0, 1.0))]
    phase_out: PolyOutput,
}

/// Per-channel oscillator state
#[derive(Default, Clone, Copy)]
struct ChannelState {
    phase: f32,
    freq: Clickless,
}

#[derive(Module)]
#[module("sine", "A sine wave oscillator")]
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

        let mut output = PolyOutput::default();
        let mut phase_out = PolyOutput::default();
        output.set_channels(num_channels as u8);
        phase_out.set_channels(num_channels as u8);

        for ch in 0..num_channels {
            let state = &mut self.channels[ch];

            if !self.params.phase.is_disconnected() {
                // Phase override mode - read phase directly with cycling
                state.phase = wrap(0.0..1.0, self.params.phase.get_value(ch));
                let sine = interpolate(LUT_SINE, state.phase, LUT_SINE_SIZE);
                output.set(ch, sine);
            } else {
                // Frequency mode - get freq for this channel with cycling
                let freq_val = self.params.freq.get_value_or(ch, 4.0).clamp(-10.0, 10.0);
                state.freq.update(freq_val);
                let frequency = 55.0f32 * 2.0f32.powf(*state.freq) / sample_rate;
                state.phase += frequency;
                while state.phase >= 1.0 {
                    state.phase -= 1.0;
                }
                let sine = interpolate(LUT_SINE, state.phase, LUT_SINE_SIZE);
                output.set(ch, sine);
            }

            phase_out.set(ch, state.phase);
        }

        self.outputs.sample = output;
        self.outputs.phase_out = phase_out;
    }
}

message_handlers!(impl SineOscillator {});
