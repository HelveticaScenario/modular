//! Strudel-compatible pattern operations.
//!
//! This module provides pattern operations that match Strudel's API
//! for feature parity with the JavaScript implementation.

use super::{Fraction, Hap, HapContext, Pattern, State, TimeSpan};
use super::combinators::{fastcat, slowcat, stack, timecat};
use super::constructors::{pure, silence};
use std::collections::HashMap;
use std::sync::Arc;

// ============================================================================
// TIER 1: Critical Operations
// ============================================================================

impl<T: Clone + Send + Sync + 'static> Pattern<T> {
    /// Apply boolean pattern structure to this pattern.
    ///
    /// Events from `self` are restructured to match the timing of `true` events
    /// in the boolean pattern. This is fundamental for Tidal-style patterning.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // sequence('a', 'b').struct(sequence(true, true, true))
    /// let pat = fastcat(vec![pure("a"), pure("b")]);
    /// let bool_pat = fastcat(vec![pure(true), pure(true), pure(true)]);
    /// let result = pat.struct_pat(&bool_pat);
    /// ```
    pub fn struct_pat(&self, bool_pat: &Pattern<bool>) -> Pattern<T> {
        let value_pat = self.clone();
        let bool_pat = bool_pat.clone();

        Pattern::new(move |state: &State| {
            let bool_haps: Vec<_> = bool_pat
                .query(state)
                .into_iter()
                .filter(|h| h.value && h.has_onset())
                .collect();

            let mut result = Vec::new();

            for bool_hap in bool_haps {
                // Query value pattern at the boolean hap's timespan
                let query_span = bool_hap.whole_or_part().clone();
                let value_haps = value_pat.query(&state.set_span(query_span));

                for value_hap in value_haps {
                    // Intersect parts
                    if let Some(part) = bool_hap.part.intersection(&value_hap.part) {
                        let context = HapContext::merge(&bool_hap.context, &value_hap.context);
                        result.push(Hap::with_context(
                            bool_hap.whole.clone(),
                            part,
                            value_hap.value.clone(),
                            context,
                        ));
                    }
                }
            }

            result
        })
    }

    /// Mask this pattern with a boolean pattern.
    ///
    /// Unlike `struct_pat`, this keeps the original pattern's structure
    /// but filters out events where the boolean pattern is false.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // sequence('a', 'b').mask(sequence(true, false))
    /// ```
    pub fn mask(&self, bool_pat: &Pattern<bool>) -> Pattern<T> {
        let value_pat = self.clone();
        let bool_pat = bool_pat.clone();

        Pattern::new(move |state: &State| {
            let value_haps = value_pat.query(state);
            let bool_haps = bool_pat.query(state);

            let mut result = Vec::new();

            for value_hap in value_haps {
                for bool_hap in &bool_haps {
                    if !bool_hap.value {
                        continue;
                    }

                    if let Some(part) = value_hap.part.intersection(&bool_hap.part) {
                        let context = HapContext::merge(&value_hap.context, &bool_hap.context);
                        result.push(Hap::with_context(
                            value_hap.whole.clone(),
                            part,
                            value_hap.value.clone(),
                            context,
                        ));
                    }
                }
            }

            result
        })
    }

    /// Apply a function conditionally based on a boolean pattern.
    ///
    /// When the boolean pattern is true, apply the function; otherwise keep original.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // pure('a').when(pure(true), (x) => x._fast(2))
    /// ```
    pub fn when<F>(&self, bool_pat: &Pattern<bool>, f: F) -> Pattern<T>
    where
        F: Fn(&Pattern<T>) -> Pattern<T> + Send + Sync + Clone + 'static,
    {
        let pat = self.clone();
        let bool_pat = bool_pat.clone();
        let transformed = f(&pat);

        Pattern::new(move |state: &State| {
            let bool_haps = bool_pat.query(state);
            let mut result = Vec::new();

            for bool_hap in bool_haps {
                let query_state = state.set_span(bool_hap.part.clone());
                let source_pat = if bool_hap.value {
                    &transformed
                } else {
                    &pat
                };

                for hap in source_pat.query(&query_state) {
                    if let Some(part) = hap.part.intersection(&bool_hap.part) {
                        result.push(Hap::with_context(
                            hap.whole.clone(),
                            part,
                            hap.value.clone(),
                            hap.context.clone(),
                        ));
                    }
                }
            }

            result
        })
    }

    /// Apply a function every nth cycle.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // pure('a').firstOf(3, (x) => x._fast(2))._fast(3)
    /// ```
    pub fn first_of<F>(&self, n: i64, f: F) -> Pattern<T>
    where
        F: Fn(&Pattern<T>) -> Pattern<T> + Send + Sync + Clone + 'static,
    {
        let pat = self.clone();
        let transformed = f(&pat);

        Pattern::new(move |state: &State| {
            state
                .span
                .span_cycles()
                .into_iter()
                .flat_map(|cycle_span| {
                    let cycle_num = cycle_span.begin.sam().to_f64() as i64;
                    let use_transformed = cycle_num % n == 0;

                    let source_pat = if use_transformed {
                        &transformed
                    } else {
                        &pat
                    };

                    source_pat.query(&state.set_span(cycle_span))
                })
                .collect()
        })
    }

    /// Alias for first_of (Tidal naming).
    pub fn every<F>(&self, n: i64, f: F) -> Pattern<T>
    where
        F: Fn(&Pattern<T>) -> Pattern<T> + Send + Sync + Clone + 'static,
    {
        self.first_of(n, f)
    }
}

impl Pattern<bool> {
    /// Invert a boolean pattern.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // sequence(true, false, [true, false]).invert()
    /// ```
    pub fn invert(&self) -> Pattern<bool> {
        self.fmap(|b| !b)
    }
}

// ============================================================================
// Numeric Pattern Operations
// ============================================================================

impl Pattern<f64> {
    /// Map values from 0-1 range to min-max range.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // sequence(0, 0).range(sequence(0, 0.5), 1)
    /// ```
    pub fn range(&self, min: f64, max: f64) -> Pattern<f64> {
        self.fmap(move |v| min + v * (max - min))
    }

    /// Map values from -1 to 1 range (bipolar) to min-max range.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // sequence(-1, -0.5, 0, 0.5).range2(1000, 1100)
    /// ```
    pub fn range2(&self, min: f64, max: f64) -> Pattern<f64> {
        self.fmap(move |v| {
            let normalized = (v + 1.0) / 2.0; // -1..1 -> 0..1
            min + normalized * (max - min)
        })
    }
}

/// Generate a sequence from 0 to n-1.
///
/// # Example (Strudel equivalent)
/// ```ignore
/// // run(4) -> sequence(0, 1, 2, 3)
/// ```
pub fn run(n: i64) -> Pattern<i64> {
    if n <= 0 {
        return silence();
    }
    fastcat((0..n).map(pure).collect())
}

/// Generate a sequence from 0.0 to n-1 as f64.
pub fn run_f64(n: i64) -> Pattern<f64> {
    if n <= 0 {
        return silence();
    }
    fastcat((0..n).map(|i| pure(i as f64)).collect())
}

/// Generate a binary pattern from a decimal number.
///
/// # Example (Strudel equivalent)
/// ```ignore
/// // binaryN(55532) -> sequence(1, 1, 0, 1, 1, 0, 0, 0, 1, 1, 1, 0, 1, 1, 0, 0)
/// ```
pub fn binary_n(num: u64, bits: Option<usize>) -> Pattern<i64> {
    let bits = bits.unwrap_or(16);
    let mut pattern_bits = Vec::with_capacity(bits);

    for i in (0..bits).rev() {
        let bit = if (num >> i) & 1 == 1 { 1i64 } else { 0i64 };
        pattern_bits.push(pure(bit));
    }

    fastcat(pattern_bits)
}

// ============================================================================
// TIER 2: Important Operations
// ============================================================================

