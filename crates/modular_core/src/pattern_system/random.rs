//! Deterministic pseudo-random patterns.
//!
//! These patterns generate pseudo-random values that are deterministic
//! based on time. The same query at the same time always returns the
//! same value, enabling reproducible randomness in patterns.

use super::{Fraction, Pattern, constructors::signal_with_controls};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Generate a deterministic hash from a time value and seed.
fn time_hash(time: &Fraction, seed: u64) -> u64 {
    let mut hasher = DefaultHasher::new();
    seed.hash(&mut hasher);
    time.numer().hash(&mut hasher);
    time.denom().hash(&mut hasher);
    hasher.finish()
}

/// Convert a hash to a float in [0, 1).
fn hash_to_float(hash: u64) -> f64 {
    (hash as f64) / (u64::MAX as f64)
}

/// Generate a deterministic hash based on cycle number and seed.
fn cycle_hash(time: &Fraction, seed: u64) -> u64 {
    let cycle = time.sam();
    time_hash(&cycle, seed)
}

/// A continuous random signal in [0, 1).
///
/// The value changes continuously with time, producing different values
/// at different query times within the same cycle.
///
/// # Example
/// ```ignore
/// let r = rand();
/// // Query at different times gives different values
/// ```
pub fn rand() -> Pattern<f64> {
    signal_with_controls(|t, controls| {
        let hash = time_hash(t, controls.rand_seed);
        hash_to_float(hash)
    })
}

/// A random signal that changes once per cycle.
///
/// All queries within the same cycle return the same random value.
pub fn rand_cycle() -> Pattern<f64> {
    signal_with_controls(|t, controls| {
        let hash = cycle_hash(t, controls.rand_seed);
        hash_to_float(hash)
    })
}

/// Choose randomly from a list of values (changes per cycle).
pub fn choose<T: Clone + Send + Sync + 'static>(values: Vec<T>) -> Pattern<T> {
    if values.is_empty() {
        panic!("choose requires at least one value");
    }
    let len = values.len();
    let rand_pat = rand_cycle();
    rand_pat.fmap(move |r| {
        let idx = (r * len as f64).floor() as usize;
        values[idx.min(len - 1)].clone()
    })
}

/// Choose randomly with weights.
pub fn wchoose<T: Clone + Send + Sync + 'static>(weighted: Vec<(T, f64)>) -> Pattern<T> {
    if weighted.is_empty() {
        panic!("wchoose requires at least one value");
    }

    let total_weight: f64 = weighted.iter().map(|(_, w)| w).sum();
    let values: Vec<T> = weighted.iter().map(|(v, _)| v.clone()).collect();
    let weights: Vec<f64> = weighted.iter().map(|(_, w)| *w).collect();

    rand_cycle().fmap(move |r| {
        let target = r * total_weight;
        let mut acc = 0.0;
        for (i, &w) in weights.iter().enumerate() {
            acc += w;
            if target < acc {
                return values[i].clone();
            }
        }
        values.last().unwrap().clone()
    })
}

