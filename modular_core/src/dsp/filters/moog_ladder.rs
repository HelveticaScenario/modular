use crate::{
    dsp::utils::{clamp, cubic_clipper, cv_to_khz},
    types::InternalParam,
};
use anyhow::{Result, anyhow};

#[derive(Default, Params)]
struct MoogLadderFilterParams {
    #[param("input", "signal input")]
    input: InternalParam,
    #[param("cutoff", "cutoff frequency in v/oct")]
    cutoff: InternalParam,
    #[param("q", "filter resonance (0-5)")]
    resonance: InternalParam,
}

#[derive(Default, Module)]
#[module("ladder", "24dB/octave Moog-style ladder filter")]
pub struct MoogLadderFilter {
    #[output("output", "filtered signal", default)]
    sample: f32,
    // State variables for 4-pole (24dB/oct) ladder filter
    stage: [f32; 4],
    delay: [f32; 4],
    smoothed_cutoff: f32,
    smoothed_resonance: f32,
    params: MoogLadderFilterParams,
}

impl MoogLadderFilter {
    fn update(&mut self, sample_rate: f32) -> () {
        let input = self.params.input.get_value();
        let target_cutoff = self.params.cutoff.get_value_or(4.0);
        let target_resonance = self.params.resonance.get_value_or(0.0);

        self.smoothed_cutoff = crate::types::smooth_value(self.smoothed_cutoff, target_cutoff);
        self.smoothed_resonance =
            crate::types::smooth_value(self.smoothed_resonance, target_resonance);

        // Convert v/oct to frequency
        let freq = 27.5f32 * 2.0f32.powf(self.smoothed_cutoff);
        let freq_clamped = freq.min(sample_rate * 0.45).max(20.0);

        // Calculate filter coefficients
        let fc = freq_clamped / sample_rate;
        let f = fc * 1.16;
        let fb = self.smoothed_resonance / 5.0 * 4.0;

        // Input with feedback
        let input_fb = input - self.sample * fb;

        // Tanh saturation for non-linearity (simplified)
        let saturate = |x: f32| {
            if x > 1.0 {
                1.0
            } else if x < -1.0 {
                -1.0
            } else {
                x
            }
        };

        // Process through 4 one-pole stages
        for i in 0..4 {
            let stage_input = if i == 0 { input_fb } else { self.stage[i - 1] };
            self.stage[i] = self.delay[i] + f * (saturate(stage_input) - self.delay[i]);
            self.delay[i] = self.stage[i];
        }

        self.sample = self.stage[3];

        // Soft clipping to prevent overflow
        self.sample = self.sample.clamp(-5.0, 5.0);
    }
}

fn tune(cut: f32) -> f32 {
    let f = cv_to_khz(cut);
    let f = clamp(0.0, 20.0, f);
    let fh = (2.0 * std::f32::consts::PI) * f / (4.0 * 44.1);
    return fh;
}

struct HeunState {
    p0: f32,
    p1: f32,
    p2: f32,
    p3: f32,
}
fn heun(heun_state: &mut HeunState, input: f32, fh: f32, res: f32) -> f32 {
    let wt0 = cubic_clipper(input - 4.0 * res * heun_state.p3);
    let wt1 = cubic_clipper(heun_state.p0);
    let dpt0 = (wt0 - wt1) * fh;
    let wt3 = cubic_clipper(heun_state.p1);
    let dpt1 = (wt1 - wt3) * fh;
    let wt5 = cubic_clipper(heun_state.p2);
    let dpt2 = (wt3 - wt5) * fh;
    let wt7 = cubic_clipper(heun_state.p3);
    let dpt3 = (wt5 - wt7) * fh;

    let pt0 = heun_state.p0 + dpt0;
    let pt1 = heun_state.p1 + dpt1;
    let pt2 = heun_state.p2 + dpt2;
    let pt3 = heun_state.p3 + dpt3;

    let w0 = cubic_clipper(input - 4.0 * res * pt3);
    let w1 = cubic_clipper(pt0);
    let dp0 = (w0 - w1) * fh;
    let w3 = cubic_clipper(pt1);
    let dp1 = (w1 - w3) * fh;
    let w5 = cubic_clipper(pt2);
    let dp2 = (w3 - w5) * fh;
    let w7 = cubic_clipper(pt3);
    let dp3 = (w5 - w7) * fh;

    heun_state.p0 += (dp0 + dpt0) / 2.0;
    heun_state.p1 += (dp1 + dpt1) / 2.0;
    heun_state.p2 += (dp2 + dpt2) / 2.0;
    heun_state.p3 += (dp3 + dpt3) / 2.0;

    return heun_state.p3;
}

struct EurlerState {
    p0: f32,
    p1: f32,
    p2: f32,
    p3: f32,
}

fn euler(state: &mut EurlerState, input: f32, fh: f32, res: f32) -> f32 {
    let w0 = cubic_clipper(input - 4.0 * res * state.p3);
    let w1 = cubic_clipper(state.p0);
    let dpt0 = (w0 - w1) * fh;
    let w3 = cubic_clipper(state.p1);
    let dpt1 = (w1 - w3) * fh;
    let w5 = cubic_clipper(state.p2);
    let dpt2 = (w3 - w5) * fh;
    let w7 = cubic_clipper(state.p3);
    let dpt3 = (w5 - w7) * fh;
    state.p0 += dpt0;
    state.p1 += dpt1;
    state.p2 += dpt2;
    state.p3 += dpt3;
    return state.p3;
}

/*
fun process_euler(input:real, cut:real, res:real):real{
   mem fh;
   if(Util.change(cut)) {
      fh = tune(cut);
   }
    _ = e:euler(input, fh, res);
    _ = e:euler(input, fh, res);
    _ = e:euler(input, fh, res);
    val out = e:euler(input, fh, res);
    return out;
}

fun process_heun(input:real, cut:real, res:real):real{
   mem fh;
   if(Util.change(cut)) {
      fh = tune(cut);
   }
    _ = h:heun(input, fh, res);
    _ = h:heun(input, fh, res);
    _ = h:heun(input, fh, res);
    val out = h:heun(input, fh, res);
    return out;
}

fun process(input:real, cut:real, res:real):real{
    return process_heun(input, cut, res);
}
*/
