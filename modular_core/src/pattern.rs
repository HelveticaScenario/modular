use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::iter::Peekable;
use std::str::Chars;

/// A span expressed in cycles (1.0 cycle is the canonical Strudel unit).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Span {
    pub start: f32,
    pub duration: f32,
}

impl Span {
    pub fn end(&self) -> f32 {
        self.start + self.duration
    }

    fn overlaps(&self, range: &TimeRange) -> bool {
        self.start < range.end && self.end() > range.start
    }
}

/// Inclusive time window in cycles used for querying.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TimeRange {
    pub start: f32,
    pub end: f32,
}

impl TimeRange {
    pub fn new(start: f32, end: f32) -> Self {
        if end < start {
            TimeRange { start: end, end: start }
        } else {
            TimeRange { start, end }
        }
    }
}

/// A Strudel-style hap: value + span in cycle space.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Hap {
    pub span: Span,
    pub value: PatternValue,
}

/// Values supported by a pattern. Kept intentionally small so patterns stay serializable.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PatternValue {
    Number { value: f64 },
    Text { value: String },
    Bool { value: bool },
}

impl PatternValue {
    pub fn number(value: f64) -> Self {
        PatternValue::Number { value }
    }

    pub fn text(value: impl Into<String>) -> Self {
        PatternValue::Text {
            value: value.into(),
        }
    }

    pub fn boolean(value: bool) -> Self {
        PatternValue::Bool { value }
    }

    fn as_f64(&self) -> Option<f64> {
        match self {
            PatternValue::Number { value } => Some(*value),
            _ => None,
        }
    }

    fn map_number(&self, func: impl FnOnce(f64) -> f64) -> PatternValue {
        match self {
            PatternValue::Number { value } => PatternValue::Number { value: func(*value) },
            PatternValue::Text { .. } => self.clone(),
            PatternValue::Bool { .. } => self.clone(),
        }
    }
}

/// Declarative value transformations that replace callback-style APIs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ValueOp {
    Identity,
    Replace { value: PatternValue },
    Add { amount: f64 },
    Subtract { amount: f64 },
    Multiply { factor: f64 },
    Divide { divisor: f64 },
    Power { exponent: f64 },
    Clamp { min: f64, max: f64 },
    Range { min: f64, max: f64 },
    Negate,
    Round,
    Floor,
    Ceil,
}

impl ValueOp {
    fn apply(&self, value: &PatternValue) -> PatternValue {
        match self {
            ValueOp::Identity => value.clone(),
            ValueOp::Replace { value: replacement } => replacement.clone(),
            ValueOp::Add { amount } => value.map_number(|v| v + amount),
            ValueOp::Subtract { amount } => value.map_number(|v| v - amount),
            ValueOp::Multiply { factor } => value.map_number(|v| v * factor),
            ValueOp::Divide { divisor } => {
                if *divisor == 0.0 {
                    value.clone()
                } else {
                    value.map_number(|v| v / divisor)
                }
            }
            ValueOp::Power { exponent } => value.map_number(|v| v.powf(*exponent)),
            ValueOp::Clamp { min, max } => value.map_number(|v| v.clamp(*min, *max)),
            ValueOp::Range { min, max } => value.map_number(|v| {
                let clamped = v.clamp(0.0, 1.0);
                min + (max - min) * clamped
            }),
            ValueOp::Negate => value.map_number(|v| -v),
            ValueOp::Round => value.map_number(|v| v.round()),
            ValueOp::Floor => value.map_number(|v| v.floor()),
            ValueOp::Ceil => value.map_number(|v| v.ceil()),
        }
    }
}

/// Serializable predicates that can be evaluated without user-supplied callbacks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Condition {
    Always,
    Equals { value: PatternValue },
    NumberGreaterThan { threshold: f64 },
    NumberLessThan { threshold: f64 },
    Between { min: f64, max: f64, inclusive: bool },
    EventIndexMultiple { step: u32 },
}