impl<T: Clone + Send + Sync + 'static> Pattern<T> {
    /// Multiply each event by repeating it n times within its timespan.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // sequence('a', ['b', 'c']).ply(3)
    /// ```
    pub fn ply(&self, n: i64) -> Pattern<T> {
        if n <= 0 {
            return silence();
        }

        let pat = self.clone();
        let n_frac = Fraction::from_integer(n);
        let steps = self.steps().cloned();

        let mut result = Pattern::new(move |state: &State| {
            let haps = pat.query(state);
            let mut result = Vec::new();

            for hap in haps {
                if let Some(whole) = &hap.whole {
                    let duration = whole.duration();
                    let sub_duration = &duration / &n_frac;

                    for i in 0..n {
                        let i_frac = Fraction::from_integer(i);
                        let sub_begin = &whole.begin + &(&i_frac * &sub_duration);
                        let sub_end = &sub_begin + &sub_duration;
                        let sub_whole = TimeSpan::new(sub_begin.clone(), sub_end.clone());

                        if let Some(part) = sub_whole.intersection(&hap.part) {
                            if let Some(query_part) = part.intersection(&state.span) {
                                result.push(Hap::with_context(
                                    Some(sub_whole),
                                    query_part,
                                    hap.value.clone(),
                                    hap.context.clone(),
                                ));
                            }
                        }
                    }
                } else {
                    result.push(hap.clone());
                }
            }

            result
        });

        if let Some(s) = steps {
            result.set_steps(&s * &Fraction::from_integer(n));
        }
        result
    }

    /// Overlay a time-shifted and transformed copy of the pattern.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // pure(30).off(0.25, add(2))
    /// ```
    pub fn off<F>(&self, offset: Fraction, f: F) -> Pattern<T>
    where
        F: Fn(&Pattern<T>) -> Pattern<T> + Send + Sync + 'static,
    {
        let original = self.clone();
        let transformed = f(&self.clone())._late(offset);
        stack(vec![original, transformed])
    }

    /// Stack multiple transformations of this pattern.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // sequence(1, 2, 3).layer(fast(2), (pat) => pat.add(3, 4))
    /// ```
    pub fn layer<F>(&self, funcs: Vec<F>) -> Pattern<T>
    where
        F: Fn(&Pattern<T>) -> Pattern<T> + Send + Sync + 'static,
    {
        let layers: Vec<Pattern<T>> = funcs.iter().map(|f| f(self)).collect();
        stack(layers)
    }

    /// Create a palindrome by playing the pattern forward then backward.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // fastcat('a', 'b', 'c').palindrome()
    /// ```
    pub fn palindrome(&self) -> Pattern<T> {
        slowcat(vec![self.clone(), self.rev()])
    }

    /// Keep the left value but use the right pattern's structure.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // pure(3).keep(pure(4))
    /// ```
    pub fn keep<U: Clone + Send + Sync + 'static>(&self, other: &Pattern<U>) -> Pattern<T> {
        self.app_left(other, |a, _b| a.clone())
    }

    /// Filter events based on a boolean pattern.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // sequence(3, 4).keepif(true, false)
    /// ```
    pub fn keepif(&self, bool_pat: &Pattern<bool>) -> Pattern<T> {
        self.mask(bool_pat)
    }

    /// Breakbeat-style transformation.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // sequence('a', 'b').brak()
    /// ```
    pub fn brak(&self) -> Pattern<T> {
        let pat = self.clone();
        let half = Fraction::new(1, 2);

        // Create the "broken" version: silence in first half, pattern in second half
        let broken = silence::<T>().compress(&Fraction::from_integer(0), &half);
        let second_half = pat.clone().compress(&half, &Fraction::from_integer(1));

        // Alternate: normal, broken
        slowcat(vec![pat, stack(vec![broken, second_half])])
    }

    /// Syncopate events by shifting them by half their duration.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // sequence('a', 'b', 'c', 'd').press()
    /// ```
    pub fn press(&self) -> Pattern<T> {
        let pat = self.clone();
        let half = Fraction::new(1, 2);

        Pattern::new(move |state: &State| {
            let haps = pat.query(state);
            let mut result = Vec::new();

            for hap in haps {
                if let Some(whole) = &hap.whole {
                    let duration = whole.duration();
                    let offset = &duration * &half;

                    let new_whole = TimeSpan::new(
                        &whole.begin + &offset,
                        &whole.end + &offset,
                    );

                    if let Some(part) = new_whole.intersection(&state.span) {
                        result.push(Hap::with_context(
                            Some(new_whole),
                            part,
                            hap.value.clone(),
                            hap.context.clone(),
                        ));
                    }
                }
            }

            result
        })
    }

    /// Linger on the first part of the pattern.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // sequence(0, 1, 2, 3, 4, 5, 6, 7).linger(0.25)
    /// ```
    pub fn linger(&self, fraction: Fraction) -> Pattern<T> {
        if fraction <= Fraction::from_integer(0) {
            return silence();
        }

        self.compress(&Fraction::from_integer(0), &fraction)
            ._fast(Fraction::from_integer(1) / fraction)
    }

    /// Extract a ribbon (portion) of the pattern.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // cat(0, 1, 2, 3, 4, 5, 6, 7).ribbon(2, 4).fast(4)
    /// ```
    pub fn ribbon(&self, start_cycle: i64, end_cycle: i64) -> Pattern<T> {
        let pat = self.clone();
        let start = Fraction::from_integer(start_cycle);
        let end = Fraction::from_integer(end_cycle);
        let len = &end - &start;

        Pattern::new(move |state: &State| {
            // Query the source pattern at the offset cycles
            let query_span = state.span.with_time(|t| t + &start);
            pat.query(&state.set_span(query_span))
                .into_iter()
                .map(|hap| hap.with_span_transform(|span| span.with_time(|t| t - &start)))
                .collect()
        })
        ._slow(len)
    }

    /// Repeat each cycle n times.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // slowcat(0, 1).repeatCycles(2).fast(6)
    /// ```
    pub fn repeat_cycles(&self, n: i64) -> Pattern<T> {
        if n <= 0 {
            return silence();
        }

        let pat = self.clone();
        let n_frac = Fraction::from_integer(n);

        Pattern::new(move |state: &State| {
            state
                .span
                .span_cycles()
                .into_iter()
                .flat_map(|cycle_span| {
                    let cycle = cycle_span.begin.floor().to_f64() as i64;
                    let source_cycle = cycle / n;
                    let offset = Fraction::from_integer(cycle - source_cycle * n);

                    let query_span = cycle_span.with_time(|t| {
                        let cycle_pos = t.cycle_pos();
                        Fraction::from_integer(source_cycle) + cycle_pos
                    });

                    pat.query(&state.set_span(query_span))
                        .into_iter()
                        .map(|hap| {
                            hap.with_span_transform(|span| {
                                let duration = span.duration();
                                let base_cycle = span.begin.floor();
                                let pos = span.begin.cycle_pos();
                                let new_begin = Fraction::from_integer(cycle) + pos;
                                TimeSpan::new(new_begin.clone(), new_begin + duration)
                            })
                        })
                        .collect::<Vec<_>>()
                })
                .collect()
        })
    }

    /// Speed up the pattern with a gap of silence.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // sequence('a', 'b', 'c')._fastGap(2)
    /// ```
    pub fn fast_gap(&self, factor: Fraction) -> Pattern<T> {
        if factor <= Fraction::from_integer(1) {
            return self.clone();
        }

        let compressed = self.compress(
            &Fraction::from_integer(0),
            &(Fraction::from_integer(1) / &factor),
        );

        compressed
    }

    /// Apply transformation at a faster rate.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // sequence('a', 'b', 'c', 'd').inside(2, rev)
    /// ```
    pub fn inside<F>(&self, factor: Fraction, f: F) -> Pattern<T>
    where
        F: Fn(&Pattern<T>) -> Pattern<T> + Send + Sync + 'static,
    {
        f(&self._fast(factor.clone()))._slow(factor)
    }

    /// Apply transformation at a slower rate.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // sequence('a', 'b', 'c', 'd')._slow(2).outside(2, rev)
    /// ```
    pub fn outside<F>(&self, factor: Fraction, f: F) -> Pattern<T>
    where
        F: Fn(&Pattern<T>) -> Pattern<T> + Send + Sync + 'static,
    {
        f(&self._slow(factor.clone()))._fast(factor)
    }

    /// Reverse the order of cycles (not just within cycles).
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // fastcat('a', 'b', 'c', 'd').slow(2).revv().fast(2)
    /// ```
    pub fn revv(&self) -> Pattern<T> {
        let pat = self.clone();

        Pattern::new(move |state: &State| {
            // Reflect query around t=0
            let reflected_span = state.span.with_time(|t| -t);
            // Swap begin and end since they got reversed
            let query_span = TimeSpan::new(
                reflected_span.end.clone(),
                reflected_span.begin.clone(),
            );

            pat.query(&state.set_span(query_span))
                .into_iter()
                .map(|hap| {
                    hap.with_span_transform(|span| {
                        TimeSpan::new(-span.end.clone(), -span.begin.clone())
                    })
                })
                .collect()
        })
    }

    /// Process pattern in chunks, applying transformation to each chunk in turn.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // sequence(0, 1, 2, 3).slow(2).chunk(2, add(10)).fast(4)
    /// ```
    pub fn chunk<F>(&self, n: i64, f: F) -> Pattern<T>
    where
        F: Fn(&Pattern<T>) -> Pattern<T> + Send + Sync + Clone + 'static,
    {
        let pat = self.clone();
        let f = Arc::new(f);

        Pattern::new(move |state: &State| {
            let f = f.clone();
            state
                .span
                .span_cycles()
                .into_iter()
                .flat_map(|cycle_span| {
                    let cycle = cycle_span.begin.floor().to_f64() as i64;
                    let chunk_idx = cycle % n;
                    let f = f.clone();

                    // Apply inside transformation for this chunk
                    let early_offset = Fraction::from_integer(chunk_idx);
                    let late_offset = Fraction::from_integer(chunk_idx);
                    let transformed = pat.inside(Fraction::from_integer(n), move |p| {
                        let p = p.clone();
                        let f = f.clone();
                        let early_offset = early_offset.clone();
                        let late_offset = late_offset.clone();
                        p.first_of(n, move |inner| {
                            // Rotate so the current chunk is first
                            let f = f.clone();
                            inner._early(early_offset.clone()).first_of(n, move |x| f(x))._late(late_offset.clone())
                        })
                    });

                    transformed.query(&state.set_span(cycle_span))
                })
                .collect()
        })
    }

    /// Fast chunk - unlike chunk, cycles proceed cycle-by-cycle.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // sequence(0, 1, 2, 3).slow(2).fastChunk(2, add(10)).fast(4)
    /// ```
    pub fn fast_chunk<F>(&self, n: i64, f: F) -> Pattern<T>
    where
        F: Fn(&Pattern<T>) -> Pattern<T> + Send + Sync + Clone + 'static,
    {
        let f = Arc::new(f);
        self.inside(Fraction::from_integer(n), move |p| {
            let f = f.clone();
            p.first_of(n, move |x| f(x))
        })
    }
}

