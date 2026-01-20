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
}
