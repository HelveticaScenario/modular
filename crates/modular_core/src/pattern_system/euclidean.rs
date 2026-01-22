//! Euclidean rhythm generation using the Bjorklund algorithm.
//!
//! Euclidean rhythms are rhythms obtained using the greatest common divisor
//! of two numbers. They were described in 2004 by Godfried Toussaint.
//!
//! The Bjorklund algorithm implementation is ported from the Haskell Music
//! Theory module by Rohan Drape.

use super::{combinators::fastcat, constructors::pure, Fraction, Pattern};

/// Generate a Euclidean rhythm pattern using the Bjorklund algorithm.
///
/// # Arguments
/// * `pulses` - Number of onsets/beats (can be negative to invert)
/// * `steps` - Total number of steps to fill
///
/// # Returns
/// A vector of booleans where true = pulse, false = rest
///
/// # Example
/// ```ignore
/// // Cuban tresillo pattern: [true, false, false, true, false, false, true, false]
/// let pattern = bjorklund(3, 8);
/// ```
pub fn bjorklund(pulses: i32, steps: u32) -> Vec<bool> {
    if steps == 0 {
        return Vec::new();
    }

    let inverted = pulses < 0;
    let abs_pulses = pulses.unsigned_abs().min(steps);
    let offs = steps - abs_pulses;

    // Initialize with ones (pulses) and zeros (rests)
    let ones: Vec<Vec<bool>> = (0..abs_pulses).map(|_| vec![true]).collect();
    let zeros: Vec<Vec<bool>> = (0..offs).map(|_| vec![false]).collect();

    let result = bjorklund_inner((abs_pulses, offs), (ones, zeros));

    // Flatten the result
    let mut pattern: Vec<bool> = result
        .0
        .into_iter()
        .chain(result.1.into_iter())
        .flat_map(|v| v.into_iter())
        .collect();

    // Invert if pulses was negative
    if inverted {
        pattern = pattern.into_iter().map(|x| !x).collect();
    }

    pattern
}

type BjorklundState = (Vec<Vec<bool>>, Vec<Vec<bool>>);

fn bjorklund_inner(n: (u32, u32), x: BjorklundState) -> BjorklundState {
    let (ons, offs) = n;

    if ons.min(offs) <= 1 {
        return x;
    }

    if ons > offs {
        let (new_n, new_x) = left(n, x);
        bjorklund_inner(new_n, new_x)
    } else {
        let (new_n, new_x) = right(n, x);
        bjorklund_inner(new_n, new_x)
    }
}

fn left(n: (u32, u32), x: BjorklundState) -> ((u32, u32), BjorklundState) {
    let (ons, offs) = n;
    let (xs, ys) = x;

    let split_point = offs as usize;
    let (_xs, __xs) = split_at(&xs, split_point);

    let zipped = zip_with_concat(&_xs, &ys);

    ((offs, ons - offs), (zipped, __xs))
}

fn right(n: (u32, u32), x: BjorklundState) -> ((u32, u32), BjorklundState) {
    let (ons, offs) = n;
    let (xs, ys) = x;

    let split_point = ons as usize;
    let (_ys, __ys) = split_at(&ys, split_point);

    let zipped = zip_with_concat(&xs, &_ys);

    ((ons, offs - ons), (zipped, __ys))
}

fn split_at<T: Clone>(vec: &[T], n: usize) -> (Vec<T>, Vec<T>) {
    let n = n.min(vec.len());
    (vec[..n].to_vec(), vec[n..].to_vec())
}

fn zip_with_concat(a: &[Vec<bool>], b: &[Vec<bool>]) -> Vec<Vec<bool>> {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let mut result = x.clone();
            result.extend(y.iter().cloned());
            result
        })
        .collect()
}

/// Rotate a vector by n positions.
fn rotate<T: Clone>(vec: &[T], n: i32) -> Vec<T> {
    if vec.is_empty() {
        return vec.to_vec();
    }

    let len = vec.len() as i32;
    let n = ((n % len) + len) % len;
    let n = n as usize;

    let mut result = vec[n..].to_vec();
    result.extend_from_slice(&vec[..n]);
    result
}

