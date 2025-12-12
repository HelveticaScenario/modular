use anyhow::{anyhow, Result};
use crate::{
    dsp::utils::{clamp, wrap},
    types::{ChannelBuffer, InternalParam, NUM_CHANNELS},
};

#[derive(Default, Params)]
struct SawOscillatorParams {
    #[param("freq", "frequency in v/oct")]
    freq: InternalParam,
    #[param("shape", "waveform shape: 0=saw, 2.5=triangle, 5=ramp")]
    shape: InternalParam,
    #[param("phase", "the phase of the oscillator, overrides freq if present")]
    phase: InternalParam,
}

#[derive(Default, Module)]
#[module("saw", "Sawtooth/Triangle/Ramp oscillator")]
pub struct SawOscillator {
    #[output("output", "signal output", default)]
    sample: ChannelBuffer,
    phase: ChannelBuffer,
    last_phase: ChannelBuffer,
    smoothed_freq: ChannelBuffer,
    smoothed_shape: ChannelBuffer,
    params: SawOscillatorParams,
}

impl SawOscillator {
    fn update(&mut self, sample_rate: f32) -> () {
        let mut target_shape = ChannelBuffer::default();
        let mut target_freq = [4.0; NUM_CHANNELS];
        let mut phase_in = ChannelBuffer::default();

        self.params.shape.get_value(&mut target_shape);
        self.params
            .freq
            .get_value_or(&mut target_freq, &[4.0; NUM_CHANNELS]);
        if self.params.phase != InternalParam::Disconnected {
            self.params.phase.get_value(&mut phase_in);
        }

        for i in 0..NUM_CHANNELS {
            target_shape[i] = target_shape[i].clamp(0.0, 5.0);
            target_freq[i] = clamp(-10.0, 10.0, target_freq[i]);
        }
        crate::types::smooth_buffer(&mut self.smoothed_shape, &target_shape);
        crate::types::smooth_buffer(&mut self.smoothed_freq, &target_freq);

        let sr = sample_rate.max(1.0);
        for i in 0..NUM_CHANNELS {
            let (current_phase, phase_increment) = if self.params.phase != InternalParam::Disconnected {
                let wrapped_phase = wrap(0.0..1.0, phase_in[i]);
                let phase_inc = if wrapped_phase >= self.last_phase[i] {
                    wrapped_phase - self.last_phase[i]
                } else {
                    wrapped_phase + (1.0 - self.last_phase[i])
                };
                self.phase[i] = wrapped_phase;
                (wrapped_phase, phase_inc)
            } else {
                let voltage = self.smoothed_freq[i];
                let frequency = 27.5f32 * voltage.exp2();
                let phase_increment = frequency / sr;
                self.phase[i] += phase_increment;
                if self.phase[i] >= 1.0 {
                    self.phase[i] -= self.phase[i].floor();
                }
                (self.phase[i], phase_increment)
            };

            self.last_phase[i] = current_phase;

            let shape_norm = self.smoothed_shape[i] / 5.0;
            let output = if shape_norm < 0.5 {
                let blend = shape_norm * 2.0;
                let saw = generate_saw(current_phase, phase_increment);
                let triangle = generate_triangle(current_phase, phase_increment);
                saw * (1.0 - blend) + triangle * blend
            } else {
                let blend = (shape_norm - 0.5) * 2.0;
                let triangle = generate_triangle(current_phase, phase_increment);
                let ramp = generate_ramp(current_phase, phase_increment);
                triangle * (1.0 - blend) + ramp * blend
            };

            self.sample[i] = output * 5.0;
        }
    }
}

// PolyBLEP (Polynomial Band-Limited Step) function
// Reduces aliasing at discontinuities
fn poly_blep(phase: f32, phase_increment: f32) -> f32 {
    // Detect discontinuity at phase wrap (0.0)
    if phase < phase_increment {
        let t = phase / phase_increment;
        return t + t - t * t - 1.0;
    }
    // Detect discontinuity at phase = 1.0
    else if phase > 1.0 - phase_increment {
        let t = (phase - 1.0) / phase_increment;
        return t * t + t + t + 1.0;
    }
    0.0
}

// Generate band-limited sawtooth wave
fn generate_saw(phase: f32, phase_increment: f32) -> f32 {
    let mut saw = 2.0 * phase - 1.0;
    saw -= poly_blep(phase, phase_increment);
    saw
}

// Generate band-limited ramp wave (reversed sawtooth)
fn generate_ramp(phase: f32, phase_increment: f32) -> f32 {
    let mut ramp = 1.0 - 2.0 * phase;
    ramp += poly_blep(phase, phase_increment);
    ramp
}

// Generate band-limited triangle wave
fn generate_triangle(phase: f32, phase_increment: f32) -> f32 {
    // Triangle is the integral of a square wave
    // We can generate it by integrating a PolyBLEP pulse
    let mut triangle = if phase < 0.5 {
        4.0 * phase - 1.0
    } else {
        3.0 - 4.0 * phase
    };
    
    // Apply PolyBLEP correction at the peak (phase = 0.5)
    triangle += poly_blep_integrated(phase, phase_increment);
    triangle -= poly_blep_integrated(
        if phase >= 0.5 { phase - 0.5 } else { phase + 0.5 },
        phase_increment,
    );
    
    triangle
}

// Integrated PolyBLEP for triangle wave
fn poly_blep_integrated(phase: f32, phase_increment: f32) -> f32 {
    if phase < phase_increment {
        let t = phase / phase_increment;
        return (t * t * t) / 3.0 - (t * t) / 2.0 + t / 2.0;
    } else if phase > 1.0 - phase_increment {
        let t = (phase - 1.0) / phase_increment;
        return -(t * t * t) / 3.0 - (t * t) / 2.0 - t / 2.0;
    }
    0.0
}