impl Condition {
    fn matches(&self, value: &PatternValue, event_index: usize) -> bool {
        match self {
            Condition::Always => true,
            Condition::Equals { value: expected } => value == expected,
            Condition::NumberGreaterThan { threshold } => {
                value.as_f64().map(|v| v > *threshold).unwrap_or(false)
            }
            Condition::NumberLessThan { threshold } => {
                value.as_f64().map(|v| v < *threshold).unwrap_or(false)
            }
            Condition::Between {
                min,
                max,
                inclusive,
            } => value.as_f64().map(|v| {
                if *inclusive {
                    v >= *min && v <= *max
                } else {
                    v > *min && v < *max
                }
            }).unwrap_or(false),
            Condition::EventIndexMultiple { step } => {
                if *step == 0 {
                    false
                } else {
                    event_index % *step as usize == 0
                }
            }
        }
    }
}

/// Declarative, serializable transformations that can be applied to a pattern.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PatternTransform {
    Fast { factor: f32 },
    Slow { factor: f32 },
    Offset { offset: f32 },
    Repeat { times: u32 },
    Map { op: ValueOp },
    Filter { condition: Condition },
    Reverse,
}

impl PatternTransform {
    pub fn apply(&self, pattern: Pattern) -> Pattern {
        match self {
            PatternTransform::Fast { factor } => pattern.fast(*factor),
            PatternTransform::Slow { factor } => pattern.slow(*factor),
            PatternTransform::Offset { offset } => pattern.offset(*offset),
            PatternTransform::Repeat { times } => pattern.repeat(*times),
            PatternTransform::Map { op } => pattern.map(op.clone()),
            PatternTransform::Filter { condition } => pattern.filter(condition.clone()),
            PatternTransform::Reverse => pattern.reverse(),
        }
    }
}

/// Identifies a node in the pattern tree by its path (child indices from root).
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct NodeId(Vec<usize>);

impl NodeId {
    fn from_path(path: &[usize]) -> Self {
        NodeId(path.to_vec())
    }
}

/// Stateful playhead storage for sequences so live edits can reuse phase.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PatternState {
    seq_positions: HashMap<NodeId, usize>,
    phase: f32,
    cycle: i64,
    cache_cycle: Option<i64>,
    cache_haps: Vec<Hap>,
}

impl PatternState {
    pub fn new() -> Self {
        Self {
            seq_positions: HashMap::new(),
            phase: 0.0,
            cycle: 0,
            cache_cycle: None,
            cache_haps: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.seq_positions.clear();
        self.phase = 0.0;
        self.cycle = 0;
        self.cache_cycle = None;
        self.cache_haps.clear();
    }
}

/// AST describing a pattern within a canonical cycle. Each variant is serializable and contains no runtime callbacks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum PatternExpr {
    Empty,
    Event { value: PatternValue, steps: u32 },
    Sequence { patterns: Vec<Pattern> },
    Stack { patterns: Vec<Pattern> },
    Repeat { times: u32, pattern: Box<Pattern> },
    Stretch { factor: f32, pattern: Box<Pattern> },
    Offset { offset: f32, pattern: Box<Pattern> },
    Map { op: ValueOp, pattern: Box<Pattern> },
    Filter { condition: Condition, pattern: Box<Pattern> },
    Reverse { pattern: Box<Pattern> },
}

