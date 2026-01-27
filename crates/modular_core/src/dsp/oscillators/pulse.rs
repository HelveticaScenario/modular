use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::types::{Clickless, Signal};

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default)]
struct PulseOscillatorParams {
    /// frequency in v/oct
    freq: Signal,
    /// pulse width (0-5, 2.5 is square)
    width: Signal,
    /// pulse width modulation input
    pwm: Signal,
    /// output range (min, max)
    /// @param min - minimum output value
    /// @param max - maximum output value
    range: (Signal, Signal),
}

#[derive(Outputs, JsonSchema)]
struct PulseOscillatorOutputs {
    #[output("output", "signal output", default)]
    sample: f32,
}

#[derive(Default, Module)]
#[module("pulse", "Pulse/Square oscillator with PWM")]
#[args(freq)]
pub struct PulseOscillator {
    outputs: PulseOscillatorOutputs,
    phase: f32,
    freq: Clickless,
    width: Clickless,
    params: PulseOscillatorParams,
}

impl PulseOscillator {
    fn update(&mut self, sample_rate: f32) -> () {
        self.freq
            .update(self.params.freq.get_value_or(4.0).clamp(-10.0, 10.0));
        let base_width = self.params.width.get_value_or(2.5);
        let pwm = self.params.pwm.get_value_or(0.0);
        self.width.update((base_width + pwm).clamp(0.0, 5.0));

        let frequency = 55.0f32 * 2.0f32.powf(*self.freq);
        let phase_increment = frequency / sample_rate;

        // Pulse width (0.0 to 1.0, 0.5 is square wave)
        let pulse_width = (self.width / 5.0).clamp(0.01, 0.99);

        self.phase += phase_increment;

        // Wrap phase
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        // Naive pulse wave
        let mut naive_pulse = if self.phase < pulse_width { 1.0 } else { -1.0 };

        // Apply PolyBLEP at the rising edge (phase = 0)
        naive_pulse += poly_blep_pulse(self.phase, phase_increment);

        // Apply PolyBLEP at the falling edge (phase = pulse_width)
        naive_pulse -= poly_blep_pulse(
            if self.phase >= pulse_width {
                self.phase - pulse_width
            } else {
                self.phase - pulse_width + 1.0
            },
            phase_increment,
        );

        let min = self.params.range.0.get_value_or(-5.0);
        let max = self.params.range.1.get_value_or(5.0);
        self.outputs.sample = crate::dsp::utils::map_range(naive_pulse, -1.0, 1.0, min, max);
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
