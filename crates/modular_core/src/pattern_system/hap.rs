//! Hap (happening/event) type for pattern events.
//!
//! A Hap represents an event occurrence within a pattern. The key distinction
//! is between `whole` (the full logical extent of the event) and `part`
//! (the portion visible in the current query window).

use super::TimeSpan;

/// Source location in the original pattern string.
/// Used for editor highlighting.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct SourceSpan {
    /// Start offset in the source string (0-indexed).
    pub start: usize,
    /// End offset in the source string (exclusive).
    pub end: usize,
}

impl SourceSpan {
    /// Create a new source span.
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Convert to a tuple for serialization.
    pub fn to_tuple(&self) -> (usize, usize) {
        (self.start, self.end)
    }
}

/// Context information attached to a Hap.
/// Contains source spans for editor highlighting.
#[derive(Clone, Debug, Default)]
pub struct HapContext {
    /// Primary source location (the main atom/value).
    pub source_span: Option<SourceSpan>,
    /// Spans from modifier patterns (scale, add, etc.).
    pub modifier_spans: Vec<SourceSpan>,
}

impl HapContext {
    /// Create an empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a context with a source span.
    pub fn with_span(span: SourceSpan) -> Self {
        Self {
            source_span: Some(span),
            modifier_spans: Vec::new(),
        }
    }

    /// Add a modifier span.
    pub fn add_modifier_span(&mut self, span: SourceSpan) {
        self.modifier_spans.push(span);
    }

    /// Combine two contexts (e.g., when combining haps in applicative operations).
    pub fn combine(&self, other: &HapContext) -> HapContext {
        let mut combined = self.clone();
        // Add the other's source span as a modifier span
        if let Some(span) = &other.source_span {
            combined.modifier_spans.push(span.clone());
        }
        // Add all of other's modifier spans
        combined.modifier_spans.extend(other.modifier_spans.iter().cloned());
        combined
    }

    /// Merge two contexts (alias for combine).
    pub fn merge(left: &HapContext, right: &HapContext) -> HapContext {
        left.combine(right)
    }

    /// Get all spans (source + modifiers) as an iterator.
    pub fn get_all_spans(&self) -> impl Iterator<Item = &SourceSpan> {
        self.source_span.iter().chain(self.modifier_spans.iter())
    }

    /// Get all spans as tuples for JSON serialization.
    pub fn get_all_span_tuples(&self) -> Vec<(usize, usize)> {
        self.get_all_spans().map(|s| s.to_tuple()).collect()
    }
}

/// An event (happening) with temporal extent and value.
///
/// - `whole`: The full logical extent of the event (None for continuous signals)
/// - `part`: The portion of the event visible in the current query
/// - `value`: The event's value
/// - `context`: Metadata including source spans for highlighting
#[derive(Clone, Debug)]
pub struct Hap<T> {
    /// Full logical extent of the event (None for continuous signals).
    pub whole: Option<TimeSpan>,
    /// Portion visible in the current query.
    pub part: TimeSpan,
    /// The event's value.
    pub value: T,
    /// Context (source spans, etc.).
    pub context: HapContext,
}

impl<T: Clone> Hap<T> {
    /// Create a new hap.
    pub fn new(whole: Option<TimeSpan>, part: TimeSpan, value: T) -> Self {
        Self {
            whole,
            part,
            value,
            context: HapContext::new(),
        }
    }

    /// Create a new hap with a source span.
    pub fn with_span(whole: Option<TimeSpan>, part: TimeSpan, value: T, span: SourceSpan) -> Self {
        Self {
            whole,
            part,
            value,
            context: HapContext::with_span(span),
        }
    }

    /// Create a new hap with context.
    pub fn with_context(
        whole: Option<TimeSpan>,
        part: TimeSpan,
        value: T,
        context: HapContext,
    ) -> Self {
        Self {
            whole,
            part,
            value,
            context,
        }
    }

    /// True if this hap includes its onset (start of whole == start of part).
    pub fn has_onset(&self) -> bool {
        match &self.whole {
            Some(whole) => whole.begin == self.part.begin,
            None => false,
        }
    }