impl PatternExpr {
    fn render_with_state(
        &self,
        path: &mut Vec<usize>,
        start: f32,
        duration: f32,
        output: &mut Vec<Hap>,
        mut event_index: usize,
        state: &mut PatternState,
    ) -> usize {
        match self {
            PatternExpr::Empty => event_index,
            PatternExpr::Event { value, steps } => {
                let step_count = (*steps).max(1);
                let step_duration = if step_count == 0 {
                    0.0
                } else {
                    duration / step_count as f32
                };
                for i in 0..step_count {
                    output.push(Hap {
                        span: Span {
                            start: start + step_duration * i as f32,
                            duration: step_duration,
                        },
                        value: value.clone(),
                    });
                    event_index += 1;
                }
                event_index
            }
            PatternExpr::Sequence { patterns } => {
                if patterns.is_empty() {
                    return event_index;
                }
                let part_duration = duration / patterns.len() as f32;
                let key = NodeId::from_path(path);
                let current_pos = *state.seq_positions.get(&key).unwrap_or(&0);
                let start_index = current_pos % patterns.len();
                let mut idx = event_index;
                for offset in 0..patterns.len() {
                    let child_index = (start_index + offset) % patterns.len();
                    path.push(child_index);
                    idx = patterns[child_index].root.render_with_state(
                        path,
                        start + part_duration * offset as f32,
                        part_duration,
                        output,
                        idx,
                        state,
                    );
                    path.pop();
                }
                state
                    .seq_positions
                    .insert(key, current_pos.wrapping_add(1));
                idx
            }
            PatternExpr::Stack { patterns } => {
                let mut idx = event_index;
                for (child_index, pat) in patterns.iter().enumerate() {
                    path.push(child_index);
                    idx = pat
                        .root
                        .render_with_state(path, start, duration, output, idx, state);
                    path.pop();
                }
                idx
            }
            PatternExpr::Repeat { times, pattern } => {
                if *times == 0 {
                    return event_index;
                }
                let slice_duration = duration / *times as f32;
                let mut idx = event_index;
                for i in 0..*times {
                    path.push(i as usize);
                    idx = pattern.root.render_with_state(
                        path,
                        start + slice_duration * i as f32,
                        slice_duration,
                        output,
                        idx,
                        state,
                    );
                    path.pop();
                }
                idx
            }
            PatternExpr::Stretch { factor, pattern } => {
                if *factor <= 0.0 {
                    return event_index;
                }
                let scaled_duration = duration / *factor;
                path.push(0);
                let next = pattern.root.render_with_state(
                    path,
                    start,
                    scaled_duration,
                    output,
                    event_index,
                    state,
                );
                path.pop();
                next
            }
            PatternExpr::Offset { offset, pattern } => {
                path.push(0);
                let next = pattern.root.render_with_state(
                    path,
                    start + offset * duration,
                    duration,
                    output,
                    event_index,
                    state,
                );
                path.pop();
                next
            }
            PatternExpr::Map { op, pattern } => {
                let mut temp = Vec::new();
                path.push(0);
                let next_index = pattern.root.render_with_state(
                    path,
                    start,
                    duration,
                    &mut temp,
                    event_index,
                    state,
                );
                path.pop();
                for hap in temp {
                    output.push(Hap {
                        value: op.apply(&hap.value),
                        span: hap.span,
                    });
                }
                next_index
            }
            PatternExpr::Filter {
                condition,
                pattern,
            } => {
                let mut temp = Vec::new();
                path.push(0);
                let next_index = pattern.root.render_with_state(
                    path,
                    start,
                    duration,
                    &mut temp,
                    event_index,
                    state,
                );
                path.pop();
                for (offset, hap) in temp.into_iter().enumerate() {
                    if condition.matches(&hap.value, event_index + offset) {
                        output.push(hap);
                    }
                }
                next_index
            }
            PatternExpr::Reverse { pattern } => {
                let mut temp = Vec::new();
                path.push(0);
                let next_index = pattern.root.render_with_state(
                    path,
                    start,
                    duration,
                    &mut temp,
                    event_index,
                    state,
                );
                path.pop();
                for mut hap in temp.into_iter().rev() {
                    let rel_start = hap.span.start - start;
                    let new_start = start + (duration - hap.span.duration - rel_start);
                    hap.span.start = new_start;
                    output.push(hap);
                }
                next_index
            }
        }
    }
}

