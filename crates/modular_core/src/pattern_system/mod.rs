//! Strudel-style pattern system for generating time-varying values.
//!
//! This module provides a functional reactive programming framework for
//! representing cyclic, time-varying patterns. At its core, a `Pattern<T>`
//! is a lazy query function that generates events (Haps) on demand for
//! any requested time range.
//!
//! # Key Concepts
//!
//! - **Pattern<T>**: A query function `State → Vec<Hap<T>>` that generates events lazily
//! - **Hap<T>**: An event with `whole` (full extent) and `part` (visible portion)
//! - **TimeSpan**: Half-open interval `[begin, end)` using exact rational time
//! - **Fraction**: Exact rational numbers for precise time (avoids float drift)
//!
//! # Example
//!
//! ```ignore
//! use modular_core::pattern_system::{Pattern, Fraction, pure, fastcat};
//!
//! // A pattern that cycles through 0, 1, 2 each cycle
//! let pat = fastcat(vec![pure(0.0), pure(1.0), pure(2.0)]);
//!
//! // Query for events in cycle 0
//! let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
//! assert_eq!(haps.len(), 3);
//! ```

mod fraction;
mod hap;
mod state;
mod timespan;

pub mod constructors;
pub mod combinators;
pub mod temporal;
pub mod applicative;
pub mod monadic;
pub mod operators;
pub mod random;
pub mod euclidean;
pub mod mini;

pub use fraction::Fraction;
pub use hap::{Hap, HapContext, SourceSpan};
pub use state::{Controls, State};
pub use timespan::TimeSpan;

pub use constructors::{pure, pure_with_span, silence, signal};
pub use combinators::{fastcat, slowcat, stack, timecat};

use std::sync::Arc;

/// The query function type: takes a State, returns events.
pub type QueryFn<T> = Arc<dyn Fn(&State) -> Vec<Hap<T>> + Send + Sync>;

/// A pattern is a lazy, query-based generator of time-varying values.
///
/// Patterns don't store events - they generate them on demand when queried.
/// This enables infinite, cyclic patterns that can be composed, transformed,
/// and combined without materializing the entire timeline.
#[derive(Clone)]
pub struct Pattern<T> {
    /// The query function that generates events.
    query: QueryFn<T>,
    /// Number of steps per cycle (for alignment operations).
    steps: Option<Fraction>,
}

impl<T: Clone + Send + Sync + 'static> Pattern<T> {
    /// Create a pattern from a query function.
    pub fn new<F>(query: F) -> Self
    where
        F: Fn(&State) -> Vec<Hap<T>> + Send + Sync + 'static,
    {
        Pattern {
            query: Arc::new(query),
            steps: None,
        }
    }

    /// Query the pattern for haps within the state's time span.
    pub fn query(&self, state: &State) -> Vec<Hap<T>> {
        (self.query)(state)
    }

    /// Query for a specific time range.
    pub fn query_arc(&self, begin: Fraction, end: Fraction) -> Vec<Hap<T>> {
        self.query(&State::new(TimeSpan::new(begin, end)))
    }

    /// Get the number of steps per cycle (if set).
    pub fn steps(&self) -> Option<&Fraction> {
        self.steps.as_ref()
    }

    /// Set the number of steps per cycle.
    pub fn with_steps(mut self, steps: Fraction) -> Self {
        self.steps = Some(steps);
        self
    }

    /// Set the number of steps per cycle (internal mutable version).
    pub fn set_steps(&mut self, steps: Fraction) {
        self.steps = Some(steps);
    }

    // ===== Functor Operations =====

    /// Map a function over the values (functor fmap).
    pub fn fmap<U, F>(&self, f: F) -> Pattern<U>
    where
        U: Clone + Send + Sync + 'static,
        F: Fn(&T) -> U + Clone + Send + Sync + 'static,
    {
        let query = self.query.clone();
        let steps = self.steps.clone();
        let mut result = Pattern::new(move |state| {
            query(state)
                .into_iter()
                .map(|hap| hap.with_value(&f))
                .collect()
        });
        if let Some(s) = steps {
            result.steps = Some(s);
        }
        result
    }

    /// Alias for fmap.
    pub fn with_value<U, F>(&self, f: F) -> Pattern<U>
    where
        U: Clone + Send + Sync + 'static,
        F: Fn(&T) -> U + Clone + Send + Sync + 'static,
    {
        self.fmap(f)
    }

    // ===== Query Transformations =====

    /// Transform the query span before querying.
    pub fn with_query_span<F>(&self, f: F) -> Pattern<T>
    where
        F: Fn(&TimeSpan) -> TimeSpan + Clone + Send + Sync + 'static,
    {
        let query = self.query.clone();
        let steps = self.steps.clone();
        let mut result = Pattern::new(move |state| {
            let new_span = f(&state.span);
            query(&state.set_span(new_span))
        });
        if let Some(s) = steps {
            result.steps = Some(s);
        }
        result
    }

    /// Transform query time (both begin and end).
    pub fn with_query_time<F>(&self, f: F) -> Pattern<T>
    where
        F: Fn(&Fraction) -> Fraction + Clone + Send + Sync + 'static,
    {
        let f_clone = f.clone();
        self.with_query_span(move |span| span.with_time(|t| f_clone(t)))
    }

    /// Transform hap spans after querying.
    pub fn with_hap_span<F>(&self, f: F) -> Pattern<T>
    where
        F: Fn(&TimeSpan) -> TimeSpan + Clone + Send + Sync + 'static,
    {
        let query = self.query.clone();
        let steps = self.steps.clone();
        let mut result = Pattern::new(move |state| {
            query(state)
                .into_iter()
                .map(|hap| hap.with_span_transform(&f))
                .collect()
        });
        if let Some(s) = steps {
            result.steps = Some(s);
        }
        result
    }

    /// Transform hap times (both begin and end of both whole and part).
    pub fn with_hap_time<F>(&self, f: F) -> Pattern<T>
    where
        F: Fn(&Fraction) -> Fraction + Clone + Send + Sync + 'static,
    {
        let f_clone = f.clone();
        self.with_hap_span(move |span| span.with_time(|t| f_clone(t)))
    }

    /// Split queries at cycle boundaries.
    ///
    /// This ensures each sub-query is contained within a single cycle,
    /// which is necessary for operations like `rev` that work per-cycle.
    pub fn split_queries(&self) -> Pattern<T> {
        let query = self.query.clone();
        let steps = self.steps.clone();
        let mut result = Pattern::new(move |state| {
            state
                .span
                .span_cycles()
                .into_iter()
                .flat_map(|subspan| query(&state.set_span(subspan)))
                .collect()
        });
        if let Some(s) = steps {
            result.steps = Some(s);
        }
        result
    }

    // ===== Filtering =====

    /// Filter haps by a predicate.
    pub fn filter_haps<F>(&self, pred: F) -> Pattern<T>
    where
        F: Fn(&Hap<T>) -> bool + Clone + Send + Sync + 'static,
    {
        let query = self.query.clone();
        let steps = self.steps.clone();
        let mut result = Pattern::new(move |state| {
            query(state)
                .into_iter()
                .filter(|hap| pred(hap))
                .collect()
        });
        if let Some(s) = steps {
            result.steps = Some(s);
        }
        result
    }

    /// Filter haps by value.
    pub fn filter_values<F>(&self, pred: F) -> Pattern<T>
    where
        F: Fn(&T) -> bool + Clone + Send + Sync + 'static,
    {
        self.filter_haps(move |hap| pred(&hap.value))
    }

    /// Keep only discrete haps (those with whole spans).
    pub fn discrete_only(&self) -> Pattern<T> {
        self.filter_haps(|hap| hap.is_discrete())
    }

    /// Keep only haps with onsets (where part.begin == whole.begin).
    pub fn onsets_only(&self) -> Pattern<T> {
        self.filter_haps(|hap| hap.has_onset())
    }

    /// Keep only continuous haps (those without whole spans).
    pub fn continuous_only(&self) -> Pattern<T> {
        self.filter_haps(|hap| hap.is_continuous())
    }
}

