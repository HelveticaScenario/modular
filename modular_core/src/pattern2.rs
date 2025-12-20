// IGNORE THIS FILE

// pattern.rs - Core pattern representation for strudel in Rust
// Converted from JavaScript to idiomatic Rust
// Copyright (C) 2025 Strudel contributors
// AGPL-3.0-or-later

use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::rc::Rc;
use std::sync::Arc;

// ============================================================================
// Core Types
// ============================================================================

/// Represents a rational number (fraction)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Fraction {
    numerator: i64,
    denominator: i64,
}

impl Fraction {
    pub fn new(numerator: i64, denominator: i64) -> Self {
        let gcd = Self::gcd(numerator.abs(), denominator.abs());
        Self {
            numerator: numerator / gcd,
            denominator: denominator / gcd,
        }
    }

    pub fn from_float(f: f64) -> Self {
        const PRECISION: i64 = 1_000_000;
        Self::new((f * PRECISION as f64) as i64, PRECISION)
    }

    fn gcd(a: i64, b: i64) -> i64 {
        if b == 0 { a } else { Self::gcd(b, a % b) }
    }

    pub fn to_float(&self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }

    pub fn add(&self, other: &Self) -> Self {
        Self::new(
            self.numerator * other.denominator + other.numerator * self.denominator,
            self.denominator * other.denominator,
        )
    }

    pub fn sub(&self, other: &Self) -> Self {
        Self::new(
            self.numerator * other.denominator - other.numerator * self.denominator,
            self.denominator * other.denominator,
        )
    }

    pub fn mul(&self, other: &Self) -> Self {
        Self::new(
            self.numerator * other.numerator,
            self.denominator * other.denominator,
        )
    }

    pub fn div(&self, other: &Self) -> Self {
        Self::new(
            self.numerator * other.denominator,
            self.denominator * other.numerator,
        )
    }

    pub fn cycle_pos(&self) -> Self {
        let rem = self.numerator.rem_euclid(self.denominator);
        Self::new(rem, self.denominator)
    }
}

impl From<i64> for Fraction {
    fn from(n: i64) -> Self {
        Self::new(n, 1)
    }
}

/// Represents a time span with begin and end times
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimeSpan {
    pub begin: Fraction,
    pub end: Fraction,
}

impl TimeSpan {
    pub fn new(begin: Fraction, end: Fraction) -> Self {
        Self { begin, end }
    }

    pub fn duration(&self) -> Fraction {
        self.end.sub(&self.begin)
    }

    pub fn intersection(&self, other: &TimeSpan) -> Option<TimeSpan> {
        let begin = if self.begin > other.begin {
            self.begin
        } else {
            other.begin
        };
        let end = if self.end < other.end {
            self.end
        } else {
            other.end
        };

        if begin < end {
            Some(TimeSpan::new(begin, end))
        } else {
            None
        }
    }

    pub fn with_time<F>(&self, func: F) -> Self
    where
        F: Fn(Fraction) -> Fraction,
    {
        Self::new(func(self.begin), func(self.end))
    }

    pub fn span_cycles(&self) -> Vec<TimeSpan> {
        let begin_cycle = self.begin.to_float().floor() as i64;
        let end_cycle = self.end.to_float().ceil() as i64;

        (begin_cycle..end_cycle)
            .map(|cycle| {
                let cycle_begin = Fraction::from(cycle);
                let cycle_end = Fraction::from(cycle + 1);
                let span_begin = if self.begin > cycle_begin {
                    self.begin
                } else {
                    cycle_begin
                };
                let span_end = if self.end < cycle_end {
                    self.end
                } else {
                    cycle_end
                };
                TimeSpan::new(span_begin, span_end)
            })
            .collect()
    }
}