// ============================================================================
// Polyrhythm and Polymeter
// ============================================================================

/// Layer multiple patterns at different speeds (polyrhythm).
///
/// # Example (Strudel equivalent)
/// ```ignore
/// // polyrhythm(['a', 'b'], ['c'])
/// ```
pub fn polyrhythm<T: Clone + Send + Sync + 'static>(patterns: Vec<Vec<Pattern<T>>>) -> Pattern<T> {
    let layer_patterns: Vec<Pattern<T>> = patterns
        .into_iter()
        .map(fastcat)
        .collect();
    stack(layer_patterns)
}

/// Layer multiple patterns stepwise (polymeter).
///
/// # Example (Strudel equivalent)
/// ```ignore
/// // polymeter(['a', 'b', 'c'], ['d', 'e']).fast(2)
/// ```
pub fn polymeter<T: Clone + Send + Sync + 'static>(patterns: Vec<Vec<Pattern<T>>>) -> Pattern<T> {
    if patterns.is_empty() {
        return silence();
    }

    // Find LCM of pattern lengths for proper alignment
    let lengths: Vec<usize> = patterns.iter().map(|p| p.len()).collect();
    let target_len = lengths.iter().max().copied().unwrap_or(1);

    let layer_patterns: Vec<Pattern<T>> = patterns
        .into_iter()
        .map(|pats| {
            if pats.is_empty() {
                silence()
            } else {
                let len = pats.len();
                slowcat(pats)._fast(Fraction::new(target_len as i64, len as i64))
            }
        })
        .collect();

    stack(layer_patterns)
}

// ============================================================================
// TIER 3: Nice to Have Operations
// ============================================================================

impl<T: Clone + Send + Sync + 'static> Pattern<T> {
    /// Take the first n steps from the pattern.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // sequence(0, 1, 2, 3, 4).take(2)
    /// ```
    pub fn take(&self, n: i64) -> Pattern<T> {
        if n == 0 {
            return silence();
        }

        let steps = self.steps().cloned().unwrap_or(Fraction::from_integer(1));
        let n_frac = Fraction::from_integer(n.abs());

        if n > 0 {
            // Take from left
            self.compress(&Fraction::from_integer(0), &(&n_frac / &steps))
        } else {
            // Take from right
            let start = &(Fraction::from_integer(1) - &n_frac / &steps);
            self.compress(start, &Fraction::from_integer(1))
        }
    }

    /// Drop the first n steps from the pattern.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // sequence(0, 1, 2, 3, 4).drop(2)
    /// ```
    pub fn drop_steps(&self, n: i64) -> Pattern<T> {
        if n == 0 {
            return self.clone();
        }

        let steps = self.steps().cloned().unwrap_or(Fraction::from_integer(1));
        let n_frac = Fraction::from_integer(n.abs());

        if n > 0 {
            // Drop from left
            let start = &n_frac / &steps;
            self.compress(&start, &Fraction::from_integer(1))
        } else {
            // Drop from right
            let end = Fraction::from_integer(1) - &n_frac / &steps;
            self.compress(&Fraction::from_integer(0), &end)
        }
    }

    /// Progressively shrink the pattern.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // sequence(0, 1, 2, 3, 4).shrink(1)
    /// ```
    pub fn shrink(&self, direction: i64) -> Pattern<T> {
        let steps = self.steps().cloned().unwrap_or(Fraction::from_integer(1));
        let n = steps.to_f64() as i64;

        if n <= 0 {
            return silence();
        }

        let mut patterns = Vec::new();
        for i in 0..n {
            let drop_count = if direction >= 0 { i } else { n - 1 - i };
            if direction >= 0 {
                patterns.push(self.drop_steps(drop_count));
            } else {
                patterns.push(self.drop_steps(-drop_count));
            }
        }

        fastcat(patterns)
    }

    /// Progressively grow the pattern.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // sequence(0, 1, 2, 3, 4).grow(1)
    /// ```
    pub fn grow(&self, direction: i64) -> Pattern<T> {
        let steps = self.steps().cloned().unwrap_or(Fraction::from_integer(1));
        let n = steps.to_f64() as i64;

        if n <= 0 {
            return silence();
        }

        let mut patterns = Vec::new();
        for i in 0..n {
            let take_count = i + 1;
            if direction >= 0 {
                patterns.push(self.take(take_count));
            } else {
                patterns.push(self.take(-take_count));
            }
        }

        fastcat(patterns)
    }

    /// Defragment touching haps with the same value.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // stack(pure('a').mask(1, 0), pure('a').mask(0, 1)).defragmentHaps()
    /// ```
    pub fn defragment_haps(&self) -> Pattern<T>
    where
        T: PartialEq,
    {
        let pat = self.clone();

        Pattern::new(move |state: &State| {
            let mut haps = pat.query(state);

            if haps.len() <= 1 {
                return haps;
            }

            // Sort by part begin
            haps.sort_by(|a, b| a.part.begin.cmp(&b.part.begin));

            let mut result = Vec::new();
            let mut current = haps.remove(0);

            for hap in haps {
                // Check if can merge: same whole, same value, touching parts
                let can_merge = current.whole == hap.whole
                    && current.value == hap.value
                    && current.part.end == hap.part.begin;

                if can_merge {
                    // Extend current
                    current = Hap::with_context(
                        current.whole,
                        TimeSpan::new(current.part.begin, hap.part.end),
                        current.value,
                        current.context,
                    );
                } else {
                    result.push(current);
                    current = hap;
                }
            }

            result.push(current);
            result
        })
    }
}

