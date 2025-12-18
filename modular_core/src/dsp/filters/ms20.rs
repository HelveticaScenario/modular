use std::f32::consts::PI;

use crate::{dsp::utils::clamp, types::InternalParam};
use anyhow::{Result, anyhow};

#[derive(Default, Params)]
struct MS20FilterParams {
    #[param("input", "signal input")]
    input: InternalParam,
    #[param("cutoff", "cutoff frequency in v/oct")]
    cutoff: InternalParam,
    #[param("q", "filter resonance (0-5)")]
    resonance: InternalParam,
}

#[derive(Default, Module)]
#[module("ms20", "Korg MS-20 style lowpass with aggressive distortion")]
pub struct MS20Filter {
    #[output("output", "filtered signal", default)]
    sample: f32,

    smoothed_cutoff: f32,
    smoothed_resonance: f32,
    params: MS20FilterParams,

    // === State variables (Faust: s1, s2, s3) ===
    s1: f32,
    s2: f32,
    s3: f32,

    // === Cached coefficients ===
    alpha: f32,
    alpha0: f32,
    b2: f32,
    b3: f32,
    k: f32,
}

impl MS20Filter {
    fn update(&mut self, sample_rate: f32) -> () {
        let input = self.params.input.get_value();
        let target_cutoff = self.params.cutoff.get_value_or(4.0);
        let target_resonance = clamp(0.0, 5.0, self.params.resonance.get_value_or(0.0));

        self.smoothed_cutoff = crate::types::smooth_value(self.smoothed_cutoff, target_cutoff);
        self.smoothed_resonance =
            crate::types::smooth_value(self.smoothed_resonance, target_resonance);

        // Convert v/oct to frequency
        let freq = 27.5f32 * 2.0f32.powf(self.smoothed_cutoff);
        let freq_clamped = clamp(20.0, sample_rate * 0.45, freq);
        self.update_coefficients(freq_clamped, self.smoothed_resonance * 2.0, sample_rate);
        self.sample = self.process(input);
        // println!("MS20 Filter output before clip: {}\n", self.sample);
        // Final stage clipping
        // self.sample = self.sample.clamp(-5.0, 5.0);
    }

    fn update_coefficients(&mut self, freq: f32, q: f32, sample_rate: f32) {
        // Faust:
        // K = 2.0*(Q - 0.707)/(10.0 - 0.707);
        self.k = 2.0 * (q - 0.707) / (10.0 - 0.707);

        // wd = 2*ma.PI*freq;
        let wd = 2.0 * PI * freq;

        // T = 1/ma.SR;
        let t = 1.0 / sample_rate;

        // wa = (2/T)*tan(wd*T/2);
        let wa = (2.0 / t) * (wd * t * 0.5).tan();

        // g = wa*T/2;
        let g = wa * t * 0.5;

        // G = g/(1.0 + g);
        let big_g = g / (1.0 + g);

        // alpha = G;
        self.alpha = big_g;

        // B3 = (K - K*G)/(1 + g);
        self.b3 = (self.k - self.k * big_g) / (1.0 + g);

        // B2 = -1/(1 + g);
        self.b2 = -1.0 / (1.0 + g);

        // alpha0 = 1/(1 - K*G + K*G*G);
        self.alpha0 = 1.0 / (1.0 - self.k * big_g + self.k * big_g * big_g);
    }

    fn process(&mut self, input: f32) -> f32 {
        // --- s1 update ---
        // 's1 = _-s1:_*(alpha*2):_+s1;
        let s1_temp = (input - self.s1) * (self.alpha * 2.0) + self.s1;

        // --- s2 update ---
        // Long expression directly mapped
        let s2_temp = {
            let v = (input - self.s1) * self.alpha + self.s1;
            let v = v + self.s3 * self.b3;
            let v = v + self.s2 * self.b2;
            let v = v * self.alpha0;
            let v = (v - self.s3) * self.alpha + self.s3;
            let v = v * self.k;
            let v = v - self.s2 * (self.alpha * 2.0);
            v + self.s2
        };

        // --- s3 update ---
        let s3_temp = {
            let v = (input - self.s1) * self.alpha + self.s1;
            let v = v + self.s3 * self.b3;
            let v = v + self.s2 * self.b2;
            let v = v * self.alpha0;
            let v = (v - self.s3) * (self.alpha * 2.0);
            v + self.s3
        };

        // --- output y ---
        // 'y = _-s1:_*alpha:_+s1:_+(s3*B3):_+(s2*B2)
        //      :_*alpha0:_-s3:_*alpha:_+s3;
        let y = {
            let v = (input - self.s1) * self.alpha + self.s1;
            let v = v + self.s3 * self.b3;
            let v = v + self.s2 * self.b2;
            let v = v * self.alpha0;
            let v = (v - self.s3) * self.alpha;
            v + self.s3
        };

        // Commit state updates
        self.s1 = s1_temp;
        self.s2 = s2_temp;
        self.s3 = s3_temp;

        y
    }
}