/// Context associated with a hap (event metadata)
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Context {
    pub locations: Vec<Location>,
    #[serde(skip)]
    pub on_trigger: Option<Arc<dyn Fn(&Hap<Value>) + Send + Sync>>,
    pub dominant_trigger: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Location {
    pub start: usize,
    pub end: usize,
}

/// State for pattern queries
#[derive(Debug, Clone)]
pub struct State {
    pub span: TimeSpan,
    pub controls: std::collections::HashMap<String, Value>,
}

impl State {
    pub fn new(span: TimeSpan) -> Self {
        Self {
            span,
            controls: std::collections::HashMap::new(),
        }
    }

    pub fn set_span(&self, span: TimeSpan) -> Self {
        Self {
            span,
            controls: self.controls.clone(),
        }
    }

    pub fn with_span<F>(&self, func: F) -> Self
    where
        F: Fn(&TimeSpan) -> TimeSpan,
    {
        self.set_span(func(&self.span))
    }
}

/// A hap (happening/event) with a value
#[derive(Clone, Serialize, Deserialize)]
pub struct Hap<T> {
    pub whole: Option<TimeSpan>,
    pub part: TimeSpan,
    pub value: T,
    #[serde(skip)]
    pub context: Context,
}

impl<T> Hap<T> {
    pub fn new(whole: Option<TimeSpan>, part: TimeSpan, value: T) -> Self {
        Self {
            whole,
            part,
            value,
            context: Context::default(),
        }
    }

    pub fn with_value<U, F>(self, func: F) -> Hap<U>
    where
        F: FnOnce(T) -> U,
    {
        Hap {
            whole: self.whole,
            part: self.part,
            value: func(self.value),
            context: self.context,
        }
    }

    pub fn with_span<F>(self, func: F) -> Self
    where
        F: Fn(&TimeSpan) -> TimeSpan,
    {
        Self {
            whole: self.whole.as_ref().map(&func),
            part: func(&self.part),
            value: self.value,
            context: self.context,
        }
    }

    pub fn set_context(mut self, context: Context) -> Self {
        self.context = context;
        self
    }

    pub fn combine_context(&self, other: &Hap<impl Clone>) -> Context {
        let mut context = self.context.clone();
        context.locations.extend(other.context.locations.clone());
        context
    }

    pub fn has_onset(&self) -> bool {
        self.whole
            .as_ref()
            .map(|w| w.begin == self.part.begin)
            .unwrap_or(false)
    }

    pub fn whole_or_part(&self) -> TimeSpan {
        self.whole.clone().unwrap_or_else(|| self.part.clone())
    }

    pub fn span_equals(&self, other: &Hap<T>) -> bool {
        self.whole == other.whole && self.part == other.part
    }
}

// ============================================================================
// Value Type (for dynamic values)
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Value {
    Number(f64),
    String(String),
    Bool(bool),
    Object(std::collections::HashMap<String, Value>),
    Array(Vec<Value>),
    Null,
}

impl Value {
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Value::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn as_string(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }
}

// ============================================================================
// Pattern Operations (Declarative/Serializable)
// ============================================================================

/// Operations that can be applied to pattern values
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValueOperation {
    // Arithmetic
    Add(f64),
    Sub(f64),
    Mul(f64),
    Div(f64),
    Mod(f64),
    Pow(f64),

    // String operations
    Append(String),
    Prepend(String),

    // Transformations
    Negate,
    Abs,
    Floor,
    Ceil,
    Round,

    // Logical
    Not,
}

/// Time-based pattern transformations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeOperation {
    Fast(f64),
    Slow(f64),
    Early(f64),
    Late(f64),
    Rev,
    Jux(Box<PatternTransform>),
}

/// High-level pattern transformations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PatternTransform {
    ValueOp(ValueOperation),
    TimeOp(TimeOperation),
    FilterOnsets,
    FilterDiscrete,
    SplitQueries,
    Degraded(f64),
    Sometimes(f64, Box<PatternTransform>),
    Stack(Vec<PatternTransform>),
    Sequence(Vec<PatternTransform>),
    Chain(Vec<PatternTransform>),
}

// ============================================================================
// Pattern Query Function
// ============================================================================

/// Function that queries a pattern for haps in a given state
pub type QueryFn<T> = Arc<dyn Fn(&State) -> Vec<Hap<T>> + Send + Sync>;

// ============================================================================
// Pattern Structure
// ============================================================================

/// Core Pattern structure
pub struct Pattern<T = Value> {
    query: QueryFn<T>,
    steps: Option<Fraction>,
}