// ============================================================================
// Sample Operations (Tier 3)
// ============================================================================

/// Sample-related value type for slice/splice operations.
#[derive(Clone, Debug, PartialEq)]
pub struct SampleValue {
    pub sound: String,
    pub begin: f64,
    pub end: f64,
    pub speed: Option<f64>,
    pub unit: Option<String>,
    pub slices: Option<i64>,
}

impl Default for SampleValue {
    fn default() -> Self {
        Self {
            sound: String::new(),
            begin: 0.0,
            end: 1.0,
            speed: None,
            unit: None,
            slices: None,
        }
    }
}

impl Pattern<SampleValue> {
    /// Slice a sample into n parts.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // s('break').slice(4, sequence(0, 1, 2, 3))
    /// ```
    pub fn slice(&self, n: i64, index_pat: &Pattern<i64>) -> Pattern<SampleValue> {
        let sample_pat = self.clone();
        let index_pat = index_pat.clone();
        let n_f = n as f64;

        sample_pat.app_left(&index_pat, move |sample, idx| {
            let idx = *idx as f64;
            let slice_size = 1.0 / n_f;
            SampleValue {
                sound: sample.sound.clone(),
                begin: (idx * slice_size).max(0.0).min(1.0),
                end: ((idx + 1.0) * slice_size).max(0.0).min(1.0),
                speed: sample.speed,
                unit: sample.unit.clone(),
                slices: Some(n),
            }
        })
    }

    /// Splice a sample (slice with speed adjustment).
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // s('break').splice(4, sequence(0, 1, 2, 3))
    /// ```
    pub fn splice(&self, n: i64, index_pat: &Pattern<i64>) -> Pattern<SampleValue> {
        let sample_pat = self.clone();
        let index_pat = index_pat.clone();
        let n_f = n as f64;

        sample_pat.app_left(&index_pat, move |sample, idx| {
            let idx = *idx as f64;
            let slice_size = 1.0 / n_f;
            SampleValue {
                sound: sample.sound.clone(),
                begin: (idx * slice_size).max(0.0).min(1.0),
                end: ((idx + 1.0) * slice_size).max(0.0).min(1.0),
                speed: Some(1.0),
                unit: Some("c".to_string()),
                slices: Some(n),
            }
        })
    }

    /// Chop a sample into n slices within each event.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // sequence({ sound: 'a' }, { sound: 'b' })._chop(2)
    /// ```
    pub fn chop(&self, n: i64) -> Pattern<SampleValue> {
        if n <= 0 {
            return self.clone();
        }

        let pat = self.clone();
        let n_f = n as f64;

        pat.squeeze_join(move |sample| {
            let mut slices = Vec::new();
            let slice_size = (sample.end - sample.begin) / n_f;

            for i in 0..n {
                let i_f = i as f64;
                slices.push(pure(SampleValue {
                    sound: sample.sound.clone(),
                    begin: sample.begin + i_f * slice_size,
                    end: sample.begin + (i_f + 1.0) * slice_size,
                    speed: sample.speed,
                    unit: sample.unit.clone(),
                    slices: sample.slices,
                }));
            }

            fastcat(slices)
        })
    }

    /// Striate - interleave sample slices across the pattern.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // sequence({ sound: 'a' }).striate(2)
    /// ```
    pub fn striate(&self, n: i64) -> Pattern<SampleValue> {
        if n <= 0 {
            return self.clone();
        }

        let pat = self.clone();
        let n_f = n as f64;

        let mut slices = Vec::new();
        for i in 0..n {
            let i_f = i as f64;
            let slice_begin = i_f / n_f;
            let slice_end = (i_f + 1.0) / n_f;

            slices.push(pat.fmap(move |sample| SampleValue {
                sound: sample.sound.clone(),
                begin: sample.begin + (sample.end - sample.begin) * slice_begin,
                end: sample.begin + (sample.end - sample.begin) * slice_end,
                speed: sample.speed,
                unit: sample.unit.clone(),
                slices: sample.slices,
            }));
        }

        fastcat(slices)
    }
}

// ============================================================================
// Jux Operations (Tier 3)
// ============================================================================

/// Value type for stereo panning.
#[derive(Clone, Debug, PartialEq)]
pub struct PannedValue<T> {
    pub value: T,
    pub pan: f64,
}

impl<T: Clone + Send + Sync + 'static> Pattern<T> {
    /// Juxtapose - apply transformation and pan left/right.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // pure({ a: 1 }).jux(fast(2))
    /// ```
    pub fn jux<F>(&self, f: F) -> Pattern<PannedValue<T>>
    where
        F: Fn(&Pattern<T>) -> Pattern<T> + Send + Sync + 'static,
    {
        self.jux_by(1.0, f)
    }

    /// Juxtapose with custom pan amount.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // pure({ a: 1 }).juxBy(0.5, fast(2))
    /// ```
    pub fn jux_by<F>(&self, amount: f64, f: F) -> Pattern<PannedValue<T>>
    where
        F: Fn(&Pattern<T>) -> Pattern<T> + Send + Sync + 'static,
    {
        let left_pan = 0.5 - amount / 2.0;
        let right_pan = 0.5 + amount / 2.0;

        let left = self.fmap(move |v| PannedValue {
            value: v.clone(),
            pan: left_pan,
        });

        let right = f(self).fmap(move |v| PannedValue {
            value: v.clone(),
            pan: right_pan,
        });

        stack(vec![left, right])
    }
}

// ============================================================================
// Pick/Inhabit Operations (Tier 3)
// ============================================================================

impl Pattern<String> {
    /// Inhabit - replace string values with patterns from a map.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // sequence('a', 'b', stack('a', 'b')).inhabit({ a: sequence(1, 2), b: sequence(10, 20, 30) })
    /// ```
    pub fn inhabit<T: Clone + Send + Sync + 'static>(&self, patterns: HashMap<String, Pattern<T>>) -> Pattern<T> {
        let name_pat = self.clone();
        let patterns = Arc::new(patterns);

        name_pat.squeeze_join(move |name| {
            patterns.get(name).cloned().unwrap_or_else(silence)
        })
    }
}

impl Pattern<i64> {
    /// Pick - select patterns by index.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // sequence(0, 1, 0, stack(0, 1)).pick([sequence(1, 2, 3, 4), sequence(10, 20, 30, 40)])
    /// ```
    pub fn pick<T: Clone + Send + Sync + 'static>(&self, patterns: Vec<Pattern<T>>) -> Pattern<T> {
        if patterns.is_empty() {
            return silence();
        }

        let idx_pat = self.clone();
        let patterns = Arc::new(patterns);
        let len = patterns.len() as i64;

