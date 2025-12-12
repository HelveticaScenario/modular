use anyhow::{anyhow, Result};
use crate::types::{ChannelBuffer, InternalParam, NUM_CHANNELS, smooth_value};

#[derive(Default, Params)]
struct FormantFilterParams {
    #[param("input", "signal input")]
    input: InternalParam,
    #[param("formant", "formant selection (0-5): a, e, i, o, u, mixed")]
    formant: InternalParam,
    #[param("morph", "morph between formants (0-5)")]
    morph: InternalParam,
    #[param("q", "resonance/Q factor (0-5)")]
    q: InternalParam,
}

#[derive(Default, Module)]
#[module("formant", "Vowel formant filter for vocal-like sounds")]
pub struct FormantFilter {
    #[output("output", "formant filtered signal", default)]
    sample: ChannelBuffer,
    // Three parallel bandpass filters for formants
    bp1_z1: ChannelBuffer,
    bp1_z2: ChannelBuffer,
    bp2_z1: ChannelBuffer,
    bp2_z2: ChannelBuffer,
    bp3_z1: ChannelBuffer,
    bp3_z2: ChannelBuffer,
    smoothed_formant: ChannelBuffer,
    smoothed_morph: ChannelBuffer,
    smoothed_q: ChannelBuffer,
    params: FormantFilterParams,
}

fn process_formant_bandpass(input: f32, freq: f32, q_val: f32, z1: &mut f32, z2: &mut f32, sample_rate: f32) -> f32 {
    let omega = 2.0 * std::f32::consts::PI * freq / sample_rate;
    let (sin_omega, cos_omega) = omega.sin_cos();
    let alpha = sin_omega / (2.0 * q_val);

    let b0 = alpha;
    let b2 = -alpha;
    let a0 = 1.0 + alpha;
    let a1 = -2.0 * cos_omega;
    let a2 = 1.0 - alpha;

    let b0_norm = b0 / a0;
    let b2_norm = b2 / a0;
    let a1_norm = a1 / a0;
    let a2_norm = a2 / a0;

    let w = input - a1_norm * *z1 - a2_norm * *z2;
    let output = b0_norm * w + b2_norm * *z2;
    *z2 = *z1;
    *z1 = w;
    output
}

impl FormantFilter {
    fn update(&mut self, sample_rate: f32) -> () {
        let mut input = ChannelBuffer::default();
        let mut target_formant = ChannelBuffer::default();
        let mut target_morph = ChannelBuffer::default();
        let mut target_q = [2.5; NUM_CHANNELS];

        self.params.input.get_value(&mut input);
        self.params.formant.get_value(&mut target_formant);
        self.params.morph.get_value(&mut target_morph);
        self.params
            .q
            .get_value_or(&mut target_q, &[2.5; NUM_CHANNELS]);

        for i in 0..NUM_CHANNELS {
            self.smoothed_formant[i] = smooth_value(self.smoothed_formant[i], target_formant[i]);
            self.smoothed_morph[i] = smooth_value(self.smoothed_morph[i], target_morph[i]);
            self.smoothed_q[i] = smooth_value(self.smoothed_q[i], target_q[i]);
        }

        let vowel_formants = [
            [730.0, 1090.0, 2440.0],
            [530.0, 1840.0, 2480.0],
            [390.0, 1990.0, 2550.0],
            [570.0, 840.0, 2410.0],
            [440.0, 1020.0, 2240.0],
        ];

        let sr = sample_rate.max(1.0);
        for i in 0..NUM_CHANNELS {
            let formant_pos = (self.smoothed_formant[i] / 5.0 * 4.0).max(0.0);
            let formant_idx = (formant_pos.floor() as usize).min(4);
            let next_idx = (formant_idx + 1).min(4);
            let morph_amount = formant_pos.fract();

            let f1 = vowel_formants[formant_idx][0] * (1.0 - morph_amount)
                + vowel_formants[next_idx][0] * morph_amount;
            let f2 = vowel_formants[formant_idx][1] * (1.0 - morph_amount)
                + vowel_formants[next_idx][1] * morph_amount;
            let f3 = vowel_formants[formant_idx][2] * (1.0 - morph_amount)
                + vowel_formants[next_idx][2] * morph_amount;

            let shift = 1.0 + (self.smoothed_morph[i] / 5.0 - 0.5) * 0.5;
            let f1_shifted = (f1 * shift).clamp(100.0, sr * 0.45);
            let f2_shifted = (f2 * shift).clamp(100.0, sr * 0.45);
            let f3_shifted = (f3 * shift).clamp(100.0, sr * 0.45);

            let q_val = (self.smoothed_q[i] / 5.0 * 9.0 + 2.0).max(0.5);

            let bp1 = process_formant_bandpass(input[i], f1_shifted, q_val, &mut self.bp1_z1[i], &mut self.bp1_z2[i], sr);
            let bp2 = process_formant_bandpass(input[i], f2_shifted, q_val, &mut self.bp2_z1[i], &mut self.bp2_z2[i], sr);
            let bp3 = process_formant_bandpass(input[i], f3_shifted, q_val, &mut self.bp3_z1[i], &mut self.bp3_z2[i], sr);

            let y = (bp1 * 1.5 + bp2 + bp3 * 0.5) / 3.0;
            self.sample[i] = y.clamp(-5.0, 5.0);
        }
    }
}
