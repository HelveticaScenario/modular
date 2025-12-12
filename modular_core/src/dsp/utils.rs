use crate::dsp::consts::{LUT_PITCH_RATIO_HIGH, LUT_PITCH_RATIO_LOW};

// The following are ported directly from Vult DSP utils.vult

/// Detect rising edges on a boolean stream.
#[derive(Default, Clone, Copy, Debug)]
pub struct EdgeDetector {
    prev: bool,
}

impl EdgeDetector {
    pub fn new() -> Self {
        Self { prev: false }
    }

    /// Returns true only when the input transitions low -> high.
    pub fn edge(&mut self, x: bool) -> bool {
        let ret = x && !self.prev;
        self.prev = x;
        ret
    }
}

/// Detect any change on a real-valued stream.
#[derive(Default, Clone, Copy, Debug)]
pub struct ChangeDetector {
    prev: Option<f32>,
}

impl ChangeDetector {
    pub fn new() -> Self {
        Self { prev: None }
    }

    /// Returns true if the value differs from the previous call.
    pub fn changed(&mut self, x: f32) -> bool {
        let changed = match self.prev {
            Some(prev) => prev != x,
            None => false,
        };
        self.prev = Some(x);
        changed
    }
}

/// Map a value from one range to another. If the input range is degenerate, returns `y0`.
pub fn map_range(x: f32, x0: f32, x1: f32, y0: f32, y1: f32) -> f32 {
    let denom = x1 - x0;
    if denom.abs() < f32::EPSILON {
        return y0;
    }
    (x - x0) * (y1 - y0) / denom + y0
}

/// Simple DC blocking filter (one-pole). Matches the Vult implementation semantics.
#[derive(Default, Clone, Copy, Debug)]
pub struct DcBlock {
    x1: f32,
    y1: f32,
}

impl DcBlock {
    pub fn new() -> Self {
        Self { x1: 0.0, y1: 0.0 }
    }

    pub fn process(&mut self, x0: f32) -> f32 {
        let y0 = x0 - self.x1 + self.y1 * 0.995;
        self.x1 = x0;
        self.y1 = y0;
        y0
    }
}

/// Low-pass smoother with fixed coefficient (approx 0.5% blend per call).
#[derive(Default, Clone, Copy, Debug)]
pub struct Smoother {
    state: f32,
}

impl Smoother {
    pub fn new() -> Self {
        Self { state: 0.0 }
    }

    pub fn process(&mut self, input: f32) -> f32 {
        self.state = self.state + (input - self.state) * 0.005;
        self.state
    }
}

/// Two-sample running average.
#[derive(Default, Clone, Copy, Debug)]
pub struct Average2 {
    prev: f32,
}

impl Average2 {
    pub fn new() -> Self {
        Self { prev: 0.0 }
    }

    pub fn process(&mut self, x1: f32) -> f32 {
        let result = (self.prev + x1) * 0.5;
        self.prev = x1;
        result
    }
}

/// Soft cubic clipper with saturation at +/- 2/3.
pub fn cubic_clipper(x: f32) -> f32 {
    if x <= -2.0 / 3.0 {
        -2.0 / 3.0
    } else if x >= 2.0 / 3.0 {
        2.0 / 3.0
    } else {
        x - (x * x * x) / 3.0
    }
}

/// Rate for a 2^10 ramp at 44.1kHz for the given MIDI pitch.
pub fn pitch_to_rate_1024(pitch: f32) -> f32 {
    // 2^10 / 44100 * 440 * 2^((pitch - 69)/12)
    0.18984168003671556_f32 * (0.057762265046662105_f32 * pitch).exp()
}

/// Rate for a 1-cycle ramp at 44.1kHz for the given MIDI pitch.
pub fn pitch_to_rate(pitch: f32) -> f32 {
    // 1.0 / 44100 * 440 * 2^((pitch - 69)/12)
    0.00018539226566085504_f32 * (0.057762265046662105_f32 * pitch).exp()
}

pub fn cv_to_pitch(cv: f32) -> f32 {
    cv * 120.0 + 24.0
}

pub fn cv_to_rate_1024(cv: f32) -> f32 {
    pitch_to_rate_1024(cv_to_pitch(cv))
}

pub fn cv_to_rate(cv: f32) -> f32 {
    pitch_to_rate(cv_to_pitch(cv))
}