        idx_pat.squeeze_join(move |idx| {
            let clamped_idx = (*idx).max(0).min(len - 1) as usize;
            patterns.get(clamped_idx).cloned().unwrap_or_else(silence)
        })
    }

    /// Pickmod - select patterns by index with modulo wrapping.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // sequence(0, 1, 2, 3).pickmod([sequence(1, 2, 3, 4), sequence(10, 20, 30, 40)])
    /// ```
    pub fn pickmod<T: Clone + Send + Sync + 'static>(&self, patterns: Vec<Pattern<T>>) -> Pattern<T> {
        if patterns.is_empty() {
            return silence();
        }

        let idx_pat = self.clone();
        let patterns = Arc::new(patterns);
        let len = patterns.len() as i64;

        idx_pat.squeeze_join(move |idx| {
            let wrapped_idx = ((*idx % len + len) % len) as usize;
            patterns.get(wrapped_idx).cloned().unwrap_or_else(silence)
        })
    }
}

// ============================================================================
// Bite/Unjoin/Into Operations (Tier 3)
// ============================================================================

impl<T: Clone + Send + Sync + 'static> Pattern<T> {
    /// Bite into the pattern - slice and rearrange.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // fastcat(slowcat('a', 'b'), slowcat(1, 2)).bite(2, sequence(0, 1))
    /// ```
    pub fn bite(&self, n: i64, index_pat: &Pattern<i64>) -> Pattern<T> {
        let pat = self.clone();
        let index_pat = index_pat.clone();
        let n_frac = Fraction::from_integer(n);

        index_pat.squeeze_join(move |idx| {
            let idx_frac = Fraction::from_integer(*idx);
            let start = &idx_frac / &n_frac;
            let end = (&idx_frac + Fraction::from_integer(1)) / &n_frac;
            pat.compress(&start, &end)
        })
    }

    /// Apply function to subcycles of the pattern.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // fastcat('a', 'b', 'c', 'd').into(fastcat(fastcat(true, true), true), fast(2))
    /// ```
    pub fn into_subcycles<F>(&self, struct_pat: &Pattern<bool>, f: F) -> Pattern<T>
    where
        F: Fn(&Pattern<T>) -> Pattern<T> + Send + Sync + Clone + 'static,
    {
        let pat = self.clone();
        let struct_pat = struct_pat.clone();

        Pattern::new(move |state: &State| {
            let struct_haps: Vec<_> = struct_pat
                .query(state)
                .into_iter()
                .filter(|h| h.has_onset())
                .collect();

            let mut result = Vec::new();

            for struct_hap in struct_haps {
                let span = struct_hap.whole_or_part();
                let sub_pat = pat.focus_span(span);
                let transformed = f(&sub_pat);
                let focused_back = transformed.focus_span(span);

                for hap in focused_back.query(state) {
                    if let Some(part) = hap.part.intersection(&struct_hap.part) {
                        result.push(Hap::with_context(
                            hap.whole,
                            part,
                            hap.value.clone(),
                            hap.context,
                        ));
                    }
                }
            }

            result
        })
    }

    /// Chunk into subcycles.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // fastcat('a', 'b', 'c').chunkInto(3, fast(2)).fast(3)
    /// ```
    pub fn chunk_into<F>(&self, n: i64, f: F) -> Pattern<T>
    where
        F: Fn(&Pattern<T>) -> Pattern<T> + Send + Sync + Clone + 'static,
    {
        let pat = self.clone();
        let f = Arc::new(f);

        Pattern::new(move |state: &State| {
            let f = f.clone();
            state
                .span
                .span_cycles()
                .into_iter()
                .flat_map(|cycle_span| {
                    let cycle = cycle_span.begin.floor().to_f64() as i64;
                    let chunk_idx = cycle % n;
                    let f = f.clone();

                    // Create boolean pattern for this chunk
                    let mut bools = vec![pure(false); n as usize];
                    bools[chunk_idx as usize] = pure(true);
                    let struct_pat = fastcat(bools);

                    pat.into_subcycles(&struct_pat, move |p| f(p))
                        .query(&state.set_span(cycle_span))
                })
                .collect()
        })
    }

    /// Chunk into subcycles backwards.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // fastcat('a', 'b', 'c').chunkBackInto(3, fast(2)).fast(3)
    /// ```
    pub fn chunk_back_into<F>(&self, n: i64, f: F) -> Pattern<T>
    where
        F: Fn(&Pattern<T>) -> Pattern<T> + Send + Sync + Clone + 'static,
    {
        let pat = self.clone();
        let f = Arc::new(f);

        Pattern::new(move |state: &State| {
            let f = f.clone();
            state
                .span
                .span_cycles()
                .into_iter()
                .flat_map(|cycle_span| {
                    let cycle = cycle_span.begin.floor().to_f64() as i64;
                    let chunk_idx = (n - 1 - (cycle % n)).max(0) as usize;
                    let f = f.clone();

                    // Create boolean pattern for this chunk
                    let mut bools = vec![pure(false); n as usize];
                    bools[chunk_idx] = pure(true);
                    let struct_pat = fastcat(bools);

                    pat.into_subcycles(&struct_pat, move |p| f(p))
                        .query(&state.set_span(cycle_span))
                })
                .collect()
        })
    }
}

// ============================================================================
// Hurry (combines fast and speed for samples)
// ============================================================================

impl Pattern<SampleValue> {
    /// Hurry - speed up both pattern and playback.
    ///
    /// # Example (Strudel equivalent)
    /// ```ignore
    /// // s(sequence('a', 'b')).hurry(2)
    /// ```
    pub fn hurry(&self, factor: Fraction) -> Pattern<SampleValue> {
        let factor_f64 = factor.to_f64();
        self._fast(factor).fmap(move |sample| SampleValue {
            speed: Some(sample.speed.unwrap_or(1.0) * factor_f64),
            ..sample.clone()
        })
    }
}

// ============================================================================
// Additional Helper Methods
// ============================================================================