impl<T: Clone> Clone for Pattern<T> {
    fn clone(&self) -> Self {
        Self {
            query: Arc::clone(&self.query),
            steps: self.steps,
        }
    }
}

impl<T> Pattern<T> {
    /// Create a new pattern from a query function
    pub fn new<F>(query: F) -> Self
    where
        F: Fn(&State) -> Vec<Hap<T>> + Send + Sync + 'static,
    {
        Self {
            query: Arc::new(query),
            steps: None,
        }
    }

    /// Create a pattern with steps information
    pub fn with_steps<F>(query: F, steps: Option<Fraction>) -> Self
    where
        F: Fn(&State) -> Vec<Hap<T>> + Send + Sync + 'static,
    {
        Self {
            query: Arc::new(query),
            steps,
        }
    }

    /// Query the pattern for a given state
    pub fn query(&self, state: &State) -> Vec<Hap<T>> {
        (self.query)(state)
    }

    /// Query haps inside the given time span
    pub fn query_arc(&self, begin: Fraction, end: Fraction) -> Vec<Hap<T>> {
        self.query(&State::new(TimeSpan::new(begin, end)))
    }

    /// Get steps information
    pub fn steps(&self) -> Option<Fraction> {
        self.steps
    }

    /// Set steps information
    pub fn set_steps(mut self, steps: Option<Fraction>) -> Self {
        self.steps = steps;
        self
    }

    // ========================================================================
    // Functor operations
    // ========================================================================

    /// Map a function over pattern values (functor fmap)
    pub fn fmap<U, F>(self, func: F) -> Pattern<U>
    where
        F: Fn(T) -> U + Send + Sync + 'static,
        T: 'static,
    {
        let query = self.query;
        let steps = self.steps;
        Pattern::with_steps(
            move |state| {
                query(state)
                    .into_iter()
                    .map(|hap| hap.with_value(&func))
                    .collect()
            },
            steps,
        )
    }

    /// Alias for fmap
    pub fn with_value<U, F>(self, func: F) -> Pattern<U>
    where
        F: Fn(T) -> U + Send + Sync + 'static,
        T: 'static,
    {
        self.fmap(func)
    }

    // ========================================================================
    // Query transformations
    // ========================================================================

    /// Apply a function to the query state
    pub fn with_state<F>(self, func: F) -> Self
    where
        F: Fn(State) -> State + Send + Sync + 'static,
        T: 'static,
    {
        let query = self.query;
        Pattern::with_steps(move |state| query(&func(state.clone())), self.steps)
    }

    /// Apply function to query timespan
    pub fn with_query_span<F>(self, func: F) -> Self
    where
        F: Fn(&TimeSpan) -> TimeSpan + Send + Sync + 'static,
        T: 'static,
    {
        self.with_state(move |state| state.with_span(&func))
    }

    /// Apply function to query times
    pub fn with_query_time<F>(self, func: F) -> Self
    where
        F: Fn(Fraction) -> Fraction + Send + Sync + 'static,
        T: 'static,
    {
        self.with_query_span(move |span| span.with_time(&func))
    }

    /// Apply function to hap timespans
    pub fn with_hap_span<F>(self, func: F) -> Self
    where
        F: Fn(&TimeSpan) -> TimeSpan + Send + Sync + 'static,
        T: 'static,
    {
        let query = self.query;
        Pattern::with_steps(
            move |state| {
                query(state)
                    .into_iter()
                    .map(|hap| hap.with_span(&func))
                    .collect()
            },
            self.steps,
        )
    }

    /// Apply function to hap times
    pub fn with_hap_time<F>(self, func: F) -> Self
    where
        F: Fn(Fraction) -> Fraction + Send + Sync + 'static,
        T: 'static,
    {
        self.with_hap_span(move |span| span.with_time(&func))
    }

    // ========================================================================
    // Filtering operations
    // ========================================================================

    /// Filter haps by predicate
    pub fn filter_haps<F>(self, predicate: F) -> Self
    where
        F: Fn(&Hap<T>) -> bool + Send + Sync + 'static,
        T: 'static,
    {
        let query = self.query;
        Pattern::with_steps(
            move |state| {
                query(state)
                    .into_iter()
                    .filter(|hap| predicate(hap))
                    .collect()
            },
            self.steps,
        )
    }