pub fn pitch_to_cv(pitch: f32) -> f32 {
    (pitch - 24.0) / 120.0
}

pub fn cv_to_period(cv: f32) -> f32 {
    let pitch = cv_to_pitch(cv);
    let f = 8.175798915643707_f32 * (0.057762265046662105_f32 * pitch).exp();
    44100.0_f32 / f / 2.0
}

/// Returns frequency in kHz for the corresponding CV.
pub fn cv_to_khz(cv: f32) -> f32 {
    let pitch = cv_to_pitch(cv);
    let f = 8.175798915643707_f32 * (0.057762265046662105_f32 * pitch).exp();
    f / 1000.0
}

// End of Vult ported code

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

pub fn wrap<T>(range: std::ops::Range<T>, mut val: T) -> T
where
    T: std::ops::Sub<Output = T>
        + std::ops::AddAssign
        + std::cmp::PartialOrd
        + std::ops::SubAssign
        + Copy,
{
    let span = range.end - range.start;
    while val > range.end {
        val -= span;
    }
    while val < range.start {
        val += span;
    }
    val
}

#[cfg(test)]
mod tests {
    use super::*;

    // Tests for make_integral_fractional
    #[test]
    fn test_make_integral_fractional_positive() {
        let (integral, fractional) = make_integral_fractional(3.75);
        assert_eq!(integral, 3);
        assert!((fractional - 0.75).abs() < 0.0001);
    }

    #[test]
    fn test_make_integral_fractional_whole_number() {
        let (integral, fractional) = make_integral_fractional(5.0);
        assert_eq!(integral, 5);
        assert!((fractional - 0.0).abs() < 0.0001);
    }

    #[test]
    fn test_make_integral_fractional_negative() {
        let (integral, fractional) = make_integral_fractional(-2.25);
        assert_eq!(integral, -2);
        // -2.25 - (-2) = -0.25
        assert!((fractional - (-0.25)).abs() < 0.0001);
    }

    // Tests for semitones_to_ratio
    #[test]
    fn test_semitones_to_ratio_zero() {
        // 0 semitones = ratio of 1.0
        let ratio = semitones_to_ratio(0.0);
        assert!(
            (ratio - 1.0).abs() < 0.01,
            "0 semitones should be ratio 1.0, got {}",
            ratio
        );
    }

    #[test]
    fn test_semitones_to_ratio_octave_up() {
        // 12 semitones = ratio of 2.0 (one octave up)
        let ratio = semitones_to_ratio(12.0);
        assert!(
            (ratio - 2.0).abs() < 0.01,
            "12 semitones should be ratio 2.0, got {}",
            ratio
        );
    }

    #[test]
    fn test_semitones_to_ratio_octave_down() {
        // -12 semitones = ratio of 0.5 (one octave down)
        let ratio = semitones_to_ratio(-12.0);
        assert!(
            (ratio - 0.5).abs() < 0.01,
            "-12 semitones should be ratio 0.5, got {}",
            ratio
        );
    }

    #[test]
    fn test_semitones_to_ratio_perfect_fifth() {
        // 7 semitones = ratio of ~1.498 (perfect fifth)
        let ratio = semitones_to_ratio(7.0);
        assert!(
            (ratio - 1.498).abs() < 0.01,
            "7 semitones should be ~1.498, got {}",
            ratio
        );
    }

    // Tests for clamp
    #[test]
    fn test_clamp_within_range() {
        assert_eq!(clamp(0.0, 10.0, 5.0), 5.0);
    }

    #[test]
    fn test_clamp_below_min() {
        assert_eq!(clamp(0.0, 10.0, -5.0), 0.0);
    }

    #[test]
    fn test_clamp_above_max() {
        assert_eq!(clamp(0.0, 10.0, 15.0), 10.0);
    }

    #[test]
    fn test_clamp_at_min() {
        assert_eq!(clamp(0.0, 10.0, 0.0), 0.0);
    }

    #[test]
    fn test_clamp_at_max() {
        assert_eq!(clamp(0.0, 10.0, 10.0), 10.0);
    }

    #[test]
    fn test_clamp_integers() {
        assert_eq!(clamp(0, 100, 50), 50);
        assert_eq!(clamp(0, 100, -10), 0);
        assert_eq!(clamp(0, 100, 200), 100);
    }