    /// Return the whole span if present, otherwise the part span.
    pub fn whole_or_part(&self) -> &TimeSpan {
        self.whole.as_ref().unwrap_or(&self.part)
    }

    /// Apply a function to transform the value.
    pub fn with_value<U, F>(&self, f: F) -> Hap<U>
    where
        F: FnOnce(&T) -> U,
    {
        Hap {
            whole: self.whole.clone(),
            part: self.part.clone(),
            value: f(&self.value),
            context: self.context.clone(),
        }
    }

    /// Apply a function to transform both whole and part spans.
    pub fn with_span_transform<F>(&self, f: F) -> Hap<T>
    where
        F: Fn(&TimeSpan) -> TimeSpan,
    {
        Hap {
            whole: self.whole.as_ref().map(&f),
            part: f(&self.part),
            value: self.value.clone(),
            context: self.context.clone(),
        }
    }

    /// Set the context.
    pub fn set_context(mut self, context: HapContext) -> Self {
        self.context = context;
        self
    }

    /// Add a modifier span to this hap's context.
    pub fn add_modifier_span(mut self, span: SourceSpan) -> Self {
        self.context.add_modifier_span(span);
        self
    }

    /// Combine this hap's context with another hap's context.
    pub fn combine_context<U>(&self, other: &Hap<U>) -> HapContext {
        self.context.combine(&other.context)
    }

    /// Check if this is a discrete hap (has a whole span).
    pub fn is_discrete(&self) -> bool {
        self.whole.is_some()
    }

    /// Check if this is a continuous hap (no whole span).
    pub fn is_continuous(&self) -> bool {
        self.whole.is_none()
    }

    /// Get the duration of the whole span (if present).
    pub fn whole_duration(&self) -> Option<super::Fraction> {
        self.whole.as_ref().map(|w| w.duration())
    }

    /// Get the duration of the part span.
    pub fn part_duration(&self) -> super::Fraction {
        self.part.duration()
    }

    // ===== f64 Fast-Path Methods for DSP =====

    /// Get part begin time as f64.
    #[inline]
    pub fn part_begin_f64(&self) -> f64 {
        self.part.begin_f64()
    }

    /// Get part end time as f64.
    #[inline]
    pub fn part_end_f64(&self) -> f64 {
        self.part.end_f64()
    }

    /// Get whole begin time as f64 (or part begin if no whole).
    #[inline]
    pub fn whole_begin_f64(&self) -> f64 {
        self.whole.as_ref().map_or_else(
            || self.part.begin_f64(),
            |w| w.begin_f64(),
        )
    }

    /// Get whole end time as f64 (or part end if no whole).
    #[inline]
    pub fn whole_end_f64(&self) -> f64 {
        self.whole.as_ref().map_or_else(
            || self.part.end_f64(),
            |w| w.end_f64(),
        )
    }

    /// Check if time t is within the part span [begin, end).
    #[inline]
    pub fn part_contains_f64(&self, t: f64) -> bool {
        t >= self.part_begin_f64() && t < self.part_end_f64()
    }

    /// Check if time t is within the whole span [begin, end).
    /// Returns true for continuous haps if t is in part.
    #[inline]
    pub fn whole_contains_f64(&self, t: f64) -> bool {
        t >= self.whole_begin_f64() && t < self.whole_end_f64()
    }

    /// Check if this hap has its onset at or before time t.
    /// Useful for determining if a note should be triggered.
    #[inline]
    pub fn onset_at_or_before_f64(&self, t: f64) -> bool {
        self.whole_begin_f64() <= t
    }

    /// Convert to a DSP-friendly cached representation.
    pub fn to_dsp_hap(&self) -> DspHap<T> {
        DspHap {
            whole_begin: self.whole_begin_f64(),
            whole_end: self.whole_end_f64(),
            part_begin: self.part_begin_f64(),
            part_end: self.part_end_f64(),
            value: self.value.clone(),
            context: self.context.clone(),
            has_whole: self.whole.is_some(),
        }
    }
}