    /// Filter haps by value predicate
    pub fn filter_values<F>(self, predicate: F) -> Self
    where
        F: Fn(&T) -> bool + Send + Sync + 'static,
        T: 'static,
    {
        self.filter_haps(move |hap| predicate(&hap.value))
    }

    /// Remove continuous haps (keep only discrete events)
    pub fn discrete_only(self) -> Self
    where
        T: 'static,
    {
        self.filter_haps(|hap| hap.whole.is_some())
    }

    /// Keep only haps with onsets
    pub fn onsets_only(self) -> Self
    where
        T: 'static,
    {
        self.filter_haps(|hap| hap.has_onset())
    }

    /// Split queries at cycle boundaries
    pub fn split_queries(self) -> Self
    where
        T: 'static,
    {
        let query = self.query;
        Pattern::with_steps(
            move |state| {
                state
                    .span
                    .span_cycles()
                    .into_iter()
                    .flat_map(|subspan| query(&state.set_span(subspan)))
                    .collect()
            },
            self.steps,
        )
    }

    // ========================================================================
    // Time transformations (user-facing with enums)
    // ========================================================================

    /// Speed up pattern by factor
    pub fn fast(self, factor: f64) -> Self
    where
        T: 'static,
    {
        let factor_frac = Fraction::from_float(factor);
        self.with_query_time(move |t| t.mul(&factor_frac))
            .with_hap_time(move |t| t.div(&factor_frac))
            .set_steps(self.steps.map(|s| s.mul(&factor_frac)))
    }

    /// Slow down pattern by factor
    pub fn slow(self, factor: f64) -> Self
    where
        T: 'static,
    {
        self.fast(1.0 / factor)
    }

    /// Shift pattern earlier in time
    pub fn early(self, amount: f64) -> Self
    where
        T: 'static,
    {
        let amount_frac = Fraction::from_float(amount);
        self.with_query_time(move |t| t.add(&amount_frac))
            .with_hap_time(move |t| t.sub(&amount_frac))
    }

    /// Shift pattern later in time
    pub fn late(self, amount: f64) -> Self
    where
        T: 'static,
    {
        self.early(-amount)
    }

    /// Apply a pattern transformation declaratively
    pub fn apply_transform(self, transform: PatternTransform) -> Self
    where
        T: 'static + Into<Value> + From<Value>,
        Value: From<T>,
    {
        match transform {
            PatternTransform::TimeOp(TimeOperation::Fast(factor)) => self.fast(factor),
            PatternTransform::TimeOp(TimeOperation::Slow(factor)) => self.slow(factor),
            PatternTransform::TimeOp(TimeOperation::Early(amount)) => self.early(amount),
            PatternTransform::TimeOp(TimeOperation::Late(amount)) => self.late(amount),
            PatternTransform::FilterOnsets => self.onsets_only(),
            PatternTransform::FilterDiscrete => self.discrete_only(),
            PatternTransform::SplitQueries => self.split_queries(),
            PatternTransform::Chain(transforms) => transforms
                .into_iter()
                .fold(self, |pat, t| pat.apply_transform(t)),
            _ => self, // TODO: implement remaining transforms
        }
    }
}

// ============================================================================
// Pattern constructors
// ============================================================================

impl<T: Clone + 'static> Pattern<T> {
    /// Create a pure/constant pattern
    pub fn pure(value: T) -> Self {
        Pattern::new(move |state| {
            vec![Hap::new(
                Some(state.span.clone()),
                state.span.clone(),
                value.clone(),
            )]
        })
    }

    /// Create a silent/empty pattern
    pub fn silence() -> Self {
        Pattern::new(|_| vec![])
    }
}

// ============================================================================
// Applicative operations
// ============================================================================

