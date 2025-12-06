use anyhow::{Result, anyhow};

use crate::{
    dsp::utils::wrap,
    dsp::{
        consts::{LUT_SINE, LUT_SINE_SIZE},
        utils::{clamp, interpolate},
    },
    types::InternalParam,
};

#[derive(Default, Params)]
struct SineOscillatorParams {
    #[param("freq", "frequency in v/oct")]
    freq: InternalParam,
    #[param("phase", "the phase of the oscillator, overrides freq if present")]
    phase: InternalParam,
}

#[derive(Default, Module)]
#[module("sine-osc", "A sine wave oscillator")]
pub struct SineOscillator {
    #[output("output", "signal output", default)]
    sample: f32,
    phase: f32,
    smoothed_freq: f32,
    params: SineOscillatorParams,
}

impl SineOscillator {
    fn update(&mut self, sample_rate: f32) -> () {
        if self.params.phase != InternalParam::Disconnected {
            self.sample = wrap(0.0..1.0, self.params.phase.get_value())
        } else {
            let target_freq = clamp(-10.0, 10.0, self.params.freq.get_value_or(4.0));
            self.smoothed_freq = crate::types::smooth_value(self.smoothed_freq, target_freq);
            let voltage = self.smoothed_freq;
            let frequency = 27.5f32 * 2.0f32.powf(voltage) / sample_rate;
            self.phase += frequency;
            while self.phase >= 1.0 {
                self.phase -= 1.0;
            }
            self.sample = 5.0 * interpolate(LUT_SINE, self.phase, LUT_SINE_SIZE);
        }
    }
}
