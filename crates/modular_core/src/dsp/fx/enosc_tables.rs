//! Lookup tables for ENOSC-based warp and twist effects.
//!
//! Adapted from the 4ms Ensemble Oscillator.
//! Copyright 4ms Company. Used under GPL v3.

use std::f32::consts::PI;
use std::sync::LazyLock;

/// Fold table size (1025 samples for smooth interpolation)
pub const FOLD_SIZE: usize = 1025;

/// Chebyshev table size (513 samples per polynomial)
pub const CHEBY_SIZE: usize = 513;

/// Number of Chebyshev polynomial tables (T₁ through T₁₆)
pub const CHEBY_TABLES_COUNT: usize = 16;

/// Triangle segment table size (9 samples per shape)
pub const TRIANGLE_SIZE: usize = 9;

/// Number of triangle segment tables
pub const TRIANGLE_TABLES_COUNT: usize = 8;

/// Fold normalization table size
pub const FOLD_MAX_SIZE: usize = (FOLD_SIZE - 1) / 2 + 1; // 513

/// Parabolic sine approximation matching 4ms Ensemble Oscillator `Math::fast_sine`.
/// Input `x` in [0, 1], output in approximately [-1, 1].
#[inline]
fn fast_sine(x: f32) -> f32 {
    let x = 2.0 * x - 1.0;
    let y = 4.0 * (x - x * x.abs());
    0.225 * (y * y.abs() - y) + y
}

/// Wavefolding lookup table.
/// Input is normalized phase [0, 1], output is folded value [-1, 1].
/// Implements 6x overfolding with sine-based smoothing.
pub static FOLD_TABLE: LazyLock<[f32; FOLD_SIZE]> = LazyLock::new(|| {
    let mut table = [0.0f32; FOLD_SIZE];
    let folds = 6.0f32;

    for i in 0..FOLD_SIZE {
        // Match reference: dynamic_data.cc fold generation
        let x = i as f32 / (FOLD_SIZE - 3) as f32; // 0..1 (the -3 makes the curve symmetrical)
        let x = folds * (2.0 * x - 1.0); // -folds..folds
        let g = 1.0 / (1.0 + x.abs()); // gain envelope
        let p = 16.0 / (2.0 * PI) * x * g;
        // Wrap phase to [0, 1] — handles negative values correctly
        let p = p.rem_euclid(1.0);
        // Folded value: -g * (x + fast_sine(p))
        table[i] = -g * (x + fast_sine(p));
    }

    table
});

/// Normalization table for fold effect (513 entries).
/// Computed from the fold table: running max from center rightward,
/// then reciprocal with 0.92 attenuation factor to prevent clipping.
pub static FOLD_MAX_TABLE: LazyLock<[f32; FOLD_MAX_SIZE]> = LazyLock::new(|| {
    let fold = &*FOLD_TABLE;
    let mut table = [0.0f32; FOLD_MAX_SIZE];
    let mut max = 0.0f32;
    // Start from center of fold table and scan rightward.
    let start = (FOLD_SIZE - 1) / 2;
    for i in 0..FOLD_MAX_SIZE {
        let idx = (i + start).min(FOLD_SIZE - 1);
        let val = fold[idx].abs();
        if val > max {
            max = val;
        }
        // Attenuation factor accounts for interpolation error
        table[i] = 0.92 / (max + 0.00001);
    }

    table
});

/// Chebyshev polynomial lookup tables (T₁ through T₁₆).
/// Each table maps input [-1, 1] (stored as [0, 1] phase) to polynomial output.
pub static CHEBY_TABLES: LazyLock<[[f32; CHEBY_SIZE]; CHEBY_TABLES_COUNT]> = LazyLock::new(|| {
    let mut tables = [[0.0f32; CHEBY_SIZE]; CHEBY_TABLES_COUNT];

    for i in 0..CHEBY_SIZE {
        let x = (i as f32 * 2.0) / (CHEBY_SIZE - 1) as f32 - 1.0; // [-1, 1]

        // T₁(x) = x (identity/fundamental)
        tables[0][i] = x;

        // T₂(x) = 2x² - 1 (second harmonic)
        tables[1][i] = 2.0 * x * x - 1.0;

        // Tₙ(x) = 2x·Tₙ₋₁(x) - Tₙ₋₂(x) (recurrence relation for T₃ through T₁₆)
        for n in 2..CHEBY_TABLES_COUNT {
            tables[n][i] = 2.0 * x * tables[n - 1][i] - tables[n - 2][i];
        }
    }

    tables
});

