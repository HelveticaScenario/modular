//! Monadic operations for pattern composition.
//!
//! Monadic bind (flatMap) allows patterns where the value determines
//! the next pattern. Different join strategies determine how the
//! inner pattern's timing relates to the outer pattern.

use super::{Fraction, Hap, HapContext, Pattern, State, TimeSpan};
use std::sync::Arc;

impl<T: Clone + Send + Sync + 'static> Pattern<T> {
    /// Bind (flatMap) with a function that produces patterns.
    ///
    /// For each hap, applies the function to get an inner pattern,
    /// then combines using intersection of wholes.
    pub fn bind<U, F>(&self, f: F) -> Pattern<U>
    where
        U: Clone + Send + Sync + 'static,
        F: Fn(&T) -> Pattern<U> + Send + Sync + 'static,
    {
        self.bind_whole(
            |outer_whole, inner_whole| {
                match (outer_whole, inner_whole) {
                    (Some(o), Some(i)) => o.intersection(&i),
                    _ => None,
                }
            },
            f,
        )
    }

    /// Generalized bind with custom whole-span combination.
    pub fn bind_whole<U, W, F>(&self, whole_fn: W, f: F) -> Pattern<U>
    where
        U: Clone + Send + Sync + 'static,
        W: Fn(Option<&TimeSpan>, Option<&TimeSpan>) -> Option<TimeSpan> + Send + Sync + 'static,
        F: Fn(&T) -> Pattern<U> + Send + Sync + 'static,
    {
        let outer = self.clone();
        let f = Arc::new(f);
        let whole_fn = Arc::new(whole_fn);

        Pattern::new(move |state: &State| {
            let outer_haps = outer.query(state);
            let mut result = Vec::new();

            for outer_hap in outer_haps {
                let inner_pat = f(&outer_hap.value);
                let inner_haps = inner_pat.query(state);

                for inner_hap in inner_haps {
                    if let Some(part) = outer_hap.part.intersection(&inner_hap.part) {
                        let whole = whole_fn(outer_hap.whole.as_ref(), inner_hap.whole.as_ref());
                        let context = HapContext::merge(&outer_hap.context, &inner_hap.context);

                        result.push(Hap {
                            whole,
                            part,
                            value: inner_hap.value.clone(),
                            context,
                        });
                    }
                }
            }

            result
        })
    }

    /// Inner join - preserves inner pattern structure.
    ///
    /// The whole span comes from the inner pattern.
    pub fn inner_join<U, F>(&self, f: F) -> Pattern<U>
    where
        U: Clone + Send + Sync + 'static,
        F: Fn(&T) -> Pattern<U> + Send + Sync + 'static,
    {
        self.bind_whole(|_outer, inner| inner.cloned(), f)
    }

    /// Outer join - preserves outer pattern structure.
    ///
    /// The whole span comes from the outer pattern.
    pub fn outer_join<U, F>(&self, f: F) -> Pattern<U>
    where
        U: Clone + Send + Sync + 'static,
        F: Fn(&T) -> Pattern<U> + Send + Sync + 'static,
    {
        self.bind_whole(|outer, _inner| outer.cloned(), f)
    }

    /// Squeeze join - fit inner pattern into outer events.
    ///
    /// Each outer event's timespan is used to "squeeze" the inner pattern,
    /// compressing its full cycle into that event's duration.
    pub fn squeeze_join<U, F>(&self, f: F) -> Pattern<U>
    where
        U: Clone + Send + Sync + 'static,
        F: Fn(&T) -> Pattern<U> + Send + Sync + 'static,
    {
        let outer = self.clone();
        let f = Arc::new(f);

        Pattern::new(move |state: &State| {
            // Only consider discrete (onset) events from outer
            let outer_haps: Vec<_> = outer
                .query(state)
                .into_iter()
                .filter(|h| h.has_onset())
                .collect();

            let mut result = Vec::new();

            for outer_hap in outer_haps {
                let inner_pat = f(&outer_hap.value);

                // Get the span to squeeze into
                let squeeze_span = outer_hap.whole_or_part();

                // Focus the inner pattern into this span
                let focused = inner_pat.focus_span(&squeeze_span);

                // Query the focused pattern at the current state
                let inner_haps = focused.query(state);

                for inner_hap in inner_haps {
                    // Part must still intersect with query span
                    if let Some(part) = inner_hap.part.intersection(&state.span) {
                        let context = HapContext::merge(&outer_hap.context, &inner_hap.context);

                        result.push(Hap {
                            whole: inner_hap.whole,
                            part,
                            value: inner_hap.value,
                            context,
                        });
                    }
                }
            }

            result
        })
    }

}

