use crate::{
    dsp::utils::wrap,
    poly::{PORT_MAX_CHANNELS, PolyOutput, PolySignal},
    types::Clickless,
};
use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default)]
struct DPulseOscillatorParams {
    /// phase input (0-1, will be wrapped)
    phase: PolySignal,
    /// pulse width (0-5, 2.5 is square)
    width: PolySignal,
    /// pulse width modulation input
    pwm: PolySignal,
}

#[derive(Outputs, JsonSchema)]
struct DPulseOscillatorOutputs {
    #[output("output", "signal output", default, range = (-1.0, 1.0))]
    sample: PolyOutput,
}

/// Per-channel state for width smoothing
#[derive(Default, Clone, Copy)]
struct ChannelState {
    width: Clickless,
}

#[derive(Module)]
#[module("dPulse", "A phase-driven pulse/square oscillator with PWM")]
#[args(phase)]
pub struct DPulseOscillator {
    outputs: DPulseOscillatorOutputs,
    channels: [ChannelState; PORT_MAX_CHANNELS],
    params: DPulseOscillatorParams,
}

impl Default for DPulseOscillator {
    fn default() -> Self {
        Self {
            outputs: DPulseOscillatorOutputs::default(),
            channels: [ChannelState::default(); PORT_MAX_CHANNELS],
            params: DPulseOscillatorParams::default(),
        }
    }
}

impl DPulseOscillator {
    fn update(&mut self, _sample_rate: f32) {
        let num_channels = self.channel_count();

        let mut output = PolyOutput::default();
        output.set_channels(num_channels as u8);

        for ch in 0..num_channels {
            let state = &mut self.channels[ch];

            let base_width = self.params.width.get_value_or(ch, 2.5);
            let pwm = self.params.pwm.get_value_or(ch, 0.0);
            state.width.update((base_width + pwm).clamp(0.0, 5.0));

            let phase = wrap(0.0..1.0, self.params.phase.get_value(ch));

            // Pulse width (0.0 to 1.0, 0.5 is square wave)
            let pulse_width = (*state.width / 5.0).clamp(0.01, 0.99);

            // Naive pulse wave (no anti-aliasing)
            let pulse = if phase < pulse_width { 1.0 } else { -1.0 };

            output.set(ch, pulse);
        }

        self.outputs.sample = output;
    }
}

message_handlers!(impl DPulseOscillator {});
