//! Applicative functor operations for pattern combination.
//!
//! These operations combine two patterns using a function, with different
//! strategies for determining the timing structure of the result:
//! - `app_both` - intersection of both patterns' structures
//! - `app_left` - structure from the left (function) pattern
//! - `app_right` - structure from the right (value) pattern

use super::{Fraction, Hap, HapContext, Pattern, State};
use std::sync::Arc;

impl<T: Clone + Send + Sync + 'static> Pattern<T> {
    /// Combine patterns using intersection structure (both patterns contribute).
    ///
    /// The resulting haps occur where both patterns have events, and the
    /// whole span is the intersection of both wholes (or None if either is None).
    pub fn app_both<U, V, F>(&self, pat_val: &Pattern<U>, f: F) -> Pattern<V>
    where
        U: Clone + Send + Sync + 'static,
        V: Clone + Send + Sync + 'static,
        F: Fn(&T, &U) -> V + Send + Sync + 'static,
    {
        let pat_fn = self.clone();
        let pat_val = pat_val.clone();
        let f = Arc::new(f);

        Pattern::new(move |state: &State| {
            let haps_fn = pat_fn.query(state);
            let haps_val = pat_val.query(state);

            let mut result = Vec::new();

            for hap_fn in &haps_fn {
                for hap_val in &haps_val {
                    // Parts must intersect
                    if let Some(part) = hap_fn.part.intersection(&hap_val.part) {
                        // Whole is intersection if both have wholes
                        let whole = match (&hap_fn.whole, &hap_val.whole) {
                            (Some(w1), Some(w2)) => w1.intersection(w2),
                            _ => None,
                        };

                        let value = f(&hap_fn.value, &hap_val.value);
                        let context = HapContext::merge(&hap_fn.context, &hap_val.context);

                        result.push(Hap {
                            whole,
                            part,
                            value,
                            context,
                        });
                    }
                }
            }

            result
        })
    }

    /// Combine patterns using left (inner) structure.
    ///
    /// The timing structure comes from the left pattern. For each hap in the
    /// left pattern, we query the right pattern at that time and combine.
    pub fn app_left<U, V, F>(&self, pat_val: &Pattern<U>, f: F) -> Pattern<V>
    where
        U: Clone + Send + Sync + 'static,
        V: Clone + Send + Sync + 'static,
        F: Fn(&T, &U) -> V + Send + Sync + 'static,
    {
        let pat_fn = self.clone();
        let pat_val = pat_val.clone();
        let f = Arc::new(f);

        Pattern::new(move |state: &State| {
            let haps_fn = pat_fn.query(state);

            let mut result = Vec::new();

            for hap_fn in haps_fn {
                // Query pat_val at the time of this hap
                let query_span = hap_fn.whole_or_part().clone();
                let val_state = state.set_span(query_span);
                let haps_val = pat_val.query(&val_state);

                for hap_val in haps_val {
                    // Part must intersect with the function hap's part
                    if let Some(part) = hap_fn.part.intersection(&hap_val.part) {
                        let value = f(&hap_fn.value, &hap_val.value);
                        // Left structure: use left's whole, merge contexts
                        let context = HapContext::merge(&hap_fn.context, &hap_val.context);

                        result.push(Hap {
                            whole: hap_fn.whole.clone(),
                            part,
                            value,
                            context,
                        });
                    }
                }
            }

            result
        })
    }

    /// Combine patterns using right (outer) structure.
    ///
    /// The timing structure comes from the right pattern. For each hap in the
    /// right pattern, we query the left pattern at that time and combine.
    pub fn app_right<U, V, F>(&self, pat_val: &Pattern<U>, f: F) -> Pattern<V>
    where
        U: Clone + Send + Sync + 'static,
        V: Clone + Send + Sync + 'static,
        F: Fn(&T, &U) -> V + Send + Sync + 'static,
    {
        let pat_fn = self.clone();
        let pat_val = pat_val.clone();
        let f = Arc::new(f);

        Pattern::new(move |state: &State| {
            let haps_val = pat_val.query(state);

            let mut result = Vec::new();

            for hap_val in haps_val {
                // Query pat_fn at the time of this hap
                let query_span = hap_val.whole_or_part().clone();
                let fn_state = state.set_span(query_span);
                let haps_fn = pat_fn.query(&fn_state);

                for hap_fn in haps_fn {
                    // Part must intersect with the value hap's part
                    if let Some(part) = hap_fn.part.intersection(&hap_val.part) {
                        let value = f(&hap_fn.value, &hap_val.value);
                        // Right structure: use right's whole, merge contexts
                        let context = HapContext::merge(&hap_fn.context, &hap_val.context);

                        result.push(Hap {
                            whole: hap_val.whole.clone(),
                            part,
                            value,
                            context,
                        });
                    }
                }
            }

            result
        })
    }

    /// Apply a pattern of functions to this pattern (app_both style).
    pub fn apply<U, F>(&self, pat_fn: &Pattern<F>) -> Pattern<U>
    where
        U: Clone + Send + Sync + 'static,
        F: Fn(&T) -> U + Clone + Send + Sync + 'static,
    {
        let pat_val = self.clone();
        let pat_fn = pat_fn.clone();

        Pattern::new(move |state: &State| {
            let haps_fn = pat_fn.query(state);
            let haps_val = pat_val.query(state);

            let mut result = Vec::new();

            for hap_fn in &haps_fn {
                for hap_val in &haps_val {
                    if let Some(part) = hap_fn.part.intersection(&hap_val.part) {
                        let whole = match (&hap_fn.whole, &hap_val.whole) {
                            (Some(w1), Some(w2)) => w1.intersection(w2),
                            _ => None,
                        };

                        let value = (hap_fn.value)(&hap_val.value);
                        let context = HapContext::merge(&hap_fn.context, &hap_val.context);

                        result.push(Hap {
                            whole,
                            part,
                            value,
                            context,
                        });
                    }
                }
            }

            result
        })
    }
}

