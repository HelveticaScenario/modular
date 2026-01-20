use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    dsp::{consts::{LUT_SINE, LUT_SINE_SIZE}, utils::{interpolate, wrap}},
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
    sample: f32,
    #[output("phaseOut", "current phase output")]
    phase_out: f32,
}

#[derive(Default, Module)]
#[module("sine", "A sine wave oscillator")]
#[args(freq)]
pub struct SineOscillator {
    outputs: SineOscillatorOutputs,
    phase: f32,
    freq: Clickless,
    params: SineOscillatorParams,
}

impl SineOscillator {
    fn update(&mut self, sample_rate: f32) -> () {
        let min = self.params.range.0.get_value_or(-5.0);
        let max = self.params.range.1.get_value_or(5.0);

        if self.params.phase != Signal::Disconnected {
            self.phase = wrap(0.0..1.0, self.params.phase.get_value());
            self.outputs.sample = crate::dsp::utils::map_range(self.phase, 0.0, 1.0, min, max);
        } else {
            self.freq.update(self.params.freq.get_value_or(4.0).clamp(-10.0, 10.0));
            let frequency = 55.0f32 * 2.0f32.powf(*self.freq) / sample_rate;
            self.phase += frequency;
            while self.phase >= 1.0 {
                self.phase -= 1.0;
            }
            let sine = interpolate(LUT_SINE, self.phase, LUT_SINE_SIZE);
            self.outputs.sample = crate::dsp::utils::map_range(sine, -1.0, 1.0, min, max);
        }

        self.outputs.phase_out = self.phase;
    }
}

message_handlers!(impl SineOscillator {});
