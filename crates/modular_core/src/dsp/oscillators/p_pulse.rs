use crate::{
    dsp::utils::wrap,
    poly::{PORT_MAX_CHANNELS, PolyOutput, PolySignal},
    types::Clickless,
};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default, rename_all = "camelCase")]
struct PPulseOscillatorParams {
    /// phasor input (0–1, wraps at boundaries)
    phase: PolySignal,
    /// pulse width (0-5, 2.5 is square)
    width: PolySignal,
    /// pulse width modulation CV — added to the width parameter
    pwm: PolySignal,
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct PPulseOscillatorOutputs {
    #[output("output", "signal output", default, range = (-5.0, 5.0))]
    sample: PolyOutput,
}

/// Per-channel state for width smoothing
#[derive(Default, Clone, Copy)]
struct ChannelState {
    width: Clickless,
}

/// Phase-driven pulse/square oscillator with pulse width modulation.
///
/// Instead of a frequency input, this oscillator is driven by an external
/// phasor signal (0–1). Connect a `ramp` or other phase source to `phase`
/// and use phase-distortion modules between them for complex timbres.
///
/// The `width` parameter sets the duty cycle: 0 = narrow pulse,
/// 2.5 = square wave, 5 = inverted narrow pulse.
/// `pwm` is added to `width` for modulation.
///
/// Output range is **±5V**.
#[module(
    name = "$pPulse",
    description = "A phase-driven pulse/square oscillator with PWM",
    args(phase)
)]
#[derive(Default)]
pub struct PPulseOscillator {
    outputs: PPulseOscillatorOutputs,
    channels: [ChannelState; PORT_MAX_CHANNELS],
    params: PPulseOscillatorParams,
}

impl PPulseOscillator {
    fn update(&mut self, _sample_rate: f32) {
        let num_channels = self.channel_count();

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

            self.outputs.sample.set(ch, pulse * 5.0);
        }
    }
}

message_handlers!(impl PPulseOscillator {});