/// Public entry point for building declarative, serializable patterns.
///
/// Patterns are defined over a single canonical cycle (0.0..1.0) and are considered
/// to repeat every cycle. Queries operate in cycle space so they can be driven at
/// sample rate without callbacks.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pattern {
    pub root: PatternExpr,
}

impl Pattern {
    pub fn pure(value: PatternValue) -> Self {
        Pattern {
            root: PatternExpr::Event { value, steps: 1 },
        }
    }

    pub fn with_steps(value: PatternValue, steps: u32) -> Self {
        Pattern {
            root: PatternExpr::Event {
                value,
                steps: steps.max(1),
            },
        }
    }

    pub fn silence() -> Self {
        Pattern {
            root: PatternExpr::Empty,
        }
    }

    pub fn sequence<I>(patterns: I) -> Self
    where
        I: IntoIterator<Item = Pattern>,
    {
        Pattern {
            root: PatternExpr::Sequence {
                patterns: patterns.into_iter().collect(),
            },
        }
    }

    pub fn stack<I>(patterns: I) -> Self
    where
        I: IntoIterator<Item = Pattern>,
    {
        Pattern {
            root: PatternExpr::Stack {
                patterns: patterns.into_iter().collect(),
            },
        }
    }

    pub fn repeat(self, times: u32) -> Self {
        Pattern {
            root: PatternExpr::Repeat {
                times,
                pattern: Box::new(self),
            },
        }
    }

    pub fn fast(self, factor: f32) -> Self {
        Pattern {
            root: PatternExpr::Stretch {
                factor,
                pattern: Box::new(self),
            },
        }
    }

    pub fn slow(self, factor: f32) -> Self {
        if factor == 0.0 {
            self
        } else {
            Pattern {
                root: PatternExpr::Stretch {
                    factor: 1.0 / factor,
                    pattern: Box::new(self),
                },
            }
        }
    }

    pub fn offset(self, offset: f32) -> Self {
        Pattern {
            root: PatternExpr::Offset {
                offset,
                pattern: Box::new(self),
            },
        }
    }

    pub fn map(self, op: ValueOp) -> Self {
        Pattern {
            root: PatternExpr::Map {
                op,
                pattern: Box::new(self),
            },
        }
    }

    pub fn filter(self, condition: Condition) -> Self {
        Pattern {
            root: PatternExpr::Filter {
                condition,
                pattern: Box::new(self),
            },
        }
    }

    pub fn reverse(self) -> Self {
        Pattern {
            root: PatternExpr::Reverse {
                pattern: Box::new(self),
            },
        }
    }

    pub fn apply(self, transform: PatternTransform) -> Self {
        transform.apply(self)
    }

    /// Advance the pattern by one audio sample, given cycles-per-second and sample rate.
    /// Returns haps active during this tick and whether a cycle boundary was crossed.
    pub fn tick(&self, state: &mut PatternState, cps: f32, sample_rate: f32) -> TickResult {
        let delta_cycles = if sample_rate <= 0.0 { 0.0 } else { cps / sample_rate };

        let start_cycle = state.cycle;
        let start_time = state.cycle as f32 + state.phase;
        let end_phase = state.phase + delta_cycles;

        let mut advanced_cycle = false;
        let mut haps = Vec::new();

        // Ensure we have cache for current cycle
        ensure_cycle_cached(self, start_cycle, state);

        if end_phase < 1.0 {
            state.phase = end_phase;
            collect_overlapping(&state.cache_haps, start_time, start_time + delta_cycles, &mut haps);
        } else {
            // consume tail of current cycle
            let end_current = start_cycle as f32 + 1.0;
            collect_overlapping(&state.cache_haps, start_time, end_current, &mut haps);

            // advance cycles (could be more than one if cps is huge)
            let mut remaining = end_phase - 1.0;
            state.cycle += 1;
            advanced_cycle = true;

            // handle possible multiple wraps
            while remaining >= 1.0 {
                state.phase = 0.0;
                ensure_cycle_cached(self, state.cycle, state);
                collect_overlapping(&state.cache_haps, state.cycle as f32, state.cycle as f32 + 1.0, &mut haps);
                state.cycle += 1;
                remaining -= 1.0;
            }

            state.phase = remaining;
            ensure_cycle_cached(self, state.cycle, state);
            let segment_start = state.cycle as f32;
            collect_overlapping(&state.cache_haps, segment_start, segment_start + remaining, &mut haps);
        }

        TickResult {
            haps,
            advanced_cycle,
        }
    }