/// Triangle segment lookup tables (8 shapes).
/// Each shape has 9 control points defining a piecewise linear transfer function.
/// Values are in 12ths of an octave, converted to normalized [-1, 1] range.
pub static TRIANGLE_TABLES: LazyLock<[[f32; TRIANGLE_SIZE]; TRIANGLE_TABLES_COUNT]> =
    LazyLock::new(|| {
        // Original values in 12ths of an octave (musical intervals)
        let triangles_12ths: [[i32; TRIANGLE_SIZE]; TRIANGLE_TABLES_COUNT] = [
            [-12, -9, -6, -3, 0, 3, 6, 9, 12],       // Linear (identity)
            [-12, -12, -8, -4, 0, 4, 8, 12, 12],     // Compressed edges
            [-12, -12, -12, -6, 0, 6, 12, 12, 12],   // More compression
            [-12, -12, -12, -12, 0, 12, 12, 12, 12], // Square-ish
            [-12, -6, -12, -6, 0, 6, 12, 6, 12],     // Rippled
            [-12, -6, 0, -12, 0, 12, 0, 6, 12],      // More rippled
            [-12, -6, 12, -12, 0, 12, -12, 6, 12],   // Extreme ripple
            [12, -12, 12, -12, 0, 12, -12, 12, -12], // Alternating
        ];

        let mut tables = [[0.0f32; TRIANGLE_SIZE]; TRIANGLE_TABLES_COUNT];

        for shape in 0..TRIANGLE_TABLES_COUNT {
            for i in 0..TRIANGLE_SIZE {
                // Convert from 12ths to normalized [-1, 1]
                tables[shape][i] = triangles_12ths[shape][i] as f32 / 12.0;
            }
        }

        tables
    });

/// Interpolate a value from a lookup table.
/// `phase` is in range [0, 1], table wraps at boundaries.
#[inline]
pub fn interpolate_table(table: &[f32], phase: f32) -> f32 {
    let size = table.len();
    let pos = phase * (size - 1) as f32;
    let idx = pos as usize;
    let frac = pos - idx as f32;

    let idx0 = idx.min(size - 1);
    let idx1 = (idx + 1).min(size - 1);

    table[idx0] + frac * (table[idx1] - table[idx0])
}

/// Interpolate between two Chebyshev tables based on amount.
/// `x` is input signal in [-1, 1] (converted to table phase internally).
/// `amount` is [0, 1] selecting between T₁ and T₁₆.
#[inline]
pub fn interpolate_cheby(x: f32, amount: f32) -> f32 {
    // Map amount [0, 1] to table index range [0, 14] (15 crossfade positions)
    let scaled = amount * (CHEBY_TABLES_COUNT - 2) as f32;
    let idx = scaled as usize;
    let frac = scaled - idx as f32;

    let idx = idx.min(CHEBY_TABLES_COUNT - 2);

    // Convert x from [-1, 1] to table phase [0, 1]
    let phase = (x + 1.0) * 0.5;

    let s1 = interpolate_table(&CHEBY_TABLES[idx], phase);
    let s2 = interpolate_table(&CHEBY_TABLES[idx + 1], phase);

    s1 + frac * (s2 - s1)
}

/// Interpolate between triangle segment tables.
/// `x` is input signal in [-1, 1].
/// `amount` is [0, 1] selecting between 8 shapes.
#[inline]
pub fn interpolate_segment(x: f32, amount: f32) -> f32 {
    // Map amount to table index range [0, 7]
    let scaled = amount * (TRIANGLE_TABLES_COUNT - 1) as f32;
    let idx = scaled as usize;
    let frac = scaled - idx as f32;

    let idx = idx.min(TRIANGLE_TABLES_COUNT - 2);

    // Map input x [-1, 1] to segment index [0, 8]
    let x_scaled = (x + 1.0) * 0.5 * (TRIANGLE_SIZE - 1) as f32;
    let x_idx = x_scaled as usize;
    let x_frac = x_scaled - x_idx as f32;

    let x_idx = x_idx.min(TRIANGLE_SIZE - 2);

    // Interpolate within each table
    let t1 = &TRIANGLE_TABLES[idx];
    let t2 = &TRIANGLE_TABLES[idx + 1];

    let s1 = t1[x_idx] + x_frac * (t1[x_idx + 1] - t1[x_idx]);
    let s2 = t2[x_idx] + x_frac * (t2[x_idx + 1] - t2[x_idx]);

    // Crossfade between tables
    s1 + frac * (s2 - s1)
}