/// Pre-computed f64 bounds for DSP contexts.
/// Avoids repeated BigRational→f64 conversion in sample-rate loops.
#[derive(Clone, Debug)]
pub struct DspHap<T> {
    /// Whole span begin (or part begin if continuous).
    pub whole_begin: f64,
    /// Whole span end (or part end if continuous).
    pub whole_end: f64,
    /// Part span begin.
    pub part_begin: f64,
    /// Part span end.
    pub part_end: f64,
    /// The event's value.
    pub value: T,
    /// Context (source spans, etc.).
    pub context: HapContext,
    /// Whether this hap has a whole span (discrete vs continuous).
    pub has_whole: bool,
}

impl<T: Clone> DspHap<T> {
    /// Check if time t is within the part span [begin, end).
    #[inline]
    pub fn part_contains(&self, t: f64) -> bool {
        t >= self.part_begin && t < self.part_end
    }

    /// Check if time t is within the whole span [begin, end).
    #[inline]
    pub fn whole_contains(&self, t: f64) -> bool {
        t >= self.whole_begin && t < self.whole_end
    }

    /// Check if this hap has its onset (start of whole) at time t.
    #[inline]
    pub fn has_onset_at(&self, t: f64, epsilon: f64) -> bool {
        self.has_whole && (self.whole_begin - t).abs() < epsilon
    }

    /// Check if this is a discrete hap.
    #[inline]
    pub fn is_discrete(&self) -> bool {
        self.has_whole
    }

    /// Get duration of whole span.
    #[inline]
    pub fn whole_duration(&self) -> f64 {
        self.whole_end - self.whole_begin
    }

    /// Get duration of part span.
    #[inline]
    pub fn part_duration(&self) -> f64 {
        self.part_end - self.part_begin
    }

