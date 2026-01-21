//! Half-open time intervals for representing event durations.
//!
//! A TimeSpan represents an interval [begin, end) using exact rational numbers.
//! Key operations include splitting spans at cycle boundaries and computing intersections.

use super::Fraction;

/// Half-open time interval [begin, end).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TimeSpan {
    pub begin: Fraction,
    pub end: Fraction,
}

impl TimeSpan {
    /// Create a new timespan from begin and end fractions.
    pub fn new(begin: Fraction, end: Fraction) -> Self {
        Self { begin, end }
    }

    /// Create a timespan from integer values.
    pub fn from_integers(begin: i64, end: i64) -> Self {
        Self {
            begin: Fraction::from_integer(begin),
            end: Fraction::from_integer(end),
        }
    }

    /// Duration of this span (end - begin).
    pub fn duration(&self) -> Fraction {
        &self.end - &self.begin
    }

    /// Split a span at cycle boundaries.
    ///
    /// Returns a list of spans, each contained within a single cycle.
    /// For example, [0.5, 2.3) becomes [[0.5, 1), [1, 2), [2, 2.3)].
    pub fn span_cycles(&self) -> Vec<TimeSpan> {
        let mut spans = Vec::new();
        let mut begin = self.begin.clone();

        // Handle zero-width (point) spans
        if begin == self.end {
            return vec![TimeSpan::new(begin, self.end.clone())];
        }

        // Handle reverse spans (shouldn't happen but be defensive)
        if begin > self.end {
            return spans;
        }

        let end_sam = self.end.sam();

        while self.end > begin {
            if begin.sam() == end_sam {
                // We're in the final cycle
                spans.push(TimeSpan::new(begin, self.end.clone()));
                break;
            }
            let next_begin = begin.next_sam();
            spans.push(TimeSpan::new(begin.clone(), next_begin.clone()));
            begin = next_begin;
        }

        spans
    }

    /// Intersection of two spans, returns None if disjoint.
    ///
    /// Handles zero-width (point) intersections specially.
    pub fn intersection(&self, other: &TimeSpan) -> Option<TimeSpan> {
        let intersect_begin = self.begin.max_of(&other.begin);
        let intersect_end = self.end.min_of(&other.end);

        if intersect_begin > intersect_end {
            return None;
        }

        // Handle zero-width (point) intersection
        if intersect_begin == intersect_end {
            // Don't allow point intersection at the exclusive end of either span
            if intersect_begin == self.end && self.begin < self.end {
                return None;
            }
            if intersect_begin == other.end && other.begin < other.end {
                return None;
            }
        }

        Some(TimeSpan::new(intersect_begin, intersect_end))
    }

    /// Returns the intersection or panics if none exists.
    pub fn intersection_e(&self, other: &TimeSpan) -> TimeSpan {
        self.intersection(other)
            .expect("TimeSpan intersection failed - spans are disjoint")
    }

    /// Apply a function to both begin and end times.
    pub fn with_time<F>(&self, f: F) -> TimeSpan
    where
        F: Fn(&Fraction) -> Fraction,
    {
        TimeSpan::new(f(&self.begin), f(&self.end))
    }

    /// Shift span to cycle zero while preserving duration.
    /// Returns a span starting at the cycle position within [0, 1).
    pub fn cycle_arc(&self) -> TimeSpan {
        let b = self.begin.cycle_pos();
        let e = &b + &self.duration();
        TimeSpan::new(b, e)
    }

    /// Check if a time is within this span [begin, end).
    pub fn contains(&self, time: &Fraction) -> bool {
        time >= &self.begin && time < &self.end
    }

    /// Check if this span is zero-width (a point).
    pub fn is_point(&self) -> bool {
        self.begin == self.end
    }

    /// Midpoint of the span.
    pub fn midpoint(&self) -> Fraction {
        (&self.begin + &self.end) / Fraction::from_integer(2)
    }

    // ===== f64 Fast-Path Methods for DSP =====

    /// Get begin time as f64 (for fast DSP comparisons).
    #[inline]
    pub fn begin_f64(&self) -> f64 {
        self.begin.to_f64()
    }

    /// Get end time as f64 (for fast DSP comparisons).
    #[inline]
    pub fn end_f64(&self) -> f64 {
        self.end.to_f64()
    }

    /// Get duration as f64.
    #[inline]
    pub fn duration_f64(&self) -> f64 {
        self.end_f64() - self.begin_f64()
    }

    /// Fast containment check using f64.
    /// Checks if time t is within [begin, end).
    #[inline]
    pub fn contains_f64(&self, t: f64) -> bool {
        t >= self.begin_f64() && t < self.end_f64()
    }
}

impl Default for TimeSpan {
    fn default() -> Self {
        TimeSpan::new(Fraction::from_integer(0), Fraction::from_integer(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_cycles_single_cycle() {
        let span = TimeSpan::new(Fraction::new(1, 4), Fraction::new(3, 4));
        let cycles = span.span_cycles();
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0], span);
    }

    #[test]
    fn test_span_cycles_multi_cycle() {
        let span = TimeSpan::new(Fraction::new(1, 2), Fraction::new(5, 2));
        let cycles = span.span_cycles();

        assert_eq!(cycles.len(), 3);
        assert_eq!(
            cycles[0],
            TimeSpan::new(Fraction::new(1, 2), Fraction::from_integer(1))
        );
        assert_eq!(
            cycles[1],
            TimeSpan::new(Fraction::from_integer(1), Fraction::from_integer(2))
        );
        assert_eq!(
            cycles[2],
            TimeSpan::new(Fraction::from_integer(2), Fraction::new(5, 2))
        );
    }

    #[test]
    fn test_span_cycles_point() {
        let span = TimeSpan::new(Fraction::new(1, 2), Fraction::new(1, 2));
        let cycles = span.span_cycles();
        assert_eq!(cycles.len(), 1);
        assert_eq!(cycles[0], span);
    }

    #[test]
    fn test_intersection() {
        let a = TimeSpan::new(Fraction::from_integer(0), Fraction::from_integer(2));
        let b = TimeSpan::new(Fraction::from_integer(1), Fraction::from_integer(3));

        let intersection = a.intersection(&b);
        assert!(intersection.is_some());
        assert_eq!(
            intersection.unwrap(),
            TimeSpan::new(Fraction::from_integer(1), Fraction::from_integer(2))
        );
    }

    #[test]
    fn test_intersection_disjoint() {
        let a = TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 2));
        let b = TimeSpan::new(Fraction::new(3, 4), Fraction::new(1, 1));

        assert!(a.intersection(&b).is_none());
    }

    #[test]
    fn test_intersection_touching() {
        // [0, 0.5) and [0.5, 1) should NOT intersect (half-open)
        let a = TimeSpan::new(Fraction::new(0, 1), Fraction::new(1, 2));
        let b = TimeSpan::new(Fraction::new(1, 2), Fraction::from_integer(1));

        assert!(a.intersection(&b).is_none());
    }

    #[test]
    fn test_duration() {
        let span = TimeSpan::new(Fraction::new(1, 4), Fraction::new(3, 4));
        assert_eq!(span.duration(), Fraction::new(1, 2));
    }

    #[test]
    fn test_cycle_arc() {
        let span = TimeSpan::new(Fraction::new(5, 4), Fraction::new(7, 4));
        let arc = span.cycle_arc();

        assert_eq!(arc.begin, Fraction::new(1, 4));
        assert_eq!(arc.duration(), Fraction::new(1, 2));
    }
}