/// Convenience functions for common operations
impl<T: Clone + Send + Sync + 'static> Pattern<T> {
    /// Add values from another pattern (app_left structure).
    pub fn add<U>(&self, other: &Pattern<U>) -> Pattern<T>
    where
        T: std::ops::Add<U, Output = T>,
        U: Clone + Send + Sync + 'static,
    {
        self.app_left(other, |a, b| a.clone() + b.clone())
    }

    /// Subtract values from another pattern (app_left structure).
    pub fn sub<U>(&self, other: &Pattern<U>) -> Pattern<T>
    where
        T: std::ops::Sub<U, Output = T>,
        U: Clone + Send + Sync + 'static,
    {
        self.app_left(other, |a, b| a.clone() - b.clone())
    }

    /// Multiply values from another pattern (app_left structure).
    pub fn mul<U>(&self, other: &Pattern<U>) -> Pattern<T>
    where
        T: std::ops::Mul<U, Output = T>,
        U: Clone + Send + Sync + 'static,
    {
        self.app_left(other, |a, b| a.clone() * b.clone())
    }

    /// Divide values from another pattern (app_left structure).
    pub fn div<U>(&self, other: &Pattern<U>) -> Pattern<T>
    where
        T: std::ops::Div<U, Output = T>,
        U: Clone + Send + Sync + 'static,
    {
        self.app_left(other, |a, b| a.clone() / b.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern_system::constructors::{pure, silence};

    #[test]
    fn test_app_both() {
        let pat1 = pure(10);
        let pat2 = pure(3);

        let combined = pat1.app_both(&pat2, |a, b| a + b);
        let haps = combined.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        assert_eq!(haps.len(), 1);
        assert_eq!(haps[0].value, 13);
    }

    #[test]
    fn test_app_left_structure() {
        use crate::pattern_system::combinators::fastcat;

        // Left pattern: [a b] (2 events per cycle)
        let left = fastcat(vec![pure(1), pure(2)]);
        // Right pattern: single event per cycle
        let right = pure(10);

        let combined = left.app_left(&right, |a, b| a + b);
        let haps = combined.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        // Should have 2 events (left structure)
        assert_eq!(haps.len(), 2);
        assert_eq!(haps[0].value, 11);
        assert_eq!(haps[1].value, 12);
    }

    #[test]
    fn test_app_right_structure() {
        use crate::pattern_system::combinators::fastcat;

        // Left pattern: single event
        let left = pure(10);
        // Right pattern: [a b] (2 events per cycle)
        let right = fastcat(vec![pure(1), pure(2)]);

        let combined = left.app_right(&right, |a, b| a + b);
        let haps = combined.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        // Should have 2 events (right structure)
        assert_eq!(haps.len(), 2);
        assert_eq!(haps[0].value, 11);
        assert_eq!(haps[1].value, 12);
    }

    #[test]
    fn test_app_both_with_silence() {
        let pat1 = pure(10);
        let pat2: Pattern<i32> = silence();

        let combined = pat1.app_both(&pat2, |a, b| a + b);
        let haps = combined.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        // No intersection with silence
        assert!(haps.is_empty());
    }

    #[test]
    fn test_add() {
        let pat1 = pure(10.0f64);
        let pat2 = pure(5.0f64);

        let combined = pat1.add(&pat2);
        let haps = combined.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        assert_eq!(haps.len(), 1);
        assert!((haps[0].value - 15.0).abs() < 0.001);
    }
}
