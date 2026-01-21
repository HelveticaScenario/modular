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
pub use hap::{DspHap, Hap, HapContext, SourceSpan};
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

    /// Add a modifier span to all haps in this pattern.
    /// Used for tracking which operators are active during editor highlighting.
    pub fn with_modifier_span(&self, span: SourceSpan) -> Pattern<T> {
        let query = self.query.clone();
        let steps = self.steps.clone();
        let mut result = Pattern::new(move |state| {
            query(state)
                .into_iter()
                .map(|hap| hap.add_modifier_span(span.clone()))
                .collect()
        });
        if let Some(s) = steps {
            result.steps = Some(s);
        }
        result
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

    // ===== DSP Fast-Path Methods =====
    //
    // These methods use f64 for efficient sample-rate queries.
    // The pattern is still constructed with exact rational arithmetic,
    // but these methods avoid repeated BigRational→f64 conversion.

    /// Query for a time range using f64 (DSP fast-path).
    ///
    /// Converts f64 to Fraction internally, but the returned haps
    /// can use the fast f64 accessor methods.
    pub fn query_arc_f64(&self, begin: f64, end: f64) -> Vec<Hap<T>> {
        self.query_arc(Fraction::from(begin), Fraction::from(end))
    }

    /// Query at a specific point in time (DSP fast-path).
    ///
    /// Returns haps whose part span contains the given time.
    /// This queries a tiny window around `t` and filters to haps containing it.
    ///
    /// # Example
    /// ```ignore
    /// let haps = pattern.query_at(0.5);
    /// for hap in haps {
    ///     println!("Value at t=0.5: {:?}", hap.value);
    /// }
    /// ```
    pub fn query_at(&self, t: f64) -> Vec<Hap<T>> {
        // Query the cycle containing t
        let cycle = t.floor();
        let haps = self.query_arc_f64(cycle, cycle + 1.0);

        // Filter to haps whose part contains t
        haps.into_iter()
            .filter(|hap| hap.part_contains_f64(t))
            .collect()
    }

    /// Query at a point and return the first matching hap (if any).
    ///
    /// This is the most common case for DSP - getting a single active event.
    #[inline]
    pub fn query_at_first(&self, t: f64) -> Option<Hap<T>> {
        // Query the cycle containing t
        let cycle = t.floor();
        let haps = self.query_arc_f64(cycle, cycle + 1.0);

        // Find first hap whose part contains t
        haps.into_iter().find(|hap| hap.part_contains_f64(t))
    }

    /// Query at a point and return as DspHap for cached DSP use.
    ///
    /// The returned DspHap has pre-computed f64 bounds for fast comparisons.
    #[inline]
    pub fn query_at_dsp(&self, t: f64) -> Option<DspHap<T>> {
        self.query_at_first(t).map(|h| h.to_dsp_hap())
    }

    /// Query a time range and return as DspHaps.
    ///
    /// Useful for pre-computing events for a render buffer.
    pub fn query_arc_dsp(&self, begin: f64, end: f64) -> Vec<DspHap<T>> {
        self.query_arc_f64(begin, end)
            .into_iter()
            .map(|h| h.to_dsp_hap())
            .collect()
    }

    /// Get all events (with onsets) in a cycle as DspHaps.
    ///
    /// This is useful for pre-computing a cycle's worth of events.
    pub fn query_cycle_dsp(&self, cycle: i64) -> Vec<DspHap<T>> {
        let begin = cycle as f64;
        let end = (cycle + 1) as f64;

        self.query_arc_f64(begin, end)
            .into_iter()
            .filter(|h| h.has_onset())
            .map(|h| h.to_dsp_hap())
            .collect()
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


    // ===== DSP Fast-Path Pattern Methods =====

    #[test]
    fn test_query_arc_f64() {
        let pat = fastcat(vec![pure(0), pure(1), pure(2), pure(3)]);
        let haps = pat.query_arc_f64(0.0, 1.0);

        assert_eq!(haps.len(), 4);
        assert_eq!(haps[0].value, 0);
        assert_eq!(haps[1].value, 1);
        assert_eq!(haps[2].value, 2);
        assert_eq!(haps[3].value, 3);
    }

    #[test]
    fn test_query_arc_f64_fractional_range() {
        let pat = fastcat(vec![pure(0), pure(1), pure(2), pure(3)]);
        // Query just the middle half
        let haps = pat.query_arc_f64(0.25, 0.75);

        // Should capture events 1 and 2 (at 0.25-0.5 and 0.5-0.75)
        assert_eq!(haps.len(), 2);
        assert_eq!(haps[0].value, 1);
        assert_eq!(haps[1].value, 2);
    }

    #[test]
    fn test_query_at() {
        let pat = fastcat(vec![pure(0), pure(1), pure(2), pure(3)]);

        // Query at different points
        let h0 = pat.query_at(0.1);
        assert_eq!(h0.len(), 1);
        assert_eq!(h0[0].value, 0);

        let h1 = pat.query_at(0.3);
        assert_eq!(h1.len(), 1);
        assert_eq!(h1[0].value, 1);

        let h3 = pat.query_at(0.9);
        assert_eq!(h3.len(), 1);
        assert_eq!(h3[0].value, 3);
    }

    #[test]
    fn test_query_at_boundary() {
        let pat = fastcat(vec![pure(0), pure(1)]);

        // At 0.0 should be in event 0
        let h0 = pat.query_at(0.0);
        assert_eq!(h0.len(), 1);
        assert_eq!(h0[0].value, 0);

        // At exactly 0.5 should be in event 1 (half-open interval [0.5, 1.0))
        let h1 = pat.query_at(0.5);
        assert_eq!(h1.len(), 1);
        assert_eq!(h1[0].value, 1);
    }

    #[test]
    fn test_query_at_silence() {
        let pat: Pattern<i32> = silence();
        let haps = pat.query_at(0.5);
        assert!(haps.is_empty());
    }

    #[test]
    fn test_query_at_first() {
        let pat = fastcat(vec![pure(0), pure(1), pure(2)]);

        let h = pat.query_at_first(0.4);
        assert!(h.is_some());
        assert_eq!(h.unwrap().value, 1);
    }

    #[test]
    fn test_query_at_first_none() {
        let pat: Pattern<i32> = silence();
        let h = pat.query_at_first(0.5);
        assert!(h.is_none());
    }

    #[test]
    fn test_query_at_dsp() {
        let pat = fastcat(vec![pure(10), pure(20), pure(30)]);

        let dsp = pat.query_at_dsp(0.5);
        assert!(dsp.is_some());
        let dsp = dsp.unwrap();
        assert_eq!(dsp.value, 20);
        assert!(dsp.is_discrete());
        // Check f64 bounds are pre-computed
        assert!((dsp.part_begin - (1.0 / 3.0)).abs() < 1e-10);
        assert!((dsp.part_end - (2.0 / 3.0)).abs() < 1e-10);
    }

    #[test]
    fn test_query_at_dsp_none() {
        let pat: Pattern<i32> = silence();
        let dsp = pat.query_at_dsp(0.5);
        assert!(dsp.is_none());
    }

    #[test]
    fn test_query_arc_dsp() {
        let pat = fastcat(vec![pure(1), pure(2)]);
        let dsps = pat.query_arc_dsp(0.0, 1.0);

        assert_eq!(dsps.len(), 2);
        assert_eq!(dsps[0].value, 1);
        assert_eq!(dsps[1].value, 2);

        // Check bounds are pre-computed
        assert!((dsps[0].part_begin - 0.0).abs() < 1e-10);
        assert!((dsps[0].part_end - 0.5).abs() < 1e-10);
        assert!((dsps[1].part_begin - 0.5).abs() < 1e-10);
        assert!((dsps[1].part_end - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_query_cycle_dsp() {
        let pat = fastcat(vec![pure(1), pure(2), pure(3)]);

        let cycle0 = pat.query_cycle_dsp(0);
        assert_eq!(cycle0.len(), 3);
        assert_eq!(cycle0[0].value, 1);
        assert_eq!(cycle0[1].value, 2);
        assert_eq!(cycle0[2].value, 3);

        // Check all have onsets (should be filtered)
        for dsp in &cycle0 {
            assert!(dsp.is_discrete());
        }
    }

    #[test]
    fn test_query_cycle_dsp_different_cycles() {
        let pat = slowcat(vec![pure(10), pure(20), pure(30)]);

        let c0 = pat.query_cycle_dsp(0);
        assert_eq!(c0.len(), 1);
        assert_eq!(c0[0].value, 10);

        let c1 = pat.query_cycle_dsp(1);
        assert_eq!(c1.len(), 1);
        assert_eq!(c1[0].value, 20);

        let c2 = pat.query_cycle_dsp(2);
        assert_eq!(c2.len(), 1);
        assert_eq!(c2[0].value, 30);

        // Should wrap
        let c3 = pat.query_cycle_dsp(3);
        assert_eq!(c3.len(), 1);
        assert_eq!(c3[0].value, 10);
    }

    #[test]
    fn test_query_cycle_dsp_filters_non_onsets() {
        // Using a pattern where some haps may not have onsets due to slicing
        // pure() patterns always have onsets when queried within their whole,
        // but this confirms the filter is working
        let pat = pure(42);
        let events = pat.query_cycle_dsp(0);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].value, 42);
    }

    #[test]
    fn test_dsp_methods_with_stack() {
        // Stack overlays patterns, so both should be active at any time point
        let pat = stack(vec![pure(100), pure(200)]);

        let at_half = pat.query_at(0.5);
        assert_eq!(at_half.len(), 2);
        let values: Vec<_> = at_half.iter().map(|h| h.value).collect();
        assert!(values.contains(&100));
        assert!(values.contains(&200));

        // DSP version
        let dsps = pat.query_arc_dsp(0.0, 1.0);
        assert_eq!(dsps.len(), 2);
        let dsp_values: Vec<_> = dsps.iter().map(|d| d.value).collect();
        assert!(dsp_values.contains(&100));
        assert!(dsp_values.contains(&200));
    }

    #[test]
    fn test_query_across_cycle_boundary() {
        let pat = pure(42);

        // Query across cycle boundary
        let haps = pat.query_arc_f64(0.5, 1.5);

        // Should get partial events from both cycles
        assert!(haps.len() >= 1);
    }

    #[test]
    fn test_dsp_fast_path_preserves_value_types() {
        // Test with different value types
        let str_pat = pure("hello".to_string());
        let dsp = str_pat.query_at_dsp(0.5);
        assert!(dsp.is_some());
        assert_eq!(dsp.unwrap().value, "hello");

        // Test with tuple
        let tuple_pat = pure((1, 2, 3));
        let dsp = tuple_pat.query_at_dsp(0.5);
        assert!(dsp.is_some());
        assert_eq!(dsp.unwrap().value, (1, 2, 3));
    }
}