impl<T: Clone + 'static> Pattern<T> {
    /// Apply a pattern of functions to this pattern (structure from left)
    pub fn app_left<U, F>(self, pat_val: Pattern<U>) -> Pattern<T>
    where
        F: Fn(U) -> T + Clone + 'static,
        T: From<fn(U) -> T>,
        U: Clone + 'static,
    {
        let pat_func = self;
        Pattern::new(move |state| {
            let mut haps = Vec::new();
            for hap_func in pat_func.query(state) {
                let hap_vals = pat_val.query(&state.set_span(hap_func.whole_or_part()));
                for hap_val in hap_vals {
                    if let Some(new_part) = hap_func.part.intersection(&hap_val.part) {
                        let context = hap_val.combine_context(&hap_func);
                        haps.push(Hap {
                            whole: hap_func.whole.clone(),
                            part: new_part,
                            value: hap_func.value.clone(),
                            context,
                        });
                    }
                }
            }
            haps
        })
    }
}

// ============================================================================
// Monadic operations
// ============================================================================

impl<T: Clone + 'static> Pattern<T> {
    /// Bind operation (flatmap)
    pub fn bind<U, F>(self, func: F) -> Pattern<U>
    where
        F: Fn(T) -> Pattern<U> + Send + Sync + 'static,
        U: Clone + 'static,
    {
        let pat_val = self;
        Pattern::new(move |state| {
            pat_val
                .query(state)
                .into_iter()
                .flat_map(|hap_a| {
                    func(hap_a.value.clone())
                        .query(&state.set_span(hap_a.part.clone()))
                        .into_iter()
                        .filter_map(move |hap_b| {
                            let whole = match (&hap_a.whole, &hap_b.whole) {
                                (Some(w_a), Some(w_b)) => w_a.intersection(w_b),
                                _ => None,
                            };
                            hap_a.part.intersection(&hap_b.part).map(|part| Hap {
                                whole,
                                part,
                                value: hap_b.value.clone(),
                                context: hap_b.combine_context(&hap_a),
                            })
                        })
                })
                .collect()
        })
    }

    /// Flatten a pattern of patterns
    pub fn join(self) -> Pattern<T>
    where
        T: Into<Pattern<T>>,
    {
        self.bind(|p| p.into())
    }

    /// Outer join - preserve structure from outer pattern
    pub fn outer_join(self) -> Pattern<T>
    where
        T: Into<Pattern<T>>,
    {
        let pat_of_pats = self;
        Pattern::new(move |state| {
            pat_of_pats
                .query(state)
                .into_iter()
                .flat_map(|outer_hap| {
                    let inner_pat: Pattern<T> = outer_hap.value.clone().into();
                    inner_pat
                        .query(&state.set_span(outer_hap.whole_or_part()))
                        .into_iter()
                        .filter_map(move |inner_hap| {
                            let whole = outer_hap.whole.clone();
                            outer_hap
                                .part
                                .intersection(&inner_hap.part)
                                .map(|part| Hap {
                                    whole,
                                    part,
                                    value: inner_hap.value.clone(),
                                    context: inner_hap.combine_context(&outer_hap),
                                })
                        })
                })
                .collect()
        })
    }
}

// ============================================================================
// Multi-pattern operations
// ============================================================================

/// Stack multiple patterns together
pub fn stack<T: Clone + 'static>(patterns: Vec<Pattern<T>>) -> Pattern<T> {
    Pattern::new(move |state| patterns.iter().flat_map(|pat| pat.query(state)).collect())
}