/// Lookup folded value from table.
/// `x` is input signal in [-1, 1].
/// `amount` scales the folding intensity [0, 1].
#[inline]
pub fn lookup_fold(x: f32, amount: f32) -> f32 {
    if amount <= 0.005 {
        return x;
    }
    // Reference: sample = x * s1_15(amount), phase = sample.to_unsigned_scale()
    // x ∈ [-1,1], amount ∈ [0,1] → sample ∈ [-1,1] → phase ∈ [0,1]
    let sample = x * amount;
    let phase = ((sample + 1.0) * 0.5).clamp(0.0, 1.0);

    let folded = interpolate_table(&*FOLD_TABLE, phase);

    // Multiply by normalization (reference: res *= fold_max.interpolate(amount))
    let norm = interpolate_table(&*FOLD_MAX_TABLE, amount);
    folded * norm
}

/// Per-effect anti-aliasing functions matching the 4ms Ensemble Oscillator.
/// `freq_norm` = freq_hz / sample_rate (normalized frequency).
/// Each function takes `(freq_norm, amount)` and returns the AA-scaled amount.

/// AA for fold warp: `max(amount × max(1−8f, 0)⁴, 0.004)`.
/// Most aggressive rolloff with floor to prevent silence.
#[inline]
pub fn aa_fold(freq_norm: f32, amount: f32) -> f32 {
    let base = (1.0 - 8.0 * freq_norm).max(0.0);
    (amount * base * base * base * base).max(0.004)
}

/// AA for cheby warp: `amount × max(1−6f, 0)`.
/// Linear rolloff.
#[inline]
pub fn aa_cheby(freq_norm: f32, amount: f32) -> f32 {
    amount * (1.0 - 6.0 * freq_norm).max(0.0)
}

/// AA for segment warp: `amount × max(1−4f, 0)³`.
/// Cubic rolloff.
#[inline]
pub fn aa_segment(freq_norm: f32, amount: f32) -> f32 {
    let base = (1.0 - 4.0 * freq_norm).max(0.0);
    amount * base * base * base
}

/// AA for feedback twist: `amount × max(1−2f, 0)²`.
/// Quadratic rolloff.
#[inline]
pub fn aa_feedback(freq_norm: f32, amount: f32) -> f32 {
    let base = (1.0 - 2.0 * freq_norm).max(0.0);
    amount * base * base
}

/// AA for pulsar twist: `(amount−1) × max(1−2f, 0)¹⁶ + 1`.
/// Very aggressive rolloff, preserving multiplier floor of 1.
/// `amount` is the pulsar multiplier (1..64).
#[inline]
pub fn aa_pulsar(freq_norm: f32, amount: f32) -> f32 {
    let base = (1.0 - 2.0 * freq_norm).max(0.0);
    (amount - 1.0) * base.powi(16) + 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fold_table_bounds() {
        // Table should be bounded — fold values are within ~[-1, 1]
        for &v in FOLD_TABLE.iter() {
            assert!(v >= -2.0 && v <= 2.0, "Fold value out of bounds: {}", v);
        }
    }

    #[test]
    fn test_fold_max_table_positive() {
        // All fold_max values should be positive (they're reciprocals)
        for &v in FOLD_MAX_TABLE.iter() {
            assert!(v > 0.0, "Fold max value should be positive: {}", v);
        }
    }

    #[test]
    fn test_cheby_table_identity() {
        // T₁(x) = x, so first table should be approximately linear
        for i in 0..CHEBY_SIZE {
            let expected = (i as f32 * 2.0) / (CHEBY_SIZE - 1) as f32 - 1.0;
            assert!((CHEBY_TABLES[0][i] - expected).abs() < 0.001);
        }
    }

    #[test]
    fn test_triangle_linear() {
        // First triangle table should be linear
        let table = &TRIANGLE_TABLES[0];
        for i in 0..TRIANGLE_SIZE {
            let expected = (i as f32 * 2.0) / (TRIANGLE_SIZE - 1) as f32 - 1.0;
            assert!((table[i] - expected).abs() < 0.001);
        }
    }
}
