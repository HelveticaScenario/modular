use anyhow::{Result, anyhow};

use crate::{
    dsp::{
        consts::{LUT_SINE, LUT_SINE_SIZE},
        utils::{clamp, interpolate, wrap},
    },
    types::{ChannelBuffer, InternalParam, NUM_CHANNELS},
};

#[derive(Default, Params)]
struct SineOscillatorParams {
    #[param("freq", "frequency in v/oct")]
    freq: InternalParam,
    #[param("phase", "the phase of the oscillator, overrides freq if present")]
    phase: InternalParam,
}

#[derive(Default, Module)]
#[module("sine", "A sine wave oscillator")]
pub struct SineOscillator {
    #[output("output", "signal output", default)]
    sample: ChannelBuffer,
    phase: ChannelBuffer,
    smoothed_freq: ChannelBuffer,
    params: SineOscillatorParams,
}

impl SineOscillator {
    fn update(&mut self, sample_rate: f32) -> () {
        if self.params.phase != InternalParam::Disconnected {
            let mut phase_in = ChannelBuffer::default();
            self.params.phase.get_value(&mut phase_in);
            for i in 0..NUM_CHANNELS {
                let p = wrap(0.0..1.0, phase_in[i]);
                self.phase[i] = p;
                self.sample[i] = 5.0 * interpolate(LUT_SINE, p, LUT_SINE_SIZE);
            }
            return;
        }

        let mut target_freq = [4.0; NUM_CHANNELS];
        self.params
            .freq
            .get_value_or(&mut target_freq, &[4.0; NUM_CHANNELS]);
        for i in 0..NUM_CHANNELS {
            target_freq[i] = clamp(-10.0, 10.0, target_freq[i]);
        }
        crate::types::smooth_buffer(&mut self.smoothed_freq, &target_freq);

        let sr = sample_rate.max(1.0);
        for i in 0..NUM_CHANNELS {
            let voltage = self.smoothed_freq[i];
            let phase_inc = 27.5f32 * voltage.exp2() / sr;
            self.phase[i] += phase_inc;
            if self.phase[i] >= 1.0 {
                self.phase[i] -= self.phase[i].floor();
            }
            self.sample[i] = 5.0 * interpolate(LUT_SINE, self.phase[i], LUT_SINE_SIZE);
        }
    }
}
