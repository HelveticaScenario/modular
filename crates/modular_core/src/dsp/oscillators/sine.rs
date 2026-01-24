use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    dsp::{
        consts::{LUT_SINE, LUT_SINE_SIZE},
        utils::{interpolate, wrap},
    },
    poly::{PolySignal, PORT_MAX_CHANNELS},
    types::{Clickless, Signal},
};

#[derive(Deserialize, Default, JsonSchema, Connect)]
#[serde(default)]
struct SineOscillatorParams {
    /// frequency in v/oct
    freq: Signal,
    /// the phase of the oscillator, overrides freq if present
    phase: Signal,
    /// sync input (expects >0V to trigger)
    sync: Signal,
    /// @param min - minimum output value
    /// @param max - maximum output value
    range: (Signal, Signal),
}

#[derive(Outputs, JsonSchema)]
struct SineOscillatorOutputs {
    #[output("output", "signal output", default)]
    sample: PolySignal,
    #[output("phaseOut", "current phase output")]
    phase_out: PolySignal,
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
        let min = self.params.range.0.get_poly_signal().get_or(0, -5.0);
        let max = self.params.range.1.get_poly_signal().get_or(0, 5.0);

        // Determine channel count from freq input (or phase if overriding)
        let num_channels = if self.params.phase != Signal::Disconnected {
            self.params.phase.get_poly_signal().channels().max(1) as usize
        } else {
            self.params.freq.get_poly_signal().channels().max(1) as usize
        };

        let mut output = PolySignal::default();
        let mut phase_out = PolySignal::default();
        output.set_channels(num_channels as u8);
        phase_out.set_channels(num_channels as u8);

        let phase_poly = self.params.phase.get_poly_signal();
        let freq_poly = self.params.freq.get_poly_signal();

        for ch in 0..num_channels {
            let state = &mut self.channels[ch];

            if self.params.phase != Signal::Disconnected {
                // Phase override mode - read phase directly with cycling
                state.phase = wrap(0.0..1.0, phase_poly.get_cycling(ch));
                let sine = interpolate(LUT_SINE, state.phase, LUT_SINE_SIZE);
                output.set(ch, crate::dsp::utils::map_range(sine, -1.0, 1.0, min, max));
            } else {
                // Frequency mode - get freq for this channel with cycling
                let freq_val = freq_poly.get_or(ch, 4.0).clamp(-10.0, 10.0);
                state.freq.update(freq_val);
                let frequency = 55.0f32 * 2.0f32.powf(*state.freq) / sample_rate;
                state.phase += frequency;
                while state.phase >= 1.0 {
                    state.phase -= 1.0;
                }
                let sine = interpolate(LUT_SINE, state.phase, LUT_SINE_SIZE);
                output.set(ch, crate::dsp::utils::map_range(sine, -1.0, 1.0, min, max));
            }

            phase_out.set(ch, state.phase);
        }

        self.outputs.sample = output;
        self.outputs.phase_out = phase_out;
    }
}

message_handlers!(impl SineOscillator {});