    /// Render the pattern for a specific cycle with supplied state (state is updated).
    pub fn query_cycle_with_state(&self, cycle: i64, state: &mut PatternState) -> Vec<Hap> {
        let mut output = Vec::new();
        let mut path = Vec::new();
        self.root
            .render_with_state(&mut path, cycle as f32, 1.0, &mut output, 0, state);
        output
    }

    /// Render the pattern for a specific cycle with ephemeral state.
    pub fn query_cycle(&self, cycle: i64) -> Vec<Hap> {
        let mut state = PatternState::new();
        self.query_cycle_with_state(cycle, &mut state)
    }

    /// Query all haps overlapping the given time range (in cycles) with supplied state.
    pub fn query_range_with_state(&self, range: TimeRange, state: &mut PatternState) -> Vec<Hap> {
        if range.end <= range.start {
            return Vec::new();
        }

        let mut result = Vec::new();
        let mut event_index = 0usize;
        let start_cycle = range.start.floor() as i64;
        let end_cycle = range.end.ceil() as i64;
        let mut path = Vec::new();

        for cycle in start_cycle..end_cycle {
            let mut cycle_events = Vec::new();
            event_index = self.root.render_with_state(
                &mut path,
                cycle as f32,
                1.0,
                &mut cycle_events,
                event_index,
                state,
            );

            for hap in cycle_events.into_iter() {
                if hap.span.overlaps(&range) {
                    result.push(hap);
                }
            }
        }

        result
    }

    /// Query all haps overlapping the given time range (in cycles) with a temporary state.
    pub fn query_range(&self, range: TimeRange) -> Vec<Hap> {
        let mut state = PatternState::new();
        self.query_range_with_state(range, &mut state)
    }

    /// Return the first hap covering the given time (in cycles) using supplied state.
    pub fn hap_at_with_state(&self, time: f32, state: &mut PatternState) -> Option<Hap> {
        let range = TimeRange::new(time, time + f32::EPSILON.max(1e-6));
        self.query_range_with_state(range, state)
            .into_iter()
            .find(|hap| time >= hap.span.start && time < hap.span.end())
    }

    /// Return the first hap covering the given time (in cycles) with a temporary state.
    pub fn hap_at(&self, time: f32) -> Option<Hap> {
        let mut state = PatternState::new();
        self.hap_at_with_state(time, &mut state)
    }
}

/// Errors that can occur while parsing mini notation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MiniError {
    pub message: String,
}

impl MiniError {
    fn new(msg: impl Into<String>) -> Self {
        MiniError {
            message: msg.into(),
        }
    }
}

