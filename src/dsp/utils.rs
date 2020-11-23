use crate::dsp::consts::{LUT_PITCH_RATIO_HIGH, LUT_PITCH_RATIO_LOW};

fn make_integral_fractional(x: f32) -> (i32, f32) {
    let integral: i32 = x as i32;
    let fractional: f32 = x - (integral as f32);
    (integral, fractional)
}

pub fn semitones_to_ratio(semitones: f32) -> f32 {
    let pitch: f32 = semitones + 128.0;
    let (pitch_integral, pitch_fractional) = make_integral_fractional(pitch);

    return LUT_PITCH_RATIO_HIGH[pitch_integral as usize]
        * LUT_PITCH_RATIO_LOW[(pitch_fractional * 256.0) as usize];
}

pub fn interpolate(table: &'static [f32], mut index: f32, size: usize) -> f32 {
    index *= size as f32;
    let (index_integral, index_fractional) = make_integral_fractional(index);
    let a: f32 = table[index_integral as usize];
    let b: f32 = table[(index_integral + 1) as usize];
    return a + (b - a) * index_fractional;
}

pub fn clamp<T: std::cmp::PartialOrd>(min: T, max: T, val: T) -> T {
    if val < min {
        min
    } else if val > max {
        max
    } else {
        val
    }
}