/// Generate a Euclidean rhythm with optional rotation.
///
/// # Arguments
/// * `pulses` - Number of onsets/beats
/// * `steps` - Total number of steps
/// * `rotation` - Optional rotation offset
pub fn euclidean_rhythm(pulses: i32, steps: u32, rotation: Option<i32>) -> Vec<bool> {
    let pattern = bjorklund(pulses, steps);
    match rotation {
        Some(r) => rotate(&pattern, -r),
        None => pattern,
    }
}

/// Create a pattern from a Euclidean rhythm.
///
/// Returns a pattern that plays the value at pulse positions and is silent elsewhere.
pub fn euclid<T: Clone + Send + Sync + 'static>(
    value: T,
    pulses: i32,
    steps: u32,
    rotation: Option<i32>,
) -> Pattern<T> {
    let rhythm = euclidean_rhythm(pulses, steps, rotation);
    let patterns: Vec<Pattern<Option<T>>> = rhythm
        .into_iter()
        .map(|is_pulse| {
            if is_pulse {
                pure(Some(value.clone()))
            } else {
                pure(None)
            }
        })
        .collect();

    fastcat(patterns).filter_values(|v| v.is_some()).fmap(|v| v.clone().unwrap())
}

/// Create a boolean pattern from a Euclidean rhythm.
///
/// Useful for gating/triggering.
pub fn euclid_bool(pulses: i32, steps: u32, rotation: Option<i32>) -> Pattern<bool> {
    let rhythm = euclidean_rhythm(pulses, steps, rotation);
    let patterns: Vec<Pattern<bool>> = rhythm.into_iter().map(pure).collect();
    fastcat(patterns)
}

/// Struct pattern from Euclidean rhythm.
///
/// Creates a pattern that has events only at the pulse positions.
pub fn euclid_struct(pulses: i32, steps: u32, rotation: Option<i32>) -> Pattern<()> {
    let rhythm = euclidean_rhythm(pulses, steps, rotation);
    let patterns: Vec<Pattern<Option<()>>> = rhythm
        .into_iter()
        .map(|is_pulse| {
            if is_pulse {
                pure(Some(()))
            } else {
                pure(None)
            }
        })
        .collect();

    fastcat(patterns).filter_values(|v| v.is_some()).fmap(|_| ())
}

