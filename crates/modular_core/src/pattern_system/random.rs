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
    rand_with_offset(0)
}

/// Like `rand()`, but with an additional offset mixed into the seed so that
/// two patterns created with different offsets produce independent streams.
pub fn rand_with_offset(offset: u64) -> Pattern<f64> {
    signal_with_controls(move |t, controls| {
        let hash = time_hash(t, controls.rand_seed.wrapping_add(offset));
        hash_to_float(hash)
    })
}

/// A random signal that changes once per cycle.
///
/// All queries within the same cycle return the same random value.
pub fn rand_cycle() -> Pattern<f64> {
    rand_cycle_with_offset(0)
}

/// Like `rand_cycle()`, but with an additional offset mixed into the seed.
pub fn rand_cycle_with_offset(offset: u64) -> Pattern<f64> {
    signal_with_controls(move |t, controls| {
        let hash = cycle_hash(t, controls.rand_seed.wrapping_add(offset));
        hash_to_float(hash)
    })
}

/// Choose randomly from a list of values (changes per cycle).
///
/// Uses seed 0. For independent streams in mini-notation, use `choose_with_seed`.
pub fn choose<T: Clone + Send + Sync + 'static>(values: Vec<T>) -> Pattern<T> {
    choose_with_seed(values, 0)
}

/// Choose randomly from a list of values with a specific seed offset.
///
/// Different seeds produce independent random streams, ensuring that multiple
/// `|` operators in a pattern don't correlate.
pub fn choose_with_seed<T: Clone + Send + Sync + 'static>(
    values: Vec<T>,
    seed: u64,
) -> Pattern<T> {
    if values.is_empty() {
        panic!("choose requires at least one value");
    }
    let len = values.len();
    let rand_pat = rand_cycle_with_offset(seed);
    rand_pat.fmap(move |r| {
        let idx = (r * len as f64).floor() as usize;
        values[idx.min(len - 1)].clone()
    })
}

/// Choose randomly with weights.
///
/// Uses seed 0. For independent streams in mini-notation, use `wchoose_with_seed`.
pub fn wchoose<T: Clone + Send + Sync + 'static>(weighted: Vec<(T, f64)>) -> Pattern<T> {
    wchoose_with_seed(weighted, 0)
}