/// Parse a limited subset of Strudel/Tidal mini-notation into a declarative `Pattern`.
/// Supported constructs:
/// - Whitespace-separated tokens form a sequence over one cycle.
/// - `token*4` speeds the token up 4x within the same cycle.
/// - `~` represents a gap (silence) for one slice of the sequence.
/// - `[ a b ]` stacks patterns in parallel over the same cycle window.
/// - `< a b c >` sequences its contents within the same slice.
/// Numbers parse to `PatternValue::Number`; other atoms become `PatternValue::Text`.
/// This intentionally avoids callbacks so the resulting pattern is serializable.
pub fn parse_mini(input: &str) -> Result<Pattern, MiniError> {
    let mut chars = input.chars().peekable();
    let pat = parse_sequence(&mut chars, Terminator::Eof)?;
    Ok(pat)
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Terminator {
    Eof,
    CloseSquare,
    CloseAngle,
}

fn parse_sequence(chars: &mut Peekable<Chars<'_>>, terminator: Terminator) -> Result<Pattern, MiniError> {
    let mut parts: Vec<Pattern> = Vec::new();
    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
            continue;
        }
        match (terminator, ch) {
            (Terminator::CloseSquare, ']') => {
                chars.next();
                break;
            }
            (Terminator::CloseAngle, '>') => {
                chars.next();
                break;
            }
            _ => {
                let part = parse_part(chars)?;
                parts.push(part);
            }
        }
    }

    if parts.is_empty() {
        Ok(Pattern::silence())
    } else if parts.len() == 1 {
        Ok(parts.remove(0))
    } else {
        Ok(Pattern::sequence(parts))
    }
}

fn parse_part(chars: &mut Peekable<Chars<'_>>) -> Result<Pattern, MiniError> {
    let mut pat = parse_atom(chars)?;

    // Optional repeat/speed syntax: *N
    if let Some('*') = chars.peek().copied() {
        chars.next();
        let number = parse_number_literal(chars)?;
        if number <= 0.0 {
            return Err(MiniError::new("repeat factor must be positive"));
        }
        pat = pat.fast(number as f32);
    }

    Ok(pat)
}

fn parse_atom(chars: &mut Peekable<Chars<'_>>) -> Result<Pattern, MiniError> {
    skip_ws(chars);
    let ch = chars.next().ok_or_else(|| MiniError::new("unexpected end of input"))?;
    match ch {
        '[' => {
            let inner = parse_sequence(chars, Terminator::CloseSquare)?;
            Ok(match inner {
                Pattern { root: PatternExpr::Sequence { patterns } } => Pattern::stack(patterns),
                other => Pattern::stack([other]),
            })
        }
        '<' => parse_sequence(chars, Terminator::CloseAngle),
        '~' => Ok(Pattern::silence()),
        '>' => Err(MiniError::new("unexpected '>'")),
        ']' => Err(MiniError::new("unexpected ']'")),
        _ => {
            let mut buf = String::new();
            buf.push(ch);
            while let Some(&next) = chars.peek() {
                if next.is_whitespace() || matches!(next, '[' | ']' | '<' | '>' | '*') {
                    break;
                }
                buf.push(next);
                chars.next();
            }

            if buf.is_empty() {
                return Err(MiniError::new("empty token"));
            }

            let value = if let Ok(num) = buf.parse::<f64>() {
                PatternValue::number(num)
            } else {
                PatternValue::text(buf)
            };
            Ok(Pattern::pure(value))
        }
    }
}

fn parse_number_literal(chars: &mut Peekable<Chars<'_>>) -> Result<f64, MiniError> {
    skip_ws(chars);
    let mut buf = String::new();
    while let Some(&ch) = chars.peek() {
        if ch.is_ascii_digit() || ch == '.' {
            buf.push(ch);
            chars.next();
        } else {
            break;
        }
    }
    if buf.is_empty() {
        return Err(MiniError::new("expected number after '*'"));
    }
    buf.parse::<f64>()
        .map_err(|_| MiniError::new("invalid number literal"))
}

fn skip_ws(chars: &mut Peekable<Chars<'_>>) {
    while matches!(chars.peek(), Some(ch) if ch.is_whitespace()) {
        chars.next();
    }
}

/// Result of advancing a pattern by one audio sample.
#[derive(Clone, Debug, PartialEq)]
pub struct TickResult {
    /// Haps active during this sample window.
    pub haps: Vec<Hap>,
    /// True if we wrapped to a new cycle on this tick.
    pub advanced_cycle: bool,
}

