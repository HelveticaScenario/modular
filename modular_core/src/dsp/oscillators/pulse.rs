use anyhow::{anyhow, Result};
use crate::{dsp::utils::clamp, types::{ChannelBuffer, InternalParam, NUM_CHANNELS}};

#[derive(Default, Params)]
struct PulseOscillatorParams {
    #[param("freq", "frequency in v/oct")]
    freq: InternalParam,
    #[param("width", "pulse width (0-5, 2.5 is square)")]
    width: InternalParam,
    #[param("pwm", "pulse width modulation input")]
    pwm: InternalParam,
}

#[derive(Default, Module)]
#[module("pulse", "Pulse/Square oscillator with PWM")]
pub struct PulseOscillator {
    #[output("output", "signal output", default)]
    sample: ChannelBuffer,
    phase: ChannelBuffer,
    smoothed_freq: ChannelBuffer,
    smoothed_width: ChannelBuffer,
    params: PulseOscillatorParams,
}

impl PulseOscillator {
    fn update(&mut self, sample_rate: f32) -> () {
        let mut target_freq = [4.0; NUM_CHANNELS];
        let mut base_width = [2.5; NUM_CHANNELS];
        let mut pwm = ChannelBuffer::default();

        self.params
            .freq
            .get_value_or(&mut target_freq, &[4.0; NUM_CHANNELS]);
        self.params
            .width
            .get_value_or(&mut base_width, &[2.5; NUM_CHANNELS]);
        self.params.pwm.get_value(&mut pwm);

        for i in 0..NUM_CHANNELS {
            target_freq[i] = clamp(-10.0, 10.0, target_freq[i]);
            let w = (base_width[i] + pwm[i]).clamp(0.0, 5.0);
            base_width[i] = w;
        }

        crate::types::smooth_buffer(&mut self.smoothed_freq, &target_freq);
        crate::types::smooth_buffer(&mut self.smoothed_width, &base_width);

        let sr = sample_rate.max(1.0);
        for i in 0..NUM_CHANNELS {
            let voltage = self.smoothed_freq[i];
            let frequency = 27.5f32 * voltage.exp2();
            let phase_increment = frequency / sr;
            let pulse_width = (self.smoothed_width[i] / 5.0).clamp(0.01, 0.99);

            self.phase[i] += phase_increment;
            if self.phase[i] >= 1.0 {
                self.phase[i] -= self.phase[i].floor();
            }

            let mut naive_pulse = if self.phase[i] < pulse_width { 1.0 } else { -1.0 };
            naive_pulse += poly_blep_pulse(self.phase[i], phase_increment);
            naive_pulse -= poly_blep_pulse(
                if self.phase[i] >= pulse_width {
                    self.phase[i] - pulse_width
                } else {
                    self.phase[i] - pulse_width + 1.0
                },
                phase_increment,
            );

            self.sample[i] = naive_pulse * 5.0;
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