    // Tests for wrap
    #[test]
    fn test_wrap_within_range() {
        let result: f32 = wrap(0.0..1.0, 0.5);
        assert!((result - 0.5).abs() < 0.0001);
    }

    #[test]
    fn test_wrap_above_range() {
        let result: f32 = wrap(0.0..1.0, 1.5);
        assert!(
            (result - 0.5).abs() < 0.0001,
            "1.5 wrapped to 0-1 should be 0.5, got {}",
            result
        );
    }

    #[test]
    fn test_wrap_below_range() {
        let result: f32 = wrap(0.0..1.0, -0.25);
        assert!(
            (result - 0.75).abs() < 0.0001,
            "-0.25 wrapped to 0-1 should be 0.75, got {}",
            result
        );
    }

    #[test]
    fn test_wrap_multiple_times_above() {
        let result: f32 = wrap(0.0..1.0, 2.75);
        assert!(
            (result - 0.75).abs() < 0.0001,
            "2.75 wrapped to 0-1 should be 0.75, got {}",
            result
        );
    }

    #[test]
    fn test_wrap_phase_range() {
        // Common use case: wrapping phase in range 0..2*PI
        use std::f32::consts::PI;
        let result: f32 = wrap(0.0..(2.0 * PI), 3.0 * PI);
        assert!(
            (result - PI).abs() < 0.0001,
            "3*PI wrapped to 0..2*PI should be PI, got {}",
            result
        );
    }

    #[test]
    fn test_wrap_integer() {
        let result = wrap(0..10, 15);
        assert_eq!(result, 5);
    }

    #[test]
    fn test_edge_detector() {
        let mut e = EdgeDetector::new();
        assert_eq!(e.edge(false), false);
        assert_eq!(e.edge(true), true);
        assert_eq!(e.edge(true), false);
        assert_eq!(e.edge(false), false);
    }

    #[test]
    fn test_change_detector() {
        let mut c = ChangeDetector::new();
        assert_eq!(c.changed(0.5), false); // first sample
        assert_eq!(c.changed(0.5), false);
        assert_eq!(c.changed(0.6), true);
        assert_eq!(c.changed(0.6), false);
    }

    #[test]
    fn test_map_range() {
        assert!((map_range(0.5, 0.0, 1.0, -1.0, 1.0) - 0.0).abs() < 1e-6);
        assert_eq!(map_range(1.0, 1.0, 1.0, 2.0, 4.0), 2.0);
    }

    #[test]
    fn test_dcblock() {
        let mut d = DcBlock::new();
        // DC blocker is a one-pole highpass with a relatively low cutoff;
        // give it enough samples to settle.
        let steady = (0..5000).fold(0.0, |_, _| d.process(1.0));
        assert!(steady.abs() < 0.2); // DC largely removed
    }

    #[test]
    fn test_smoother_moves_toward_input() {
        let mut s = Smoother::new();
        let mut out = 0.0;
        for _ in 0..200 {
            out = s.process(1.0);
        }
        assert!(out > 0.6 && out < 1.0);
    }

    #[test]
    fn test_average2() {
        let mut a = Average2::new();
        assert_eq!(a.process(2.0), 1.0);
        assert_eq!(a.process(4.0), 3.0);
    }

    #[test]
    fn test_cubic_clipper() {
        assert_eq!(cubic_clipper(-2.0), -2.0 / 3.0);
        assert_eq!(cubic_clipper(2.0), 2.0 / 3.0);
        let mid = cubic_clipper(0.5);
        assert!(mid < 0.5 && mid > 0.0);
    }

    #[test]
    fn test_pitch_and_cv_rates() {
        let rate = pitch_to_rate(69.0); // A4
        let rate_1024 = pitch_to_rate_1024(69.0);
        assert!(rate > 0.0);
        assert!(rate_1024 > rate);

        let cv = pitch_to_cv(69.0);
        let back_pitch = cv_to_pitch(cv);
        assert!((back_pitch - 69.0).abs() < 1e-3);

        let rate_cv = cv_to_rate(cv);
        assert!((rate_cv - rate).abs() / rate < 0.01);
    }

    #[test]
    fn test_cv_to_period_and_khz() {
        let cv = 0.5;
        let period = cv_to_period(cv);
        let khz = cv_to_khz(cv);
        assert!(period > 0.0);
        assert!(khz > 0.0);
    }
}
