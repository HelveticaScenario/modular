//! Deterministic pseudo-random patterns.
//!
//! These patterns generate pseudo-random values that are deterministic
//! based on time. The same query at the same time always returns the
//! same value, enabling reproducible randomness in patterns.

use super::{constructors::signal_with_controls, Fraction, Pattern};
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

/// Random integer in range [min, max].
pub fn irand(min: i64, max: i64) -> Pattern<i64> {
    rand().fmap(move |r| {
        let range = (max - min + 1) as f64;
        min + (r * range).floor() as i64
    })
}

/// Random integer that changes once per cycle.
pub fn irand_cycle(min: i64, max: i64) -> Pattern<i64> {
    rand_cycle().fmap(move |r| {
        let range = (max - min + 1) as f64;
        min + (r * range).floor() as i64
    })
}

/// Random float in range [min, max].
pub fn frand(min: f64, max: f64) -> Pattern<f64> {
    rand().fmap(move |r| min + r * (max - min))
}

/// Random float that changes once per cycle.
pub fn frand_cycle(min: f64, max: f64) -> Pattern<f64> {
    rand_cycle().fmap(move |r| min + r * (max - min))
}

/// Choose randomly from a list of values (changes per cycle).
pub fn choose<T: Clone + Send + Sync + 'static>(values: Vec<T>) -> Pattern<T> {
    if values.is_empty() {
        panic!("choose requires at least one value");
    }
    let len = values.len();
    irand_cycle(0, (len - 1) as i64).fmap(move |i| values[*i as usize].clone())
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
        pat.app_left(&rand_pat, move |val, r| {
            if *r < prob {
                Some(val.clone())
            } else {
                None
            }
        })
        .filter_values(|v| v.is_some())
        .fmap(|v| v.clone().unwrap())
    }

    /// Randomly drop events with 50% probability.
    pub fn degrade(&self) -> Pattern<T> {
        self.degrade_by(0.5)
    }

    /// Randomly remove events, opposite of degrade_by.
    ///
    /// # Arguments
    /// * `prob` - Probability of removing an event (0.0 to 1.0)
    pub fn undegrade_by(&self, prob: f64) -> Pattern<T> {
        self.degrade_by(1.0 - prob)
    }

    /// Sometimes apply a function based on probability.
    pub fn sometimes_by<F>(&self, prob: f64, f: F) -> Pattern<T>
    where
        F: Fn(&Pattern<T>) -> Pattern<T> + Send + Sync + 'static,
    {
        // This is a simplification - true implementation would
        // randomly choose which events to transform
        if prob <= 0.0 {
            return self.clone();
        }
        if prob >= 1.0 {
            return f(self);
        }

        // Mix original and transformed based on probability
        let original = self.clone();
        let transformed = f(self);

        // Create a random pattern to decide which to use
        let rand_pat = rand_cycle();

        Pattern::new(move |state| {
            let r_haps = rand_pat.query(state);
            let r = if r_haps.is_empty() {
                0.5
            } else {
                r_haps[0].value
            };

            if r < prob {
                transformed.query(state)
            } else {
                original.query(state)
            }
        })
    }

    /// Apply function with 50% probability.
    pub fn sometimes<F>(&self, f: F) -> Pattern<T>
    where
        F: Fn(&Pattern<T>) -> Pattern<T> + Send + Sync + 'static,
    {
        self.sometimes_by(0.5, f)
    }

    /// Apply function rarely (25% probability).
    pub fn rarely<F>(&self, f: F) -> Pattern<T>
    where
        F: Fn(&Pattern<T>) -> Pattern<T> + Send + Sync + 'static,
    {
        self.sometimes_by(0.25, f)
    }

    /// Apply function often (75% probability).
    pub fn often<F>(&self, f: F) -> Pattern<T>
    where
        F: Fn(&Pattern<T>) -> Pattern<T> + Send + Sync + 'static,
    {
        self.sometimes_by(0.75, f)
    }

    /// Apply function almost always (90% probability).
    pub fn almost_always<F>(&self, f: F) -> Pattern<T>
    where
        F: Fn(&Pattern<T>) -> Pattern<T> + Send + Sync + 'static,
    {
        self.sometimes_by(0.9, f)
    }

    /// Apply function almost never (10% probability).
    pub fn almost_never<F>(&self, f: F) -> Pattern<T>
    where
        F: Fn(&Pattern<T>) -> Pattern<T> + Send + Sync + 'static,
    {
        self.sometimes_by(0.1, f)
    }

    /// Shuffle the events within each cycle randomly.
    pub fn shuffle(&self) -> Pattern<T> {
        // This is a complex operation - for now, just rotate randomly
        let pat = self.clone();

        Pattern::new(move |state| {
            let mut haps = pat.query(state);
            if haps.len() <= 1 {
                return haps;
            }

            // Use cycle number as seed for consistent shuffle per cycle
            let cycle = state.span.begin.sam();
            let seed = time_hash(&cycle, 42);

            // Fisher-Yates shuffle with deterministic random
            let n = haps.len();
            for i in 0..n - 1 {
                let mut hasher = DefaultHasher::new();
                seed.hash(&mut hasher);
                i.hash(&mut hasher);
                let j = i + (hasher.finish() as usize % (n - i));
                haps.swap(i, j);
            }

            haps
        })
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
    fn test_irand() {
        let pat = irand(1, 6);
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::new(1, 100));

        assert!(haps[0].value >= 1);
        assert!(haps[0].value <= 6);
    }

    #[test]
    fn test_choose() {
        let pat = choose(vec!["a", "b", "c"]);

        // Multiple queries in different cycles
        let mut found = std::collections::HashSet::new();
        for i in 0..20 {
            let haps =
                pat.query_arc(Fraction::from_integer(i), Fraction::from_integer(i) + Fraction::new(1, 100));
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
            let haps =
                degraded.query_arc(Fraction::from_integer(i), Fraction::from_integer(i + 1));
            if !haps.is_empty() {
                present_count += 1;
            }
        }

        // With 50% probability, should have roughly 50% present
        // Allow wide margin for randomness
        assert!(present_count > 20);
        assert!(present_count < 80);
    }
}
