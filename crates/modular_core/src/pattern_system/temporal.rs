//! Temporal transformations for patterns.
//!
//! These operations modify the timing of patterns:
//! - `fast(n)` / `slow(n)` - Speed up or slow down
//! - `early(n)` / `late(n)` - Shift in time
//! - `rev()` - Reverse within each cycle

use super::{Fraction, Hap, Pattern, State, TimeSpan};

impl<T: Clone + Send + Sync + 'static> Pattern<T> {
    /// Shift the pattern earlier in time.
    ///
    /// Events that would occur at time `t` now occur at time `t - offset`.
    /// Accepts both constant values and patterns.
    ///
    /// # Example
    /// ```ignore
    /// let pat = pure(42).early(Fraction::new(1, 4));
    /// // Events shifted 1/4 cycle earlier
    /// ```
    pub fn early<F: super::IntoPattern<Fraction> + 'static>(&self, offset: F) -> Pattern<T> {
        let offset_pat = offset.into_pattern();
        let pat = self.clone();

        offset_pat.inner_join(move |o| {
            pat._early(o.clone())
        })
    }

    /// Internal constant-offset early (no pattern overhead).
    pub(crate) fn _early(&self, offset: Fraction) -> Pattern<T> {
        let query = self.query.clone();
        let steps = self.steps.clone();
        let offset_clone = offset.clone();

        let mut result = Pattern::new(move |state: &State| {
            // Query at later time
            let new_span = state.span.with_time(|t| t + &offset_clone);
            let haps = query(&state.set_span(new_span));

            // Shift results earlier
            haps.into_iter()
                .map(|hap| hap.with_span_transform(|span| span.with_time(|t| t - &offset_clone)))
                .collect()
        });

        if let Some(s) = steps {
            result.set_steps(s);
        }
        result
    }

    /// Shift the pattern later in time.
    ///
    /// Events that would occur at time `t` now occur at time `t + offset`.
    /// Accepts both constant values and patterns.
    ///
    /// # Example
    /// ```ignore
    /// let pat = pure(42).late(Fraction::new(1, 4));
    /// // Events shifted 1/4 cycle later
    /// ```
    pub fn late<F: super::IntoPattern<Fraction> + 'static>(&self, offset: F) -> Pattern<T> {
        let offset_pat = offset.into_pattern();
        let pat = self.clone();

        offset_pat.inner_join(move |o| {
            pat._late(o.clone())
        })
    }

    /// Internal constant-offset late (no pattern overhead).
    pub(crate) fn _late(&self, offset: Fraction) -> Pattern<T> {
        self._early(-offset.clone())
    }

    /// Reverse the pattern within each cycle.
    ///
    /// Events are mirrored around the cycle center, so an event at position
    /// 0.25 moves to position 0.75, and vice versa.
    ///
    /// # Example
    /// ```ignore
    /// let pat = fastcat(vec![pure(0), pure(1), pure(2), pure(3)]).rev();
    /// // Order becomes: 3, 2, 1, 0 within each cycle
    /// ```
    pub fn rev(&self) -> Pattern<T> {
        let query = self.query.clone();
        let steps = self.steps.clone();

        let result = Pattern::new(move |state: &State| {
            // Process each cycle separately
            state
                .span
                .span_cycles()
                .into_iter()
                .flat_map(|cycle_span| {
                    let cycle = cycle_span.begin.sam();
                    let next_cycle = cycle_span.begin.next_sam();

                    // Reflect a time around the cycle center
                    let reflect = |t: &Fraction| -> Fraction {
                        &cycle + &(&next_cycle - t)
                    };

                    // Reflect the query span (swap begin and end after reflection)
                    let reflected_begin = reflect(&cycle_span.end);
                    let reflected_end = reflect(&cycle_span.begin);
                    let reflected_span = TimeSpan::new(reflected_begin, reflected_end);

                    // Query with reflected span
                    let haps = query(&state.set_span(reflected_span));

                    // Reflect the results back
                    haps.into_iter()
                        .map(|hap| {
                            let new_part = TimeSpan::new(
                                reflect(&hap.part.end),
                                reflect(&hap.part.begin),
                            );
                            let new_whole = hap.whole.map(|w| {
                                TimeSpan::new(reflect(&w.end), reflect(&w.begin))
                            });
                            Hap::with_context(new_whole, new_part, hap.value.clone(), hap.context.clone())
                        })
                        .collect::<Vec<_>>()
                })
                .collect()
        });

        let mut result = result;
        if let Some(s) = steps {
            result.set_steps(s);
        }
        result
    }

    /// Focus the pattern to play within a specific span.
    ///
    /// The first cycle of the pattern is stretched/compressed to fit
    /// within the given span. Used for squeeze operations.
    pub fn focus_span(&self, span: &TimeSpan) -> Pattern<T> {
        let duration = span.duration();
        if duration.is_zero() {
            return super::constructors::silence();
        }

        let span_begin = span.begin.clone();
        let span_sam = span_begin.sam();

        // Use internal methods for efficiency (constant factors, no pattern overhead)
        self._early(span_sam.clone())
            ._fast(Fraction::from_integer(1) / duration)
            ._late(span_begin)
    }

    /// Repeat the pattern n times per cycle.
    ///
    /// Equivalent to `fast(n)` but may have different step semantics.
    pub fn repeat(&self, n: u32) -> Pattern<T> {
        if n == 0 {
            return super::constructors::silence();
        }
        self._fast(Fraction::from_integer(n as i64))
    }

    /// Play the pattern only during the first part of each cycle.
    ///
    /// # Arguments
    /// * `ratio` - Fraction of the cycle during which the pattern plays (0 to 1)
    pub fn sustain(&self, ratio: Fraction) -> Pattern<T> {
        if ratio.is_zero() {
            return super::constructors::silence();
        }
        if ratio >= Fraction::from_integer(1) {
            return self.clone();
        }

        self.compress(&Fraction::from_integer(0), &ratio)
    }

    /// Rotate the pattern within each cycle.
    ///
    /// Positive values rotate right (later events appear first),
    /// negative values rotate left (earlier events appear first).
    pub fn rotate(&self, amount: Fraction) -> Pattern<T> {
        // Rotating by `amount` is the same as shifting early by `amount`
        // but wrapping around the cycle
        self.early(amount)
    }

    /// Play the pattern at double speed.
    pub fn double(&self) -> Pattern<T> {
        self.fast(Fraction::from_integer(2))
    }

    /// Play the pattern at half speed.
    pub fn half(&self) -> Pattern<T> {
        self.slow(Fraction::from_integer(2))
    }

    /// Discretize a continuous signal by sampling it n times per cycle.
    ///
    /// Essential for converting continuous signals (like `saw()` or `sine()`)
    /// to discrete events that can be used with other pattern operations.
    ///
    /// # Arguments
    /// * `n` - Number of samples per cycle (accepts both values and patterns)
    ///
    /// # Example
    /// ```ignore
    /// // Sample a sawtooth 8 times per cycle
    /// saw().segment(8)
    /// ```
    pub fn segment<N: super::IntoPattern<Fraction> + 'static>(&self, n: N) -> Pattern<T> {
        let n_pat = n.into_pattern();
        let pat = self.clone();

        n_pat.inner_join(move |n_frac| {
            pat._segment(n_frac.clone())
        })
    }

    /// Internal constant-n segment (no pattern overhead).
    fn _segment(&self, n_frac: Fraction) -> Pattern<T> {
        let n = n_frac.to_f64() as i64;
        if n <= 0 {
            return super::constructors::silence();
        }

        let pat = self.clone();
        let frac_n = Fraction::from_integer(n);

        Pattern::new(move |state: &State| {
            let mut result = Vec::new();

            for span in state.span.span_cycles() {
                let cycle_start = span.begin.sam();

                for i in 0..n {
                    let frac_i = Fraction::from_integer(i);
                    let event_start = &cycle_start + &frac_i / &frac_n;
                    let event_end = &cycle_start + (&frac_i + Fraction::from_integer(1)) / &frac_n;
                    let event_span = TimeSpan::new(event_start.clone(), event_end.clone());

                    // Check if this event intersects the query span
                    if let Some(part) = event_span.intersection(&span) {
                        // Sample the pattern at the event start
                        let sample_state = state.set_span(TimeSpan::new(
                            event_start.clone(),
                            event_start.clone() + Fraction::new(1, 10000), // tiny window
                        ));

                        if let Some(hap) = pat.query(&sample_state).into_iter().next() {
                            result.push(Hap::new(
                                Some(event_span),
                                part,
                                hap.value.clone(),
                            ));
                        }
                    }
                }
            }

            result
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pattern_system::{constructors::pure, combinators::fastcat};

    #[test]
    fn test_early() {
        let pat = pure(42).early(Fraction::new(1, 4));
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        // Early shifts the query later, which may cross cycle boundaries
        // We get events from both cycles that the shifted query spans
        assert!(haps.len() >= 1);

        // Find the event from cycle 0 (shifted from [-1/4, 3/4))
        let cycle0_hap = haps.iter().find(|h| {
            h.whole
                .as_ref()
                .map(|w| w.begin == Fraction::new(-1, 4))
                .unwrap_or(false)
        });
        assert!(cycle0_hap.is_some());
    }

    #[test]
    fn test_late() {
        let pat = pure(42).late(Fraction::new(1, 4));
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        // Late shifts the query earlier, which may cross cycle boundaries
        assert!(haps.len() >= 1);

        // Find the event from cycle 0 (shifted to [1/4, 5/4))
        let cycle0_hap = haps.iter().find(|h| {
            h.whole
                .as_ref()
                .map(|w| w.begin == Fraction::new(1, 4))
                .unwrap_or(false)
        });
        assert!(cycle0_hap.is_some());
    }

    #[test]
    fn test_rev() {
        let pat = fastcat(vec![pure(0), pure(1), pure(2), pure(3)]).rev();
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        assert_eq!(haps.len(), 4);

        // Values should be reversed: 3, 2, 1, 0
        // Sort by begin time to check order
        let mut sorted_haps = haps.clone();
        sorted_haps.sort_by(|a, b| a.part.begin.cmp(&b.part.begin));

        assert_eq!(sorted_haps[0].value, 3);
        assert_eq!(sorted_haps[1].value, 2);
        assert_eq!(sorted_haps[2].value, 1);
        assert_eq!(sorted_haps[3].value, 0);
    }

    #[test]
    fn test_repeat() {
        let pat = pure(42).repeat(3);
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        assert_eq!(haps.len(), 3);
    }

    #[test]
    fn test_double() {
        let pat = pure(42).double();
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        assert_eq!(haps.len(), 2);
    }

    #[test]
    fn test_half() {
        let pat = pure(42).half();
        let haps = pat.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        // Should get one event that spans 2 cycles
        assert_eq!(haps.len(), 1);
        let whole = haps[0].whole.as_ref().unwrap();
        assert_eq!(whole.duration(), Fraction::from_integer(2));
    }
}