/// Choose randomly with weights and a specific seed offset.
pub fn wchoose_with_seed<T: Clone + Send + Sync + 'static>(
    weighted: Vec<(T, f64)>,
    seed: u64,
) -> Pattern<T> {
    if weighted.is_empty() {
        panic!("wchoose requires at least one value");
    }

    let total_weight: f64 = weighted.iter().map(|(_, w)| w).sum();
    let values: Vec<T> = weighted.iter().map(|(v, _)| v.clone()).collect();
    let weights: Vec<f64> = weighted.iter().map(|(_, w)| *w).collect();

    rand_cycle_with_offset(seed).fmap(move |r| {
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
    /// Uses seed 0. For independent streams in mini-notation, use `degrade_by_with_seed`.
    ///
    /// # Arguments
    /// * `prob` - Probability of keeping an event (0.0 to 1.0)
    pub fn degrade_by(&self, prob: f64) -> Pattern<T> {
        self.degrade_by_with_seed(prob, 0)
    }

    /// Randomly drop events with given probability and a specific seed offset.
    pub fn degrade_by_with_seed(&self, prob: f64, seed: u64) -> Pattern<T> {
        let pat = self.clone();
        let rand_pat = rand_with_offset(seed);

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
    /// Uses seed 0. For independent streams in mini-notation, use
    /// `degrade_by_with_rest_seeded`.
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
        self.degrade_by_with_rest_seeded(prob, rest, 0)
    }

    /// Randomly replace events with a rest value, using a specific seed offset.
    ///
    /// Different seeds produce independent random streams, ensuring that multiple
    /// `?` operators in a pattern don't correlate.
    pub fn degrade_by_with_rest_seeded(&self, prob: f64, rest: T, seed: u64) -> Pattern<T> {
        let pat = self.clone();
        let rand_pat = rand_with_offset(seed);

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

    #[test]
    fn test_degrade_independence_in_fastcat() {
        // Simulates [0?, 1?, 2?] — three degraded elements in a fastcat.
        // Each should get an independent random stream even though fastcat
        // normalises their inner times to the same values.
        // Uses explicit seeds (as the mini-notation parser would assign).
        use crate::pattern_system::combinators::fastcat;
        use crate::pattern_system::constructors::pure;

        let elements: Vec<Pattern<i32>> = (0..3)
            .map(|i| pure(i).degrade_by_with_rest_seeded(0.5, -1, i as u64))
            .collect();
        let pat = fastcat(elements);

        // Collect keep/drop decisions across many cycles.
        // For each cycle we get 3 events (one per fastcat element).
        // If they were correlated, the 3 decisions within a cycle would
        // always be identical.
        let mut all_same_count = 0;
        let num_cycles = 200;
        for c in 0..num_cycles {
            let haps = pat.query_arc(
                Fraction::from_integer(c),
                Fraction::from_integer(c + 1),
            );
            assert_eq!(haps.len(), 3, "fastcat of 3 should yield 3 haps");
            let decisions: Vec<bool> = haps.iter().map(|h| h.value != -1).collect();
            if decisions[0] == decisions[1] && decisions[1] == decisions[2] {
                all_same_count += 1;
            }
        }
        // With independent 50/50 decisions, P(all same) = 0.25 per cycle.
        // Over 200 cycles expect ~50.  If correlated, all_same_count = 200.
        assert!(
            all_same_count < 100,
            "Degraded elements in fastcat appear correlated: {all_same_count}/200 cycles had all-same decisions"
        );
    }

    #[test]
    fn test_choose_independence_in_fastcat() {
        // Simulates [a|b, a|b] — two random-choice elements in a fastcat.
        // Each should pick independently.
        // Uses explicit seeds (as the mini-notation parser would assign).
        use crate::pattern_system::combinators::fastcat;

        let elements: Vec<Pattern<&str>> = (0..2)
            .map(|i| choose_with_seed(vec!["a", "b"], i as u64))
            .collect();
        let pat = fastcat(elements);

        let mut combos = std::collections::HashMap::<String, usize>::new();
        let num_cycles = 400;
        for c in 0..num_cycles {
            let haps = pat.query_arc(
                Fraction::from_integer(c),
                Fraction::from_integer(c + 1),
            );
            assert_eq!(haps.len(), 2);
            let key = format!("{}{}", haps[0].value, haps[1].value);
            *combos.entry(key).or_default() += 1;
        }
        // With independence, expect ~100 each of aa, ab, ba, bb.
        // If correlated, we'd only see aa and bb.
        assert!(
            combos.len() == 4,
            "Expected all 4 combinations (aa, ab, ba, bb), got: {:?}",
            combos
        );
        for (combo, count) in &combos {
            assert!(
                *count > 50 && *count < 200,
                "Combination {combo} has {count}/400 — expected ~100"
            );
        }
    }

    #[test]
    fn test_deterministic_seeds_from_parse() {
        // Verify that parsing the same pattern twice produces identical
        // seed assignments, and that different patterns get different seeds.
        use crate::pattern_system::mini::parser::parse;

        let ast1 = parse("a? b?").unwrap();
        let ast2 = parse("a? b?").unwrap();
        // Same input → identical AST (including seeds)
        assert_eq!(ast1, ast2, "Same pattern should produce identical ASTs");

        // Verify seeds are distinct within the pattern
        if let crate::pattern_system::mini::ast::MiniAST::Sequence(elements) = &ast1 {
            if let (crate::pattern_system::mini::ast::MiniAST::Degrade(_, _, seed0), _) = &elements[0] {
                if let (crate::pattern_system::mini::ast::MiniAST::Degrade(_, _, seed1), _) = &elements[1] {
                    assert_ne!(seed0, seed1, "Different ? operators should get different seeds");
                }
            }
        }
    }
}