/// Concatenate patterns sequentially
pub fn fastcat<T: Clone + 'static>(patterns: Vec<Pattern<T>>) -> Pattern<T> {
    let n = patterns.len();
    if n == 0 {
        return Pattern::silence();
    }

    Pattern::new(move |state| {
        patterns
            .iter()
            .enumerate()
            .flat_map(|(i, pat)| {
                let begin = Fraction::from(i as i64).div(&Fraction::from(n as i64));
                let end = Fraction::from((i + 1) as i64).div(&Fraction::from(n as i64));

                let mut haps = Vec::new();
                for cycle_span in state.span.span_cycles() {
                    let cycle_n = cycle_span.begin.to_float().floor() as i64;
                    let cycle_span_local = TimeSpan::new(
                        cycle_span.begin.sub(&Fraction::from(cycle_n)),
                        cycle_span.end.sub(&Fraction::from(cycle_n)),
                    );

                    let seg_begin = begin.add(&Fraction::from(cycle_n));
                    let seg_end = end.add(&Fraction::from(cycle_n));
                    let seg_span = TimeSpan::new(seg_begin, seg_end);

                    if let Some(intersect) = cycle_span_local.intersection(&seg_span) {
                        let query_span = TimeSpan::new(
                            intersect.begin.sub(&begin).mul(&Fraction::from(n as i64)),
                            intersect.end.sub(&begin).mul(&Fraction::from(n as i64)),
                        );

                        for hap in pat.query(&state.set_span(query_span)) {
                            let new_whole = hap.whole.as_ref().map(|w| {
                                TimeSpan::new(
                                    w.begin.div(&Fraction::from(n as i64)).add(&begin),
                                    w.end.div(&Fraction::from(n as i64)).add(&begin),
                                )
                            });
                            let new_part = TimeSpan::new(
                                hap.part.begin.div(&Fraction::from(n as i64)).add(&begin),
                                hap.part.end.div(&Fraction::from(n as i64)).add(&begin),
                            );
                            haps.push(Hap {
                                whole: new_whole,
                                part: new_part,
                                value: hap.value.clone(),
                                context: hap.context.clone(),
                            });
                        }
                    }
                }
                haps
            })
            .collect()
    })
}

/// Sequence patterns (alias for fastcat)
pub fn sequence<T: Clone + 'static>(patterns: Vec<Pattern<T>>) -> Pattern<T> {
    fastcat(patterns)
}

/// Slow concatenation
pub fn slowcat<T: Clone + 'static>(patterns: Vec<Pattern<T>>) -> Pattern<T> {
    let len = patterns.len();
    fastcat(patterns).slow(len as f64)
}

// ============================================================================
// Helper functions
// ============================================================================

/// Convert a value to a pattern (reification)
pub trait IntoPattern<T> {
    fn into_pattern(self) -> Pattern<T>;
}

impl<T: Clone + 'static> IntoPattern<T> for T {
    fn into_pattern(self) -> Pattern<T> {
        Pattern::pure(self)
    }
}

impl<T: Clone + 'static> IntoPattern<T> for Pattern<T> {
    fn into_pattern(self) -> Pattern<T> {
        self
    }
}

// ============================================================================
// Display/Debug implementations
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fraction() {
        let a = Fraction::new(1, 2);
        let b = Fraction::new(1, 4);
        let sum = a.add(&b);
        assert_eq!(sum, Fraction::new(3, 4));
    }

    #[test]
    fn test_pure_pattern() {
        let pat = Pattern::pure(42);
        let haps = pat.query_arc(Fraction::from(0), Fraction::from(1));
        assert_eq!(haps.len(), 1);
        assert_eq!(haps[0].value, 42);
    }

    #[test]
    fn test_fmap() {
        let pat = Pattern::pure(10);
        let mapped = pat.fmap(|x| x * 2);
        let haps = mapped.query_arc(Fraction::from(0), Fraction::from(1));
        assert_eq!(haps[0].value, 20);
    }

    #[test]
    fn test_fast() {
        let pat = Pattern::pure(1);
        let fast_pat = pat.fast(2.0);
        let haps = fast_pat.query_arc(Fraction::from(0), Fraction::from(1));
        // Fast(2) should give us events happening twice as fast
        assert!(!haps.is_empty());
    }

    #[test]
    fn test_stack() {
        let pat1 = Pattern::pure(1);
        let pat2 = Pattern::pure(2);
        let stacked = stack(vec![pat1, pat2]);
        let haps = stacked.query_arc(Fraction::from(0), Fraction::from(1));
        assert_eq!(haps.len(), 2);
    }

    #[test]
    fn test_sequence() {
        let pat1 = Pattern::pure(1);
        let pat2 = Pattern::pure(2);
        let pat3 = Pattern::pure(3);
        let seq = sequence(vec![pat1, pat2, pat3]);
        let haps = seq.query_arc(Fraction::from(0), Fraction::from(1));
        assert_eq!(haps.len(), 3);
    }
}
