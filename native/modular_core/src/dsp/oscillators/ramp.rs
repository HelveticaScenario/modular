use anyhow::{anyhow, Result};

use crate::{dsp::utils::clamp, dsp::utils::wrap, types::InternalParam};
#[derive(Default, Params)]
struct RampOscillatorParams {
    #[param("freq", "frequency in v/oct")]
    freq: InternalParam,
    #[param("phase", "the phase of the oscillator, overrides freq if present")]
    phase: InternalParam,
}

#[derive(Default, Module)]
#[module("ramp-oscillator", "A ramp oscillator")]
pub struct RampOscillator {
    #[output("output", "signal output")]
    sample: f32,
    phase: f32,
    params: RampOscillatorParams,
}

impl RampOscillator {
    fn update(&mut self, sample_rate: f32) -> () {
        if self.params.phase != InternalParam::Disconnected {
            self.sample = wrap(0.0..1.0, self.params.phase.get_value());
        } else {
            let voltage = clamp(self.params.freq.get_value_or(4.0), 12.0, 0.0);
            let frequency = 27.5f32 * 2.0f32.powf(voltage) / sample_rate;
            // let frequency = semitones_to_ratio(voltage * 12.0) * 220.0 / SAMPLE_RATE * 100.0;
            self.phase += frequency;
            if self.phase >= 1.0 {
                self.phase -= 1.0;
            }
            self.sample = 5.0 * self.phase;
        }
    }
}