    /// Get all source spans as tuples for reporting to frontend.
    pub fn get_active_spans(&self) -> Vec<(usize, usize)> {
        self.context.get_all_span_tuples()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern_system::Fraction;

    #[test]
    fn test_hap_has_onset() {
        let whole = TimeSpan::new(Fraction::from_integer(0), Fraction::from_integer(1));
        let part = TimeSpan::new(Fraction::from_integer(0), Fraction::new(1, 2));

        let hap = Hap::new(Some(whole), part, 42);
        assert!(hap.has_onset());
    }

    #[test]
    fn test_hap_no_onset() {
        let whole = TimeSpan::new(Fraction::from_integer(0), Fraction::from_integer(1));
        let part = TimeSpan::new(Fraction::new(1, 2), Fraction::from_integer(1));

        let hap = Hap::new(Some(whole), part, 42);
        assert!(!hap.has_onset());
    }

    #[test]
    fn test_hap_continuous() {
        let part = TimeSpan::new(Fraction::from_integer(0), Fraction::from_integer(1));
        let hap = Hap::new(None, part, 42);

        assert!(!hap.has_onset());
        assert!(hap.is_continuous());
        assert!(!hap.is_discrete());
    }

    #[test]
    fn test_hap_with_value() {
        let whole = TimeSpan::new(Fraction::from_integer(0), Fraction::from_integer(1));
        let part = whole.clone();

        let hap = Hap::new(Some(whole), part, 10);
        let doubled = hap.with_value(|v| v * 2);

        assert_eq!(doubled.value, 20);
    }

    #[test]
    fn test_context_combine() {
        let mut ctx1 = HapContext::with_span(SourceSpan::new(0, 5));
        ctx1.add_modifier_span(SourceSpan::new(10, 15));

        let ctx2 = HapContext::with_span(SourceSpan::new(20, 25));

        let combined = ctx1.combine(&ctx2);

        let spans: Vec<_> = combined.get_all_span_tuples();
        assert_eq!(spans.len(), 3);
        assert_eq!(spans[0], (0, 5)); // Original source span
        assert_eq!(spans[1], (10, 15)); // Original modifier span
        assert_eq!(spans[2], (20, 25)); // Combined from ctx2
    }


    // ===== Fast-Path / DSP Tests =====

    #[test]
    fn test_hap_f64_accessors() {
        let whole = TimeSpan::new(Fraction::new(1, 4), Fraction::new(3, 4));
        let part = TimeSpan::new(Fraction::new(1, 4), Fraction::new(1, 2));
        let hap = Hap::new(Some(whole), part, 42);

        assert!((hap.part_begin_f64() - 0.25).abs() < 1e-10);
        assert!((hap.part_end_f64() - 0.5).abs() < 1e-10);
        assert!((hap.whole_begin_f64() - 0.25).abs() < 1e-10);
        assert!((hap.whole_end_f64() - 0.75).abs() < 1e-10);
    }

    #[test]
    fn test_hap_f64_continuous_fallback() {
        // Continuous hap (no whole span) should use part values for whole accessors
        let part = TimeSpan::new(Fraction::new(1, 3), Fraction::new(2, 3));
        let hap: Hap<i32> = Hap::new(None, part, 42);

        assert!((hap.whole_begin_f64() - hap.part_begin_f64()).abs() < 1e-10);
        assert!((hap.whole_end_f64() - hap.part_end_f64()).abs() < 1e-10);
    }

    #[test]
    fn test_hap_part_contains_f64() {
        let whole = TimeSpan::new(Fraction::from_integer(0), Fraction::from_integer(1));
        let part = TimeSpan::new(Fraction::new(1, 4), Fraction::new(3, 4));
        let hap = Hap::new(Some(whole), part, 42);

        // Inside part range
        assert!(hap.part_contains_f64(0.25)); // start (inclusive)
        assert!(hap.part_contains_f64(0.5));
        assert!(hap.part_contains_f64(0.7499));

        // Outside part range
        assert!(!hap.part_contains_f64(0.0));
        assert!(!hap.part_contains_f64(0.24));
        assert!(!hap.part_contains_f64(0.75)); // end (exclusive)
        assert!(!hap.part_contains_f64(1.0));
    }

    #[test]
    fn test_hap_whole_contains_f64() {
        let whole = TimeSpan::new(Fraction::from_integer(0), Fraction::from_integer(1));
        let part = TimeSpan::new(Fraction::new(1, 4), Fraction::new(3, 4));
        let hap = Hap::new(Some(whole), part, 42);

        // Inside whole range
        assert!(hap.whole_contains_f64(0.0)); // start (inclusive)
        assert!(hap.whole_contains_f64(0.5));
        assert!(hap.whole_contains_f64(0.9999));

        // Outside whole range
        assert!(!hap.whole_contains_f64(-0.1));
        assert!(!hap.whole_contains_f64(1.0)); // end (exclusive)
        assert!(!hap.whole_contains_f64(1.5));
    }

    #[test]
    fn test_hap_onset_at_or_before_f64() {
        let whole = TimeSpan::new(Fraction::new(1, 2), Fraction::from_integer(1));
        let part = whole.clone();
        let hap = Hap::new(Some(whole), part, 42);

        assert!(hap.onset_at_or_before_f64(0.5)); // exactly at onset
        assert!(hap.onset_at_or_before_f64(0.75)); // after onset
        assert!(hap.onset_at_or_before_f64(1.0)); // after onset
        assert!(!hap.onset_at_or_before_f64(0.4)); // before onset
    }

    #[test]
    fn test_to_dsp_hap_discrete() {
        let whole = TimeSpan::new(Fraction::new(1, 4), Fraction::new(3, 4));
        let part = TimeSpan::new(Fraction::new(1, 4), Fraction::new(1, 2));
        let hap = Hap::new(Some(whole), part, 42);
        let dsp = hap.to_dsp_hap();

        assert!((dsp.part_begin - 0.25).abs() < 1e-10);
        assert!((dsp.part_end - 0.5).abs() < 1e-10);
        assert!((dsp.whole_begin - 0.25).abs() < 1e-10);
        assert!((dsp.whole_end - 0.75).abs() < 1e-10);
        assert_eq!(dsp.value, 42);
        assert!(dsp.has_whole);
        assert!(dsp.is_discrete());
    }

    #[test]
    fn test_to_dsp_hap_continuous() {
        let part = TimeSpan::new(Fraction::new(1, 3), Fraction::new(2, 3));
        let hap: Hap<i32> = Hap::new(None, part, 99);
        let dsp = hap.to_dsp_hap();

        // For continuous, whole should equal part
        assert!((dsp.whole_begin - dsp.part_begin).abs() < 1e-10);
        assert!((dsp.whole_end - dsp.part_end).abs() < 1e-10);
        assert_eq!(dsp.value, 99);
        assert!(!dsp.has_whole);
        assert!(!dsp.is_discrete());
    }

    #[test]
    fn test_dsp_hap_part_contains() {
        let whole = TimeSpan::new(Fraction::from_integer(0), Fraction::from_integer(1));
        let part = TimeSpan::new(Fraction::new(1, 4), Fraction::new(3, 4));
        let dsp = Hap::new(Some(whole), part, 42).to_dsp_hap();

        assert!(dsp.part_contains(0.25));
        assert!(dsp.part_contains(0.5));
        assert!(!dsp.part_contains(0.24));
        assert!(!dsp.part_contains(0.75));
    }

    #[test]
    fn test_dsp_hap_whole_contains() {
        let whole = TimeSpan::new(Fraction::from_integer(0), Fraction::from_integer(1));
        let part = TimeSpan::new(Fraction::new(1, 4), Fraction::new(3, 4));
        let dsp = Hap::new(Some(whole), part, 42).to_dsp_hap();

        assert!(dsp.whole_contains(0.0));
        assert!(dsp.whole_contains(0.5));
        assert!(dsp.whole_contains(0.99));
        assert!(!dsp.whole_contains(1.0));
    }

    #[test]
    fn test_dsp_hap_has_onset_at() {
        let whole = TimeSpan::new(Fraction::new(1, 2), Fraction::from_integer(1));
        let part = whole.clone();
        let dsp = Hap::new(Some(whole), part, 42).to_dsp_hap();

        // Should match with small epsilon
        assert!(dsp.has_onset_at(0.5, 1e-9));
        assert!(dsp.has_onset_at(0.500001, 1e-5));
        assert!(!dsp.has_onset_at(0.51, 1e-5));
        assert!(!dsp.has_onset_at(0.0, 1e-5));
    }

    #[test]
    fn test_dsp_hap_has_onset_at_continuous() {
        // Continuous hap has no onset
        let part = TimeSpan::new(Fraction::from_integer(0), Fraction::from_integer(1));
        let dsp: DspHap<i32> = Hap::new(None, part, 42).to_dsp_hap();

        // Continuous haps never have onset
        assert!(!dsp.has_onset_at(0.0, 1e-9));
        assert!(!dsp.has_onset_at(0.5, 1e-9));
    }

    #[test]
    fn test_dsp_hap_durations() {
        let whole = TimeSpan::new(Fraction::from_integer(0), Fraction::from_integer(1));
        let part = TimeSpan::new(Fraction::new(1, 4), Fraction::new(3, 4));
        let dsp = Hap::new(Some(whole), part, 42).to_dsp_hap();

        assert!((dsp.whole_duration() - 1.0).abs() < 1e-10);
        assert!((dsp.part_duration() - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_dsp_hap_preserves_context() {
        let whole = TimeSpan::new(Fraction::from_integer(0), Fraction::from_integer(1));
        let part = whole.clone();

        let mut ctx = HapContext::with_span(SourceSpan::new(10, 20));
        ctx.add_modifier_span(SourceSpan::new(30, 40));

        let hap = Hap::with_context(Some(whole), part, 42, ctx);
        let dsp = hap.to_dsp_hap();

        let spans = dsp.get_active_spans();
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0], (10, 20));
        assert_eq!(spans[1], (30, 40));
    }
}