impl<T: Clone + Send + Sync + 'static> Pattern<T> {
    /// Sort haps by their part begin time.
    pub fn sort_haps_by_part(&self) -> Pattern<T> {
        let pat = self.clone();

        Pattern::new(move |state: &State| {
            let mut haps = pat.query(state);
            haps.sort_by(|a, b| a.part.begin.cmp(&b.part.begin));
            haps
        })
    }

    /// Get values from first cycle as a simple vec (for testing).
    pub fn first_cycle_values(&self) -> Vec<T> {
        self.query_arc(Fraction::from_integer(0), Fraction::from_integer(1))
            .into_iter()
            .map(|h| h.value)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Test Helpers (matching Strudel's test helpers)
    // ========================================================================

    fn ts(begin: f64, end: f64) -> TimeSpan {
        TimeSpan::new(Fraction::from(begin), Fraction::from(end))
    }

    fn hap<T: Clone>(whole: TimeSpan, part: TimeSpan, value: T) -> Hap<T> {
        Hap::new(Some(whole), part, value)
    }

    fn hap_continuous<T: Clone>(part: TimeSpan, value: T) -> Hap<T> {
        Hap::new(None, part, value)
    }

    /// Compare first cycles of two patterns (order-independent).
    fn same_first<T: Clone + Send + Sync + PartialEq + std::fmt::Debug + 'static>(
        a: &Pattern<T>,
        b: &Pattern<T>,
    ) -> bool {
        let mut haps_a = a.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        let mut haps_b = b.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        haps_a.sort_by(|x, y| x.part.begin.cmp(&y.part.begin));
        haps_b.sort_by(|x, y| x.part.begin.cmp(&y.part.begin));

        if haps_a.len() != haps_b.len() {
            eprintln!("Length mismatch: {} vs {}", haps_a.len(), haps_b.len());
            return false;
        }

        for (ha, hb) in haps_a.iter().zip(haps_b.iter()) {
            if ha.value != hb.value || ha.part != hb.part || ha.whole != hb.whole {
                eprintln!("Hap mismatch:\n  {:?}\n  {:?}", ha, hb);
                return false;
            }
        }

        true
    }

    fn third() -> Fraction {
        Fraction::new(1, 3)
    }

    fn two_thirds() -> Fraction {
        Fraction::new(2, 3)
    }

    // ========================================================================
    // TimeSpan Tests (matching Strudel lines 75-95)
    // ========================================================================

    #[test]
    fn test_timespan_equals() {
        assert!(ts(0.0, 4.0) == ts(0.0, 4.0));
    }

    #[test]
    fn test_timespan_split_cycles() {
        let span = ts(0.0, 2.0);
        assert_eq!(span.span_cycles().len(), 2);
    }

    #[test]
    fn test_timespan_intersection() {
        let a = ts(0.0, 2.0);
        let b = ts(1.0, 3.0);
        let c = ts(1.0, 2.0);
        assert_eq!(a.intersection(&b), Some(c));
    }

    // ========================================================================
    // Hap Tests (matching Strudel lines 97-148)
    // ========================================================================

    #[test]
    fn test_hap_has_onset() {
        let h = hap(ts(0.0, 1.0), ts(0.0, 1.0), "thing");
        assert!(h.has_onset());
    }

    #[test]
    fn test_hap_span_equals() {
        let a = hap(ts(0.0, 0.5), ts(0.0, 0.5), "a");
        let b = hap(ts(0.0, 0.5), ts(0.0, 0.5), "b");
        assert!(a.whole == b.whole && a.part == b.part);
    }

    #[test]
    fn test_hap_span_not_equals() {
        let a = hap(ts(0.0, 0.5), ts(0.0, 0.5), "a");
        let c = hap(ts(0.0, 0.25), ts(0.0, 0.5), "c");
        assert!(a.whole != c.whole);
    }

    #[test]
    fn test_hap_whole_or_part() {
        let discrete = hap(ts(0.0, 1.0), ts(0.0, 0.5), "hello");
        let continuous = hap_continuous(ts(0.0, 1.0), "hello");

        assert_eq!(discrete.whole_or_part(), &ts(0.0, 1.0));
        assert_eq!(continuous.whole_or_part(), &ts(0.0, 1.0));
    }

    // ========================================================================
    // Pattern Pure Tests (matching Strudel lines 150-157)
    // ========================================================================

    #[test]
    fn test_pure_can_make_pattern() {
        let haps = pure("hello").query_arc(Fraction::from(0.5), Fraction::from(2.5));
        assert_eq!(haps.len(), 3);
    }

    #[test]
    fn test_pure_zero_width_queries() {
        let haps = pure("hello").query_arc(Fraction::from_integer(0), Fraction::from_integer(0));
        assert_eq!(haps.len(), 1);
    }

    // ========================================================================
    // Fmap Tests (matching Strudel lines 158-166)
    // ========================================================================

    #[test]
    fn test_fmap_can_add() {
        let result = pure(3).fmap(|x| x + 4);
        let haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps[0].value, 7);
    }

    // ========================================================================
    // Add Tests (matching Strudel lines 172-229)
    // ========================================================================

    #[test]
    fn test_add_app_both() {
        let result = pure(4.0).app_both(&pure(5.0), |a, b| a + b);
        let haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps[0].value, 9.0);
    }

    #[test]
    fn test_add_app_left() {
        let result = pure(3.0).app_left(&pure(4.0), |a, b| a + b);
        let haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps[0].value, 7.0);
    }

    #[test]
    fn test_sub() {
        let result = pure(3.0).app_left(&pure(4.0), |a, b| a - b);
        let haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps[0].value, -1.0);
    }

    #[test]
    fn test_mul() {
        let result = pure(3.0).app_left(&pure(2.0), |a, b| a * b);
        let haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps[0].value, 6.0);
    }

    #[test]
    fn test_div() {
        let result = pure(3.0).app_left(&pure(2.0), |a, b| a / b);
        let haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps[0].value, 1.5);
    }

    // ========================================================================
    // Stack Tests (matching Strudel lines 387-398)
    // ========================================================================

    #[test]
    fn test_stack() {
        let result = stack(vec![pure("a"), pure("b"), pure("c")]);
        let values: Vec<_> = result
            .query_arc(Fraction::from_integer(0), Fraction::from_integer(1))
            .iter()
            .map(|h| h.value.clone())
            .collect();
        assert!(values.contains(&"a"));
        assert!(values.contains(&"b"));
        assert!(values.contains(&"c"));
    }

    // ========================================================================
    // Fast Tests (matching Strudel lines 399-457)
    // ========================================================================

    #[test]
    fn test_fast_makes_things_faster() {
        let result = pure("a")._fast(Fraction::from_integer(2));
        let haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps.len(), 2);
    }

    #[test]
    fn test_fast_with_pattern() {
        let result = pure("a").fast(fastcat(vec![pure(Fraction::from_integer(1)), pure(Fraction::from_integer(4))]));
        let haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps.len(), 3); // 1 + 2 events
    }

    // ========================================================================
    // Slow Tests (matching Strudel lines 459-487)
    // ========================================================================

    #[test]
    fn test_slow_makes_things_slower() {
        let result = pure("a")._slow(Fraction::from_integer(2));
        let haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps.len(), 1);
        assert_eq!(haps[0].whole.as_ref().unwrap().duration(), Fraction::from_integer(2));
    }

    // ========================================================================
    // Inside/Outside Tests (matching Strudel lines 488-497)
    // ========================================================================

    #[test]
    fn test_inside_rev() {
        let pat = fastcat(vec![pure("a"), pure("b"), pure("c"), pure("d")]);
        let result = pat.inside(Fraction::from_integer(2), |p| p.rev());
        let expected = fastcat(vec![pure("b"), pure("a"), pure("d"), pure("c")]);
        assert!(same_first(&result, &expected));
    }

    #[test]
    fn test_outside_rev() {
        let pat = fastcat(vec![pure("a"), pure("b"), pure("c"), pure("d")])._slow(Fraction::from_integer(2));
        let result = pat.outside(Fraction::from_integer(2), |p| p.rev());
        let expected = fastcat(vec![pure("d"), pure("c")]);
        assert!(same_first(&result, &expected));
    }

    // ========================================================================
    // FilterValues Tests (matching Strudel lines 498-506)
    // ========================================================================

    #[test]
    fn test_filter_values() {
        let result = pure(true).filter_values(|x| *x);
        let haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps.len(), 1);
    }

    // ========================================================================
    // When Tests (matching Strudel lines 507-531)
    // ========================================================================

    #[test]
    fn test_when_always_faster() {
        let result = pure("a").when(&pure(true), |x| x._fast(Fraction::from_integer(2)));
        let haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps.len(), 2);
    }

    #[test]
    fn test_when_never_faster() {
        let result = pure("a").when(&pure(false), |x| x._fast(Fraction::from_integer(2)));
        let haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps.len(), 1);
    }

    // ========================================================================
    // Fastcat/Slowcat Tests (matching Strudel lines 532-578)
    // ========================================================================

    #[test]
    fn test_fastcat_concatenate() {
        let result = fastcat(vec![pure("a"), pure("b")]);
        let values: Vec<_> = result
            .query_arc(Fraction::from_integer(0), Fraction::from_integer(1))
            .iter()
            .map(|h| h.value.clone())
            .collect();
        assert_eq!(values, vec!["a", "b"]);
    }

    #[test]
    fn test_slowcat_concatenate_slowly() {
        let result = slowcat(vec![pure("a"), pure("b")]);

        let cycle0: Vec<_> = result
            .query_arc(Fraction::from_integer(0), Fraction::from_integer(1))
            .iter()
            .map(|h| h.value.clone())
            .collect();
        assert_eq!(cycle0, vec!["a"]);

        let cycle1: Vec<_> = result
            .query_arc(Fraction::from_integer(1), Fraction::from_integer(2))
            .iter()
            .map(|h| h.value.clone())
            .collect();
        assert_eq!(cycle1, vec!["b"]);
    }

    // ========================================================================
    // Rev Tests (matching Strudel lines 579-601)
    // ========================================================================

    #[test]
    fn test_rev() {
        let pat = fastcat(vec![pure("a"), pure("b"), pure("c")]);
        let result = pat.rev();
        let mut haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        haps.sort_by(|a, b| a.part.begin.cmp(&b.part.begin));
        let values: Vec<_> = haps.iter().map(|h| h.value.clone()).collect();
        assert_eq!(values, vec!["c", "b", "a"]);
    }

    // ========================================================================
    // Palindrome Tests (matching Strudel lines 607-618)
    // ========================================================================

    #[test]
    fn test_palindrome() {
        let pat = fastcat(vec![pure("a"), pure("b"), pure("c")]);
        let result = pat.palindrome()._fast(Fraction::from_integer(2));
        let mut haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        haps.sort_by(|a, b| a.part.begin.cmp(&b.part.begin));
        let values: Vec<_> = haps.iter().map(|h| h.value.clone()).collect();
        assert_eq!(values, vec!["a", "b", "c", "c", "b", "a"]);
    }

    // ========================================================================
    // Polyrhythm/Polymeter Tests (matching Strudel lines 619-632)
    // ========================================================================

    #[test]
    fn test_polyrhythm() {
        let result = polyrhythm(vec![
            vec![pure("a"), pure("b")],
            vec![pure("c")],
        ]);
        let expected = stack(vec![
            fastcat(vec![pure("a"), pure("b")]),
            pure("c"),
        ]);
        assert!(same_first(&result, &expected));
    }

    // ========================================================================
    // FirstOf Tests (matching Strudel lines 634-667)
    // ========================================================================

    #[test]
    fn test_first_of() {
        let result = pure("a")
            .first_of(3, |x| x._fast(Fraction::from_integer(2)))
            ._fast(Fraction::from_integer(3));

        let expected = fastcat(vec![
            fastcat(vec![pure("a"), pure("a")]),
            pure("a"),
            pure("a"),
        ]);

        assert!(same_first(&result, &expected));
    }

    // ========================================================================
    // Brak Tests (matching Strudel lines 668-672)
    // ========================================================================

    #[test]
    fn test_brak() {
        let pat = fastcat(vec![pure("a"), pure("b")]);
        let result = pat.brak()._fast(Fraction::from_integer(2));
        // Should be: a b, silence a, b silence
        let haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert!(haps.len() >= 3); // At least some events
    }

    // ========================================================================
    // TimeCat Tests (matching Strudel lines 673-679)
    // ========================================================================

    #[test]
    fn test_timecat() {
        let result = timecat(vec![
            (Fraction::from_integer(1), pure("a")),
            (Fraction::new(1, 2), pure("a")),
            (Fraction::new(1, 2), pure("a")),
        ]);
        let expected = fastcat(vec![pure("a"), fastcat(vec![pure("a"), pure("a")])]);
        assert!(same_first(&result, &expected));
    }

    // ========================================================================
    // Struct Tests (matching Strudel lines 680-714)
    // ========================================================================

    #[test]
    fn test_struct_basic() {
        let pat = fastcat(vec![pure("a"), pure("b")]);
        let bool_pat = fastcat(vec![pure(true), pure(true), pure(true)]);
        let result = pat.struct_pat(&bool_pat);
        let haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        // Should have 4 haps matching Strudel's output
        assert_eq!(haps.len(), 4);
    }

    #[test]
    fn test_struct_with_silence() {
        let result = pure("a").struct_pat(&fastcat(vec![pure(true), fastcat(vec![pure(true), pure(false)]), pure(true)]));
        let expected = fastcat(vec![pure("a"), fastcat(vec![pure("a"), silence()]), pure("a")]);
        assert!(same_first(&result, &expected));
    }

    // ========================================================================
    // Mask Tests (matching Strudel lines 715-735)
    // ========================================================================

    #[test]
    fn test_mask() {
        let pat = fastcat(vec![fastcat(vec![pure("a"), pure("b")]), pure("c")]);
        let bool_pat = fastcat(vec![pure(true), pure(false)]);
        let result = pat.mask(&bool_pat);
        let expected = fastcat(vec![fastcat(vec![pure("a"), pure("b")]), silence()]);
        assert!(same_first(&result, &expected));
    }

    // ========================================================================
    // Invert Tests (matching Strudel lines 736-742)
    // ========================================================================

    #[test]
    fn test_invert() {
        let pat = fastcat(vec![pure(true), pure(false), fastcat(vec![pure(true), pure(false)])]);
        let result = pat.invert();
        let expected = fastcat(vec![pure(false), pure(true), fastcat(vec![pure(false), pure(true)])]);
        assert!(same_first(&result, &expected));
    }

    // ========================================================================
    // Early/Late Tests (matching Strudel lines 816-829)
    // ========================================================================

    #[test]
    fn test_late() {
        let result = pure(30)._late(Fraction::new(1, 4));
        let haps = result.query_arc(Fraction::from_integer(1), Fraction::from_integer(2));

        assert_eq!(haps.len(), 2);
        // Check timing
        let h0 = &haps[0];
        assert_eq!(h0.whole.as_ref().unwrap().begin, Fraction::new(1, 4));
    }

    // ========================================================================
    // Off Tests (matching Strudel lines 830-836)
    // ========================================================================

    #[test]
    fn test_off() {
        let result = pure(30.0).off(Fraction::new(1, 4), |p| p.app_left(&pure(2.0), |a, b| a + b));
        let expected = stack(vec![
            pure(30.0),
            pure(30.0)._late(Fraction::new(1, 4)).app_left(&pure(2.0), |a, b| a + b),
        ]);
        assert!(same_first(&result, &expected));
    }

    // ========================================================================
    // Ply Tests (matching Strudel lines 881-891)
    // ========================================================================

    #[test]
    fn test_ply() {
        let pat = fastcat(vec![pure("a"), fastcat(vec![pure("b"), pure("c")])]);
        let result = pat.ply(3);
        let expected = fastcat(vec![
            pure("a")._fast(Fraction::from_integer(3)),
            fastcat(vec![
                pure("b")._fast(Fraction::from_integer(3)),
                pure("c")._fast(Fraction::from_integer(3)),
            ]),
        ]);
        assert!(same_first(&result, &expected));
    }

    // ========================================================================
    // Range Tests (matching Strudel lines 932-943)
    // ========================================================================

    #[test]
    fn test_range() {
        let pat = fastcat(vec![pure(0.0), pure(0.0)]);
        let min_pat = fastcat(vec![pure(0.0), pure(0.5)]);
        // For now just test basic range
        let result = pat.range(0.0, 1.0);
        let expected = fastcat(vec![pure(0.0), pure(0.0)]);
        assert!(same_first(&result, &expected));
    }

    #[test]
    fn test_range2() {
        let pat = fastcat(vec![pure(-1.0), pure(-0.5), pure(0.0), pure(0.5)]);
        let result = pat.range2(1000.0, 1100.0);
        let expected = fastcat(vec![pure(1000.0), pure(1025.0), pure(1050.0), pure(1075.0)]);
        assert!(same_first(&result, &expected));
    }

    // ========================================================================
    // Run Tests (matching Strudel lines 944-948)
    // ========================================================================

    #[test]
    fn test_run() {
        let result = run(4);
        let expected = fastcat(vec![pure(0i64), pure(1), pure(2), pure(3)]);
        assert!(same_first(&result, &expected));
    }

    // ========================================================================
    // BinaryN Tests (matching Strudel lines 949-960)
    // ========================================================================

    #[test]
    fn test_binary_n() {
        let result = binary_n(55532, Some(16));
        let expected = fastcat(vec![
            pure(1i64), pure(1), pure(0), pure(1),
            pure(1), pure(0), pure(0), pure(0),
            pure(1), pure(1), pure(1), pure(0),
            pure(1), pure(1), pure(0), pure(0),
        ]);
        assert!(same_first(&result, &expected));
    }

    // ========================================================================
    // Ribbon Tests (matching Strudel lines 961-967)
    // ========================================================================

    #[test]
    fn test_ribbon() {
        let pat = slowcat(vec![
            pure(0), pure(1), pure(2), pure(3),
            pure(4), pure(5), pure(6), pure(7),
        ]);
        let result = pat.ribbon(2, 4)._fast(Fraction::from_integer(4));
        let expected = fastcat(vec![pure(2), pure(3), pure(4), pure(5)]);
        assert!(same_first(&result, &expected));
    }

    // ========================================================================
    // Linger Tests (matching Strudel lines 968-974)
    // ========================================================================

    #[test]
    fn test_linger() {
        let pat = fastcat(vec![
            pure(0), pure(1), pure(2), pure(3),
            pure(4), pure(5), pure(6), pure(7),
        ]);
        let result = pat.linger(Fraction::new(1, 4));
        let expected = fastcat(vec![pure(0), pure(1), pure(0), pure(1), pure(0), pure(1), pure(0), pure(1)]);
        assert!(same_first(&result, &expected));
    }

    // ========================================================================
    // DefragmentHaps Tests (matching Strudel lines 980-997)
    // ========================================================================

    #[test]
    fn test_defragment_haps_merge() {
        let pat = stack(vec![
            pure("a").mask(&fastcat(vec![pure(true), pure(false)])),
            pure("a").mask(&fastcat(vec![pure(false), pure(true)])),
        ]);
        let result = pat.defragment_haps();
        let haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps.len(), 1);
    }

    #[test]
    fn test_defragment_haps_no_merge_different_values() {
        let pat = stack(vec![
            pure("a").mask(&fastcat(vec![pure(true), pure(false)])),
            pure("b").mask(&fastcat(vec![pure(false), pure(true)])),
        ]);
        let result = pat.defragment_haps();
        let haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert_eq!(haps.len(), 2);
    }

    // ========================================================================
    // Press Tests (matching Strudel lines 998-1002)
    // ========================================================================

    #[test]
    fn test_press() {
        let pat = fastcat(vec![pure("a"), pure("b"), pure("c"), pure("d")]);
        let result = pat.press();
        // Events should be shifted by half their duration
        let haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert!(haps.len() >= 2); // Some events will shift into next cycle
    }

    // ========================================================================
    // RepeatCycles Tests (matching Strudel lines 1066-1070)
    // ========================================================================

    #[test]
    fn test_repeat_cycles() {
        let pat = slowcat(vec![pure(0), pure(1)]);
        let result = pat.repeat_cycles(2)._fast(Fraction::from_integer(6));
        let values = result.first_cycle_values();
        assert_eq!(values, vec![0, 0, 1, 1, 0, 0]);
    }

    // ========================================================================
    // Shrink Tests (matching Strudel lines 1187-1194)
    // ========================================================================

    #[test]
    fn test_shrink() {
        let pat = fastcat(vec![pure(0), pure(1), pure(2), pure(3), pure(4)]);
        let result = pat.shrink(1);
        // Progressive shrinking from the left
        let haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert!(haps.len() > 5); // Multiple sub-patterns
    }

    // ========================================================================
    // Grow Tests (matching Strudel lines 1195-1202)
    // ========================================================================

    #[test]
    fn test_grow() {
        let pat = fastcat(vec![pure(0), pure(1), pure(2), pure(3), pure(4)]);
        let result = pat.grow(1);
        let haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));
        assert!(haps.len() > 5); // Multiple sub-patterns
    }

    // ========================================================================
    // Take/Drop Tests (matching Strudel lines 1203-1225)
    // ========================================================================

    #[test]
    fn test_take_from_left() {
        let pat = fastcat(vec![pure(0), pure(1), pure(2), pure(3), pure(4)]);
        let result = pat.take(2);
        let expected = fastcat(vec![pure(0), pure(1)]);
        assert!(same_first(&result, &expected));
    }

    #[test]
    fn test_drop_from_left() {
        let pat = fastcat(vec![pure(0), pure(1), pure(2), pure(3), pure(4)]);
        let result = pat.drop_steps(2);
        let expected = fastcat(vec![pure(2), pure(3), pure(4)]);
        assert!(same_first(&result, &expected));
    }

    #[test]
    fn test_take_from_right() {
        let pat = fastcat(vec![pure(0), pure(1), pure(2), pure(3), pure(4)]);
        let result = pat.take(-2);
        let expected = fastcat(vec![pure(3), pure(4)]);
        assert!(same_first(&result, &expected));
    }

    #[test]
    fn test_drop_from_right() {
        let pat = fastcat(vec![pure(0), pure(1), pure(2), pure(3), pure(4)]);
        let result = pat.drop_steps(-2);
        let expected = fastcat(vec![pure(0), pure(1), pure(2)]);
        assert!(same_first(&result, &expected));
    }

    // ========================================================================
    // Jux Tests (matching Strudel lines 837-854)
    // ========================================================================

    #[test]
    fn test_jux() {
        let result = pure(1).jux(|p| p._fast(Fraction::from_integer(2)));
        let haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        // Should have original (pan=0) and transformed (pan=1)
        assert!(haps.len() >= 2);
        let pans: Vec<_> = haps.iter().map(|h| h.value.pan).collect();
        assert!(pans.contains(&0.0));
        assert!(pans.contains(&1.0));
    }

    #[test]
    fn test_jux_by() {
        let result = pure(1).jux_by(0.5, |p| p._fast(Fraction::from_integer(2)));
        let haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        let pans: Vec<_> = haps.iter().map(|h| h.value.pan).collect();
        assert!(pans.contains(&0.25)); // 0.5 - 0.5/2
        assert!(pans.contains(&0.75)); // 0.5 + 0.5/2
    }

    // ========================================================================
    // FastGap Tests (matching Strudel lines 404-424)
    // ========================================================================

    #[test]
    fn test_fast_gap() {
        let pat = fastcat(vec![pure("a"), pure("b"), pure("c")]);
        let result = pat.fast_gap(Fraction::from_integer(2));
        let expected = fastcat(vec![
            fastcat(vec![pure("a"), pure("b"), pure("c")]),
            silence(),
        ]);
        assert!(same_first(&result, &expected));
    }

    // ========================================================================
    // Sample Operations Tests
    // ========================================================================

    #[test]
    fn test_sample_slice() {
        let sample = pure(SampleValue {
            sound: "break".to_string(),
            begin: 0.0,
            end: 1.0,
            speed: None,
            unit: None,
            slices: None,
        });

        let indices = fastcat(vec![pure(0i64), pure(1), pure(2), pure(3)]);
        let result = sample.slice(4, &indices);
        let haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        assert_eq!(haps.len(), 4);
        assert_eq!(haps[0].value.begin, 0.0);
        assert_eq!(haps[0].value.end, 0.25);
        assert_eq!(haps[1].value.begin, 0.25);
        assert_eq!(haps[1].value.end, 0.5);
    }

    #[test]
    fn test_sample_striate() {
        let sample = pure(SampleValue {
            sound: "a".to_string(),
            begin: 0.0,
            end: 1.0,
            speed: None,
            unit: None,
            slices: None,
        });

        let result = sample.striate(2);
        let haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        assert_eq!(haps.len(), 2);
        assert_eq!(haps[0].value.begin, 0.0);
        assert_eq!(haps[0].value.end, 0.5);
        assert_eq!(haps[1].value.begin, 0.5);
        assert_eq!(haps[1].value.end, 1.0);
    }

    #[test]
    fn test_sample_chop() {
        let sample = pure(SampleValue {
            sound: "a".to_string(),
            begin: 0.0,
            end: 1.0,
            speed: None,
            unit: None,
            slices: None,
        });

        let result = sample.chop(2);
        let haps = result.query_arc(Fraction::from_integer(0), Fraction::from_integer(1));

        assert_eq!(haps.len(), 2);
    }
}