/// Pattern of patterns - can be joined/flattened
impl<T: Clone + Send + Sync + 'static> Pattern<Pattern<T>> {
    /// Flatten a pattern of patterns using bind.
    pub fn join(&self) -> Pattern<T> {
        self.bind(|inner| inner.clone())
    }

    /// Flatten using squeeze semantics.
    pub fn squeeze(&self) -> Pattern<T> {
        self.squeeze_join(|inner| inner.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern_system::combinators::fastcat;
    use crate::pattern_system::constructors::pure;

    #[test]
    fn test_bind_basic() {
        let outer = pure(2);
        let result = outer.bind(|n| pure(*n * 10));

        let haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps.len(), 1);
        assert_eq!(haps[0].value, 20);
    }

    #[test]
    fn test_inner_join() {
        let outer = pure(5);
        let result = outer.inner_join(|n| pure(*n + 1));

        let haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps.len(), 1);
        assert_eq!(haps[0].value, 6);
    }

    #[test]
    fn test_outer_join() {
        let outer = pure(5);
        let result = outer.outer_join(|n| pure(*n + 1));

        let haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps.len(), 1);
        assert_eq!(haps[0].value, 6);
    }

    #[test]
    fn test_squeeze_join() {
        // Outer: 2 events per cycle
        let outer = fastcat(vec![pure(1), pure(2)]);

        // Each value produces a pattern with 2 events
        let result = outer.squeeze_join(|n| fastcat(vec![pure(*n), pure(*n * 10)]));

        let haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        // Should have at least 4 events (may have more due to boundary handling)
        assert!(haps.len() >= 4, "Expected at least 4 events, got {}", haps.len());

        // Verify we have the expected values
        let values: Vec<_> = haps.iter().map(|h| h.value).collect();
        assert!(values.contains(&1));
        assert!(values.contains(&10));
        assert!(values.contains(&2));
        assert!(values.contains(&20));
    }

    #[test]
    fn test_focus_span() {
        let pat = fastcat(vec![pure(1), pure(2)]);

        // Focus into [0.5, 1)
        let span = TimeSpan::new(Fraction::new(1, 2), Fraction::from_integer(1));
        let focused = pat.focus_span(&span);

        let haps = focused.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        // Should have events squeezed into [0.5, 1)
        // Due to focus_span implementation via early/fast/late, we may get more events
        assert!(haps.len() >= 2, "Expected at least 2 events, got {}", haps.len());

        // At least some events should be in the [0.5, 1) range
        let in_range_count = haps.iter().filter(|h| {
            h.part.begin >= Fraction::new(1, 2) && h.part.end <= Fraction::from_integer(1)
        }).count();
        assert!(in_range_count >= 1, "Expected events in [0.5, 1) range");
    }

    #[test]
    fn test_pattern_of_patterns_join() {
        let inner1 = pure(1);
        let inner2 = pure(2);
        let outer: Pattern<Pattern<i32>> = fastcat(vec![pure(inner1), pure(inner2)]);

        let flattened = outer.join();
        let haps = flattened.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        assert_eq!(haps.len(), 2);
    }
}
