use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    PORT_MAX_CHANNELS,
    dsp::utils::voct_to_hz,
    poly::{PolyOutput, PolySignal},
    types::Clickless,
};

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default, rename_all = "camelCase")]
struct PulseOscillatorParams {
    /// frequency in v/oct
    freq: PolySignal,
    /// pulse width (0-5, 2.5 is square)
    width: PolySignal,
    /// pulse width modulation input
    pwm: PolySignal,
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct PulseOscillatorOutputs {
    #[output("output", "signal output", default, range = (-5.0, 5.0))]
    sample: PolyOutput,
}

#[derive(Default, Clone, Copy)]
struct PulseChannelState {
    phase: f32,
    freq: f32,
    width: Clickless,
}

#[module(name = "pulse", description = "Pulse/Square oscillator with PWM", args(freq))]
#[derive(Default)]
pub struct PulseOscillator {
    outputs: PulseOscillatorOutputs,
    channels: [PulseChannelState; PORT_MAX_CHANNELS],
    params: PulseOscillatorParams,
}

impl PulseOscillator {
    fn update(&mut self, sample_rate: f32) {
        let num_channels = self.channel_count();

        for ch in 0..num_channels {
            let state = &mut self.channels[ch];

            let base_width = self.params.width.get_value_or(ch, 2.5);
            let pwm = self.params.pwm.get_value_or(ch, 0.0);
            state.width.update((base_width + pwm).clamp(0.0, 5.0));

            let frequency = voct_to_hz(self.params.freq.get_value_or(ch, 0.0));
            let phase_increment = frequency / sample_rate;

            // Pulse width (0.0 to 1.0, 0.5 is square wave)
            let pulse_width = (*state.width / 5.0).clamp(0.01, 0.99);

            state.phase += phase_increment;

            // Wrap phase
            if state.phase >= 1.0 {
                state.phase -= 1.0;
            }

            // Naive pulse wave
            let mut naive_pulse = if state.phase < pulse_width { 1.0 } else { -1.0 };

            // Apply PolyBLEP at the rising edge (phase = 0)
            naive_pulse += poly_blep_pulse(state.phase, phase_increment);

            // Apply PolyBLEP at the falling edge (phase = pulse_width)
            naive_pulse -= poly_blep_pulse(
                if state.phase >= pulse_width {
                    state.phase - pulse_width
                } else {
                    state.phase - pulse_width + 1.0
                },
                phase_increment,
            );

            self.outputs.sample.set(ch, naive_pulse * 5.0);
        }
    }
}

// PolyBLEP for pulse wave
fn poly_blep_pulse(phase: f32, phase_increment: f32) -> f32 {
    // Detect discontinuity at phase wrap (0.0)
    if phase < phase_increment {
        let t = phase / phase_increment;
        return t + t - t * t - 1.0;
    }
    // Detect discontinuity approaching 1.0
    else if phase > 1.0 - phase_increment {
        let t = (phase - 1.0) / phase_increment;
        return t * t + t + t + 1.0;
    }
    0.0
}

message_handlers!(impl PulseOscillator {});