impl<T: Clone + Send + Sync + 'static> std::fmt::Debug for Pattern<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Pattern {{ steps: {:?} }}", self.steps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pure_pattern() {
        let pat = pure(42);
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        assert_eq!(haps.len(), 1);
        assert_eq!(haps[0].value, 42);
        assert!(haps[0].has_onset());
    }

    #[test]
    fn test_silence_pattern() {
        let pat: Pattern<i32> = silence();
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        assert_eq!(haps.len(), 0);
    }

    #[test]
    fn test_fmap() {
        let pat = pure(10);
        let doubled = pat.fmap(|x| x * 2);
        let haps = doubled.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        assert_eq!(haps.len(), 1);
        assert_eq!(haps[0].value, 20);
    }

    #[test]
    fn test_fastcat() {
        let pat = fastcat(vec![pure(0), pure(1), pure(2)]);
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        assert_eq!(haps.len(), 3);
        assert_eq!(haps[0].value, 0);
        assert_eq!(haps[1].value, 1);
        assert_eq!(haps[2].value, 2);
    }

    #[test]
    fn test_stack() {
        let pat = stack(vec![pure(0), pure(1)]);
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        assert_eq!(haps.len(), 2);
        // Both values should be present (order may vary)
        let values: Vec<_> = haps.iter().map(|h| h.value).collect();
        assert!(values.contains(&0));
        assert!(values.contains(&1));
    }

    #[test]
    fn test_slowcat() {
        let pat = slowcat(vec![pure(0), pure(1), pure(2)]);

        // Cycle 0 should have value 0
        let haps0 = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps0.len(), 1);
        assert_eq!(haps0[0].value, 0);

        // Cycle 1 should have value 1
        let haps1 = pat.query_arc(Fraction::from_integer(1), Fraction::from_integer(2));
        assert_eq!(haps1.len(), 1);
        assert_eq!(haps1[0].value, 1);

        // Cycle 2 should have value 2
        let haps2 = pat.query_arc(Fraction::from_integer(2), Fraction::from_integer(3));
        assert_eq!(haps2.len(), 1);
        assert_eq!(haps2[0].value, 2);

        // Cycle 3 should wrap back to 0
        let haps3 = pat.query_arc(Fraction::from_integer(3), Fraction::from_integer(4));
        assert_eq!(haps3.len(), 1);
        assert_eq!(haps3[0].value, 0);
    }
}