impl<T: Clone + Send + Sync + 'static> Pattern<T> {
    /// Apply Euclidean rhythm structure to this pattern.
    ///
    /// The pattern's events are distributed according to the Euclidean rhythm.
    /// Non-pulse positions are filtered out (no hap returned).
    /// 
    /// For patterns that support rests, use `euclid_with_rest` instead to
    /// ensure queries always return a hap.
    pub fn euclid(&self, pulses: i32, steps: u32) -> Pattern<T> {
        self.euclid_rot(pulses, steps, 0)
    }

    /// Apply Euclidean rhythm structure with rotation.
    /// 
    /// Non-pulse positions are filtered out (no hap returned).
    /// For patterns that support rests, use `euclid_rot_with_rest` instead.
    pub fn euclid_rot(&self, pulses: i32, steps: u32, rotation: i32) -> Pattern<T> {
        let struct_pat = euclid_struct(pulses, steps, Some(rotation));
        self.app_left(&struct_pat, |val, _| val.clone())
    }

    /// Apply Euclidean rhythm structure with rest values at non-pulse positions.
    ///
    /// This ensures the pattern always returns a hap when queried.
    pub fn euclid_with_rest(&self, pulses: i32, steps: u32, rest: T) -> Pattern<T> {
        self.euclid_rot_with_rest(pulses, steps, 0, rest)
    }

    /// Apply Euclidean rhythm structure with rotation and rest values.
    ///
    /// Non-pulse positions produce the rest value instead of being filtered.
    /// This ensures the pattern always returns a hap when queried.
    pub fn euclid_rot_with_rest(&self, pulses: i32, steps: u32, rotation: i32, rest: T) -> Pattern<T> {
        let bool_pat = euclid_bool(pulses, steps, Some(rotation));
        
        // Use app_right to take structure from bool_pat (the euclidean rhythm steps)
        // while querying self for values at each position
        let pat = self.clone();
        let rest_val = rest.clone();
        
        pat.app_right(&bool_pat, move |val, is_pulse| {
            if *is_pulse {
                val.clone()
            } else {
                rest_val.clone()
            }
        })
    }

    /// Apply Euclidean rhythm with patterned parameters.
    ///
    /// All parameters (pulses, steps, rotation) can be patterns.
    /// This allows rhythms like `c([2 3], 8)` that alternate between
    /// 2-in-8 and 3-in-8 euclidean patterns.
    pub fn euclid_pat_with_rest<P, S, R>(
        &self,
        pulses: P,
        steps: S,
        rotation: R,
        rest: T,
    ) -> Pattern<T>
    where
        P: super::IntoPattern<i32> + 'static,
        S: super::IntoPattern<u32> + 'static,
        R: super::IntoPattern<i32> + 'static,
    {
        let pulses_pat = pulses.into_pattern();
        let steps_pat = steps.into_pattern();
        let rotation_pat = rotation.into_pattern();
        let pat = self.clone();

        // Combine the three parameter patterns into one pattern of (pulses, steps, rotation)
        // Use inner_join to get structure from the parameter patterns
        pulses_pat.inner_join(move |p| {
            let steps_pat = steps_pat.clone();
            let rotation_pat = rotation_pat.clone();
            let pat = pat.clone();
            let rest = rest.clone();
            let pulses = *p;

            steps_pat.inner_join(move |s| {
                let rotation_pat = rotation_pat.clone();
                let pat = pat.clone();
                let rest = rest.clone();
                let steps = *s;

                rotation_pat.inner_join(move |r| {
                    pat.euclid_rot_with_rest(pulses, steps, *r, rest.clone())
                })
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bjorklund_tresillo() {
        // Cuban tresillo: 3 pulses in 8 steps
        let pattern = bjorklund(3, 8);
        assert_eq!(pattern.len(), 8);
        assert_eq!(pattern.iter().filter(|&&x| x).count(), 3);

        // Expected: [1,0,0,1,0,0,1,0]
        assert_eq!(
            pattern,
            vec![true, false, false, true, false, false, true, false]
        );
    }

    #[test]
    fn test_bjorklund_cinquillo() {
        // Cuban cinquillo: 5 pulses in 8 steps
        let pattern = bjorklund(5, 8);
        assert_eq!(pattern.len(), 8);
        assert_eq!(pattern.iter().filter(|&&x| x).count(), 5);
    }

    #[test]
    fn test_bjorklund_full() {
        // All pulses
        let pattern = bjorklund(4, 4);
        assert_eq!(pattern, vec![true, true, true, true]);
    }

    #[test]
    fn test_bjorklund_empty() {
        // No pulses
        let pattern = bjorklund(0, 4);
        assert_eq!(pattern, vec![false, false, false, false]);
    }

    #[test]
    fn test_bjorklund_inverted() {
        // Negative pulses invert the pattern
        let normal = bjorklund(3, 8);
        let inverted = bjorklund(-3, 8);

        for (n, i) in normal.iter().zip(inverted.iter()) {
            assert_eq!(*n, !*i);
        }
    }

    #[test]
    fn test_rotate() {
        let vec = vec![1, 2, 3, 4, 5];
        assert_eq!(rotate(&vec, 0), vec![1, 2, 3, 4, 5]);
        assert_eq!(rotate(&vec, 1), vec![2, 3, 4, 5, 1]);
        assert_eq!(rotate(&vec, -1), vec![5, 1, 2, 3, 4]);
        assert_eq!(rotate(&vec, 5), vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_euclid_pattern() {
        let pat = euclid(42, 3, 8, None);
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        // Should have 3 events (pulses)
        assert_eq!(haps.len(), 3);
        for hap in &haps {
            assert_eq!(hap.value, 42);
        }
    }

    #[test]
    fn test_euclid_bool_pattern() {
        let pat = euclid_bool(3, 8, None);
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        // Should have 8 events total
        assert_eq!(haps.len(), 8);

        // 3 should be true
        let true_count = haps.iter().filter(|h| h.value).count();
        assert_eq!(true_count, 3);
    }

    #[test]
    fn test_pattern_euclid_method() {
        let pat = pure(100).euclid(3, 8);
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        assert_eq!(haps.len(), 3);
    }
}