fn ensure_cycle_cached(pattern: &Pattern, cycle: i64, state: &mut PatternState) {
    if state.cache_cycle == Some(cycle) {
        return;
    }

    let mut path = Vec::new();
    let mut output = Vec::new();
    pattern
        .root
        .render_with_state(&mut path, cycle as f32, 1.0, &mut output, 0, state);
    state.cache_cycle = Some(cycle);
    state.cache_haps = output;
}

fn collect_overlapping(haps: &[Hap], start: f32, end: f32, out: &mut Vec<Hap>) {
    for hap in haps {
        if hap.span.start < end && hap.span.end() > start {
            out.push(hap.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn num(v: f64) -> PatternValue {
        PatternValue::number(v)
    }

    #[test]
    fn repeat_splits_time_equally() {
        let pattern = Pattern::pure(num(1.0)).repeat(2);
        let haps = pattern.query_cycle(0);
        assert_eq!(haps.len(), 2);
        assert!((haps[0].span.start - 0.0).abs() < 1e-6);
        assert!((haps[0].span.duration - 0.5).abs() < 1e-6);
        assert!((haps[1].span.start - 0.5).abs() < 1e-6);
    }

    #[test]
    fn stack_overlays_events() {
        let base = Pattern::pure(num(1.0));
        let late = Pattern::pure(num(2.0)).offset(0.25);
        let pattern = Pattern::stack([base, late]);
        let haps = pattern.query_cycle(0);
        assert_eq!(haps.len(), 2);
        let starts: Vec<f32> = haps.iter().map(|h| h.span.start).collect();
        assert!(starts.contains(&0.0));
        assert!(starts.contains(&0.25));
    }

    #[test]
    fn map_adds_amount() {
        let pattern = Pattern::pure(num(1.5)).map(ValueOp::Add { amount: 0.5 });
        let haps = pattern.query_cycle(0);
        assert_eq!(haps.len(), 1);
        match &haps[0].value {
            PatternValue::Number { value } => assert!((*value - 2.0).abs() < 1e-6),
            _ => panic!("expected number"),
        }
    }

    #[test]
    fn filter_keeps_every_other_event() {
        let pattern = Pattern::with_steps(num(1.0), 4)
            .filter(Condition::EventIndexMultiple { step: 2 });
        let haps = pattern.query_cycle(0);
        assert_eq!(haps.len(), 2);
        assert!((haps[0].span.start - 0.0).abs() < 1e-6);
        assert!((haps[1].span.start - 0.5).abs() < 1e-6);
    }

    #[test]
    fn hap_at_returns_covering_event() {
        let pattern = Pattern::pure(num(1.0)).fast(2.0);
        let hap = pattern.hap_at(0.25).expect("should find hap");
        match hap.value {
            PatternValue::Number { value } => assert!((value - 1.0).abs() < 1e-6),
            _ => panic!("expected number"),
        }
        assert!((hap.span.duration - 0.5).abs() < 1e-6);
    }

    #[test]
    fn range_query_spans_multiple_cycles() {
        let pattern = Pattern::pure(num(1.0));
        let haps = pattern.query_range(TimeRange::new(0.0, 2.0));
        assert_eq!(haps.len(), 2);
        let starts: Vec<f32> = haps.iter().map(|h| h.span.start).collect();
        assert!(starts.contains(&0.0));
        assert!(starts.contains(&1.0));
    }

    #[test]
    fn serde_roundtrip() {
        let pattern = Pattern::sequence([
            Pattern::pure(num(1.0)),
            Pattern::pure(num(2.0)).fast(2.0),
        ])
        .map(ValueOp::Negate)
        .apply(PatternTransform::Reverse);

        let json = serde_json::to_string(&pattern).unwrap();
        let roundtrip: Pattern = serde_json::from_str(&json).unwrap();
        assert_eq!(pattern, roundtrip);
    }

    #[test]
    fn parse_simple_sequence() {
        let pattern = parse_mini("1 2").unwrap();
        let haps = pattern.query_cycle(0);
        assert_eq!(haps.len(), 2);
        assert!((haps[0].span.start - 0.0).abs() < 1e-6);
        assert!((haps[1].span.start - 0.5).abs() < 1e-6);
    }

    #[test]
    fn parse_stack_and_repeat() {
        let pattern = parse_mini("[a b]*2").unwrap();
        let haps = pattern.query_cycle(0);
        assert_eq!(haps.len(), 2);
        let starts: Vec<f32> = haps.iter().map(|h| h.span.start).collect();
        assert!(starts.iter().any(|s| (*s - 0.0).abs() < 1e-6));
        // fast(2) compresses into first half of the cycle
        assert!(starts.iter().all(|s| *s < 0.5 + 1e-6));
    }

    #[test]
    fn parse_nested_angle_and_gap() {
        let pattern = parse_mini("<1 ~ 2>").unwrap();
        let haps = pattern.query_cycle(0);
        assert_eq!(haps.len(), 2);
        assert!((haps[0].span.start - 0.0).abs() < 1e-6);
        assert!((haps[1].span.start - (2.0 / 3.0)).abs() < 1e-6);
    }

    #[test]
    fn parse_numbers_and_text() {
        let pattern = parse_mini("440 hz").unwrap();
        let haps = pattern.query_cycle(0);
        assert_eq!(haps.len(), 2);
        match haps[0].value {
            PatternValue::Number { value } => assert!((value - 440.0).abs() < 1e-6),
            _ => panic!("expected number"),
        }
        match &haps[1].value {
            PatternValue::Text { value } => assert_eq!(value, "hz"),
            _ => panic!("expected text"),
        }
    }

    #[test]
    fn tick_advances_phase_and_reports_cycle_wrap() {
        let pattern = parse_mini("1 2").unwrap();
        let mut state = PatternState::new();

        // 1 cycle per second at 2 samples/sec: each tick advances half a cycle
        let first = pattern.tick(&mut state, 1.0, 2.0);
        assert!(!first.advanced_cycle);
        assert_eq!(first.haps.len(), 1);

        let second = pattern.tick(&mut state, 1.0, 2.0);
        assert!(second.advanced_cycle); // wrapped into next cycle
        assert_eq!(second.haps.len(), 1);
    }

    #[test]
    fn sequence_advances_across_cycles_with_state() {
        let pattern = parse_mini("1 2 3").unwrap();
        let mut state = PatternState::new();

        let first = pattern.query_cycle_with_state(0, &mut state);
        let second = pattern.query_cycle_with_state(1, &mut state);

        let first_vals: Vec<_> = first
            .iter()
            .map(|h| match h.value { PatternValue::Number { value } => value as i32, _ => -1 })
            .collect();
        let second_vals: Vec<_> = second
            .iter()
            .map(|h| match h.value { PatternValue::Number { value } => value as i32, _ => -1 })
            .collect();

        assert_eq!(first_vals, vec![1, 2, 3]);
        assert_eq!(second_vals, vec![2, 3, 1]);
    }

    #[test]
    fn sequence_state_survives_inner_edit() {
        let original = parse_mini("[a b c]").unwrap();
        let mut state = PatternState::new();
        let first = original.query_cycle_with_state(0, &mut state);
        let starts_first: Vec<_> = first.iter().map(|h| match &h.value {
            PatternValue::Text { value } => value.clone(),
            _ => "".into(),
        }).collect();
        assert_eq!(starts_first, vec!["a", "b", "c"]);

        // Edit middle element, keep structure so node path remains; playhead should continue.
        let edited = parse_mini("[a x c]").unwrap();
        let second = edited.query_cycle_with_state(1, &mut state);
        let starts_second: Vec<_> = second.iter().map(|h| match &h.value {
            PatternValue::Text { value } => value.clone(),
            _ => "".into(),
        }).collect();

        // Stack keeps overlay order; editing a child preserves paths; contents update
        assert_eq!(starts_second, vec!["a", "x", "c"]);
    }
}