impl<T: Clone + Send + Sync + 'static> Pattern<T> {
    /// Randomly drop events with given probability.
    ///
    /// # Arguments
    /// * `prob` - Probability of keeping an event (0.0 to 1.0)
    pub fn degrade_by(&self, prob: f64) -> Pattern<T> {
        let pat = self.clone();
        let rand_pat = rand();

        // Use app_left to preserve structure from self
        pat.app_left(
            &rand_pat,
            move |val, r| {
                if *r < prob { Some(val.clone()) } else { None }
            },
        )
        .filter_values(|v| v.is_some())
        .fmap(|v| v.clone().unwrap())
    }

    /// Randomly replace events with a rest value based on probability.
    ///
    /// Unlike `degrade_by` which filters out events entirely, this method
    /// replaces degraded events with the provided rest value, preserving the
    /// time slot. This is important for sequencers where we want the degraded
    /// slot to be cached rather than re-querying the pattern every tick.
    ///
    /// # Arguments
    /// * `prob` - Probability of keeping the original event (0.0 to 1.0)
    /// * `rest` - Value to use when the event is degraded
    pub fn degrade_by_with_rest(&self, prob: f64, rest: T) -> Pattern<T> {
        let pat = self.clone();
        let rand_pat = rand();

        // Use app_left to preserve structure from self
        pat.app_left(&rand_pat, move |val, r| {
            if *r < prob { val.clone() } else { rest.clone() }
        })
    }

    /// Randomly drop events with 50% probability.
    pub fn degrade(&self) -> Pattern<T> {
        self.degrade_by(0.5)
    }

    /// Randomly replace events with a rest value with 50% probability.
    ///
    /// See `degrade_by_with_rest` for details on why this preserves time slots.
    pub fn degrade_with_rest(&self, rest: T) -> Pattern<T> {
        self.degrade_by_with_rest(0.5, rest)
    }

    /// Randomly remove events, opposite of degrade_by.
    ///
    /// # Arguments
    /// * `prob` - Probability of removing an event (0.0 to 1.0)
    pub fn undegrade_by(&self, prob: f64) -> Pattern<T> {
        self.degrade_by(1.0 - prob)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern_system::constructors::pure;

    #[test]
    fn test_rand_deterministic() {
        let pat = rand();

        // Same query should return same value
        let haps1 = pat.query_arc(Fraction::from_integer(0), Fraction::new(1, 100));
        let haps2 = pat.query_arc(Fraction::from_integer(0), Fraction::new(1, 100));

        assert_eq!(haps1.len(), 1);
        assert_eq!(haps2.len(), 1);
        assert_eq!(haps1[0].value, haps2[0].value);
    }

    #[test]
    fn test_rand_different_times() {
        let pat = rand();

        let haps1 = pat.query_arc(Fraction::from_integer(0), Fraction::new(1, 100));
        let haps2 = pat.query_arc(Fraction::new(1, 2), Fraction::new(51, 100));

        // Different times should (usually) give different values
        // Note: there's a tiny chance they could be equal, but extremely unlikely
        assert_ne!(haps1[0].value, haps2[0].value);
    }

    #[test]
    fn test_rand_in_range() {
        let pat = rand();
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::new(1, 100));

        assert!(haps[0].value >= 0.0);
        assert!(haps[0].value < 1.0);
    }

    #[test]
    fn test_choose() {
        let pat = choose(vec!["a", "b", "c"]);

        // Multiple queries in different cycles
        let mut found = std::collections::HashSet::new();
        for i in 0..20 {
            let haps = pat.query_arc(
                Fraction::from_integer(i),
                Fraction::from_integer(i) + Fraction::new(1, 100),
            );
            if !haps.is_empty() {
                found.insert(haps[0].value);
            }
        }

        // Should eventually find multiple values
        assert!(found.len() > 1, "Choose should produce different values");
    }

    #[test]
    fn test_degrade() {
        let pat = pure(42);

        // Over many cycles, degrade should drop some events
        let mut present_count = 0;
        for i in 0..100 {
            let degraded = pat.degrade();
            let haps = degraded.query_arc(Fraction::from_integer(i), Fraction::from_integer(i + 1));
            if !haps.is_empty() {
                present_count += 1;
            }
        }

        // With 50% probability, should have roughly 50% present
        // Allow wide margin for randomness
        assert!(present_count > 20);
        assert!(present_count < 80);
    }

    #[test]
    fn test_degrade_by_with_rest() {
        let pat = pure(42i32);
        let rest_value = -1i32;

        // With prob=0.0, all events should become rest
        let degraded = pat.degrade_by_with_rest(0.0, rest_value);
        for i in 0..10 {
            let haps = degraded.query_arc(Fraction::from_integer(i), Fraction::from_integer(i + 1));
            // Should always have exactly one hap (the rest)
            assert_eq!(
                haps.len(),
                1,
                "Should have a hap (rest value) at cycle {}",
                i
            );
            assert_eq!(haps[0].value, rest_value, "Value should be the rest value");
        }

        // With prob=1.0, all events should be kept
        let kept = pat.degrade_by_with_rest(1.0, rest_value);
        for i in 0..10 {
            let haps = kept.query_arc(Fraction::from_integer(i), Fraction::from_integer(i + 1));
            assert_eq!(haps.len(), 1, "Should have a hap at cycle {}", i);
            assert_eq!(haps[0].value, 42, "Value should be the original");
        }

        // With prob=0.5, should get a mix (and always have a hap)
        let mixed = pat.degrade_by_with_rest(0.5, rest_value);
        let mut kept_count = 0;
        for i in 0..100 {
            let haps = mixed.query_arc(Fraction::from_integer(i), Fraction::from_integer(i + 1));
            // Should always have exactly one hap
            assert_eq!(
                haps.len(),
                1,
                "Should always have a hap (either value or rest)"
            );
            if haps[0].value == 42 {
                kept_count += 1;
            }
        }
        // With 50% probability, should have roughly 50% kept
        assert!(kept_count > 20, "Should have some kept values");
        assert!(kept_count < 80, "Should have some rest values");
    }
}
