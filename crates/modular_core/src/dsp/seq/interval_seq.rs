//! IntervalSeq module - A scale-degree sequencer with two additive patterns.
//!
//! This module sequences scale degrees using two mini notation patterns:
//! - `interval_pattern`: Primary scale degree pattern
//! - `add_pattern`: Offset pattern added to interval_pattern
//!
//! The sum of both patterns is treated as a scale degree index, quantized to
//! the configured scale with octave wrapping.
//!
//! The sequencer outputs:
//! - CV: V/Oct pitch (quantized to scale)
//! - Gate: High while note is active
//! - Trig: Short pulse at note onset

use std::cmp::Ordering;

use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    Patch, PolySignal,
    dsp::utilities::quantizer::ScaleParam,
    dsp::utils::{TempGate, TempGateState, midi_to_voct_f64},
    pattern_system::{DspHap, Fraction, Pattern},
    poly::{PORT_MAX_CHANNELS, PolyOutput},
    types::Connect,
};

/// Scale parameter for IntervalSeq that supports an optional octave in the root.
///
/// Wraps [`ScaleParam`] but deserializes via `parse_with_octave`, accepting
/// syntax like `"C3(major)"` or `"Db3(min)"` in addition to `"C(major)"`.
#[derive(Clone, Debug)]
struct IntervalScaleParam(ScaleParam);

impl Default for IntervalScaleParam {
    fn default() -> Self {
        Self(ScaleParam::parse_with_octave("C(major)").unwrap())
    }
}

impl<'de> serde::Deserialize<'de> for IntervalScaleParam {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let source = String::deserialize(deserializer)?;
        ScaleParam::parse_with_octave(&source)
            .map(Self)
            .ok_or_else(|| {
                serde::de::Error::custom(format!("Invalid scale specification: {}", source))
            })
    }
}

impl schemars::JsonSchema for IntervalScaleParam {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("IntervalScaleParam")
    }
    fn json_schema(_gen: &mut schemars::SchemaGenerator) -> schemars::Schema {
        String::json_schema(_gen)
    }
}

impl Connect for IntervalScaleParam {
    fn connect(&mut self, _patch: &Patch) {}
}

impl std::ops::Deref for IntervalScaleParam {
    type Target = ScaleParam;
    fn deref(&self) -> &ScaleParam {
        &self.0
    }
}

/// Value type for interval patterns: either a degree or rest.
#[derive(Clone, Debug)]
pub enum IntervalValue {
    /// Scale degree (can be negative for downward movement)
    Degree(i32),
    /// Rest - no output, gate low
    Rest,
}

impl IntervalValue {
    pub fn is_rest(&self) -> bool {
        matches!(self, IntervalValue::Rest)
    }

    pub fn degree(&self) -> Option<i32> {
        match self {
            IntervalValue::Degree(d) => Some(*d),
            IntervalValue::Rest => None,
        }
    }
}

impl crate::pattern_system::mini::convert::FromMiniAtom for IntervalValue {
    fn from_atom(
        atom: &crate::pattern_system::mini::ast::AtomValue,
    ) -> Result<Self, crate::pattern_system::mini::convert::ConvertError> {
        match atom {
            crate::pattern_system::mini::ast::AtomValue::Number(n) => {
                Ok(IntervalValue::Degree(*n as i32))
            }
            crate::pattern_system::mini::ast::AtomValue::Midi(m) => {
                Ok(IntervalValue::Degree(*m as i32))
            }
            _ => Err(
                crate::pattern_system::mini::convert::ConvertError::InvalidAtom(
                    "IntervalValue only supports integers".to_string(),
                ),
            ),
        }
    }

    fn from_list(
        atoms: &[crate::pattern_system::mini::ast::AtomValue],
    ) -> Result<Self, crate::pattern_system::mini::convert::ConvertError> {
        if atoms.len() == 1 {
            Self::from_atom(&atoms[0])
        } else {
            Err(crate::pattern_system::mini::convert::ConvertError::ListNotSupported)
        }
    }

    fn combine_with_head(
        _head_atoms: &[crate::pattern_system::mini::ast::AtomValue],
        _tail: &Self,
    ) -> Result<Self, crate::pattern_system::mini::convert::ConvertError> {
        Err(crate::pattern_system::mini::convert::ConvertError::ListNotSupported)
    }

    fn rest_value() -> Option<Self> {
        Some(IntervalValue::Rest)
    }

    fn supports_rest() -> bool {
        true
    }
}

impl crate::pattern_system::mini::convert::HasRest for IntervalValue {
    fn rest_value() -> Self {
        IntervalValue::Rest
    }
}

/// A pattern parameter for interval/degree patterns.
///
/// This struct is serialized as a simple string but contains the parsed pattern.
#[derive(Default, JsonSchema, Debug)]
#[serde(transparent)]
#[schemars(transparent)]
pub struct IntervalPatternParam {
    /// The source pattern string
    #[allow(dead_code)]
    source: String,

    /// The parsed pattern
    #[serde(skip, default)]
    #[schemars(skip)]
    pub(crate) pattern: Option<Pattern<IntervalValue>>,

    /// All leaf spans in the pattern (character offsets within the pattern string).
    /// Computed once at parse time for creating Monaco tracked decorations.
    ///
    /// These differ from the "spans" returned in module state:
    /// - `all_spans`: All pattern leaves, used to create decorations that track edits
    /// - `spans` (in get_state): Currently active/playing spans, used for highlighting
    #[serde(skip, default)]
    #[schemars(skip)]
    pub(crate) all_spans: Vec<(usize, usize)>,
}

impl IntervalPatternParam {
    /// Parse a pattern string.
    fn parse(source: &str) -> Result<Self, String> {
        if source.is_empty() {
            return Ok(Self {
                source: source.to_string(),
                pattern: None,
                all_spans: Vec::new(),
            });
        }

        // Parse mini notation AST first (for span collection)
        let ast = crate::pattern_system::mini::parse_ast(source).map_err(|e| e.to_string())?;

        // Collect all leaf spans from AST
        let all_spans = crate::pattern_system::mini::collect_leaf_spans(&ast);

        // Convert AST to pattern
        let pattern = crate::pattern_system::mini::convert::<IntervalValue>(&ast)
            .map_err(|e| e.to_string())?;

        Ok(Self {
            source: source.to_string(),
            pattern: Some(pattern),
            all_spans,
        })
    }

    /// Get the parsed pattern.
    pub fn pattern(&self) -> Option<&Pattern<IntervalValue>> {
        self.pattern.as_ref()
    }

    /// Get the source pattern string.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Get all leaf spans in the pattern (for frontend tracked decorations).
    pub fn all_spans(&self) -> &[(usize, usize)] {
        &self.all_spans
    }
}

impl<'de> Deserialize<'de> for IntervalPatternParam {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let source = String::deserialize(deserializer)?;
        if source.is_empty() {
            return Ok(Self::default());
        }
        Self::parse(&source).map_err(serde::de::Error::custom)
    }
}

impl Connect for IntervalPatternParam {
    fn connect(&mut self, _patch: &Patch) {
        // IntervalPatternParam has no signals to connect
    }
}

/// Cached hap info for voice assignment.
#[derive(Clone, Debug)]
struct CachedIntervalHap {
    /// Index in the combined hap list for unique identification
    hap_index: usize,
    /// The cycle this hap was cached for
    cached_cycle: i64,
    /// Hap timing
    whole_begin: f64,
    whole_end: f64,
    /// Source spans from interval pattern
    interval_spans: Vec<(usize, usize)>,
    /// Source spans from add pattern
    add_spans: Vec<(usize, usize)>,
}

impl CachedIntervalHap {
    fn contains(&self, playhead: f64) -> bool {
        playhead >= self.whole_begin && playhead < self.whole_end
    }
}

/// Per-voice state for polyphonic interval sequencer.
#[derive(Clone)]
struct IntervalVoiceState {
    /// Cached hap info for this voice
    cached_hap: Option<CachedIntervalHap>,
    /// Quantized voltage cached at voice allocation time
    cached_voltage: f64,
    /// Gate generator for this voice
    gate: TempGate,
    /// Trigger generator for this voice
    trigger: TempGate,
    /// Whether this voice is currently active
    active: bool,
    /// Timestamp when this voice was last assigned (for LRU stealing)
    last_assigned: f64,
}

impl Default for IntervalVoiceState {
    fn default() -> Self {
        Self {
            cached_hap: None,
            cached_voltage: 0.0,
            gate: TempGate::new_gate(TempGateState::Low),
            trigger: TempGate::new_gate(TempGateState::Low),
            active: false,
            last_assigned: 0.0,
        }
    }
}

fn default_channels() -> usize {
    4
}

#[derive(Deserialize, Default, ChannelCount, JsonSchema, Connect, Debug)]
#[serde(default, rename_all = "camelCase")]
pub struct IntervalSeqParams {
    /// Primary interval/degree pattern
    interval_pattern: IntervalPatternParam,
    /// Offset pattern added to interval_pattern
    add_pattern: IntervalPatternParam,
    /// Scale for quantizing degrees to pitches (supports optional octave, e.g. "C3(major)")
    scale: IntervalScaleParam,
    /// 2 channel control signal, sums the first 2 channels
    #[default_connection(module = RootClock, port = "playhead", channels = [0, 1])]
    playhead: PolySignal,
    /// Number of polyphonic voices (1-16, default 4)
    #[serde(default = "default_channels")]
    pub channels: usize,
}

/// Channel count derivation for IntervalSeq.
///
/// Analyzes both patterns together to determine maximum polyphony.
/// Since overlapping haps produce the Cartesian product (all combinations),
/// we must compute the combined haps and find max simultaneous events.
pub fn interval_seq_derive_channel_count(params: &IntervalSeqParams) -> usize {
    // If channels was explicitly set (non-default), use that
    if params.channels != default_channels() {
        return params.channels.clamp(1, PORT_MAX_CHANNELS);
    }

    derive_combined_polyphony(
        params.interval_pattern.pattern(),
        params.add_pattern.pattern(),
    )
}

/// Derive polyphony by computing combined haps from both patterns.
fn derive_combined_polyphony(
    interval_pattern: Option<&Pattern<IntervalValue>>,
    add_pattern: Option<&Pattern<IntervalValue>>,
) -> usize {
    const NUM_CYCLES: i64 = 90;
    const MAX_POLYPHONY: usize = 16;

    // Query both patterns
    let interval_haps = interval_pattern
        .map(|p| {
            p.query_arc(
                Fraction::from_integer(0),
                Fraction::from_integer(NUM_CYCLES),
            )
        })
        .unwrap_or_default();

    let add_haps = add_pattern
        .map(|p| {
            p.query_arc(
                Fraction::from_integer(0),
                Fraction::from_integer(NUM_CYCLES),
            )
        })
        .unwrap_or_default();

    // If both are empty, return 1
    if interval_haps.is_empty() && add_haps.is_empty() {
        return 1;
    }

    // Collect combined hap spans (as Fraction pairs for precision)
    // Use `part` spans since those are always present (whole can be None for continuous)
    let mut combined_spans: Vec<(Fraction, Fraction)> = Vec::new();

    if add_haps.is_empty() {
        // Only interval pattern: use its haps directly
        for hap in &interval_haps {
            if !hap.value.is_rest() {
                combined_spans.push((hap.part.begin.clone(), hap.part.end.clone()));
            }
        }
    } else if interval_haps.is_empty() {
        // Only add pattern: use its haps directly
        for hap in &add_haps {
            if !hap.value.is_rest() {
                combined_spans.push((hap.part.begin.clone(), hap.part.end.clone()));
            }
        }
    } else {
        // Both patterns: compute Cartesian product of overlapping haps
        for int_hap in &interval_haps {
            for add_hap in &add_haps {
                // Check for overlap using part spans
                if add_hap.part.begin < int_hap.part.end && add_hap.part.end > int_hap.part.begin {
                    // Both must be non-rest to produce a combined note
                    if !int_hap.value.is_rest() && !add_hap.value.is_rest() {
                        // Intersection of part spans
                        let begin = int_hap.part.begin.clone().max(add_hap.part.begin.clone());
                        let end = int_hap.part.end.clone().min(add_hap.part.end.clone());
                        combined_spans.push((begin, end));
                    }
                }
            }
        }
    }

    if combined_spans.is_empty() {
        return 1;
    }

    // Sweep line algorithm on combined spans
    let mut events: Vec<(Fraction, i32)> = Vec::with_capacity(combined_spans.len() * 2);

    for (begin, end) in combined_spans {
        events.push((begin, 1)); // start
        events.push((end, -1)); // end
    }

    events.sort_by(|a, b| {
        match a.0.cmp(&b.0) {
            Ordering::Equal => a.1.cmp(&b.1), // ends before starts at same time
            other => other,
        }
    });

    let mut current: usize = 0;
    let mut max_simultaneous: usize = 0;

    for (_time, delta) in events {
        if delta > 0 {
            current += 1;
            max_simultaneous = max_simultaneous.max(current);
            if max_simultaneous >= MAX_POLYPHONY {
                return MAX_POLYPHONY;
            }
        } else {
            current = current.saturating_sub(1);
        }
    }

    max_simultaneous.max(1)
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct IntervalSeqOutputs {
    #[output("cv", "control voltage output", default)]
    cv: PolyOutput,
    #[output("gate", "gate output")]
    gate: PolyOutput,
    #[output("trig", "trigger output")]
    trig: PolyOutput,
}

#[module(
    name = "seq.iCycle",
    description = "A scale-degree sequencer with interval and add patterns",
    channels_derive = interval_seq_derive_channel_count,
    args(intervalPattern, scale),
    stateful,
    patch_update,
)]
pub struct IntervalSeq {
    outputs: IntervalSeqOutputs,
    params: IntervalSeqParams,
    /// Per-voice state array
    voices: [IntervalVoiceState; PORT_MAX_CHANNELS],
    /// Round-robin voice index for allocation
    next_voice: usize,
    /// Cached cycle number
    cached_cycle: Option<i64>,
    /// Cached combined haps for the current cycle
    cached_combined_haps: Vec<CombinedHap>,
    /// Cached scale intervals for degree-to-semitone conversion
    scale_intervals: Vec<i8>,
    /// Base MIDI note for degree 0 (includes root pitch class + octave)
    base_midi: i32,
}

/// A combined hap from both patterns
#[derive(Clone, Debug)]
struct CombinedHap {
    whole_begin: f64,
    whole_end: f64,
    part_begin: f64,
    part_end: f64,
    /// Combined degree (interval + add), None if rest or missing
    degree: Option<i32>,
    has_onset: bool,
    /// Source spans from interval pattern
    interval_spans: Vec<(usize, usize)>,
    /// Source spans from add pattern
    add_spans: Vec<(usize, usize)>,
}

impl Default for IntervalSeq {
    fn default() -> Self {
        Self {
            outputs: IntervalSeqOutputs::default(),
            params: IntervalSeqParams::default(),
            voices: std::array::from_fn(|_| IntervalVoiceState::default()),
            next_voice: 0,
            cached_cycle: None,
            cached_combined_haps: Vec::new(),
            scale_intervals: vec![0, 2, 4, 5, 7, 9, 11], // Default major scale
            base_midi: 60,                               // C4
            _channel_count: 0,
        }
    }
}

impl IntervalSeq {
    /// Invalidate the cycle cache.
    fn invalidate_cache(&mut self) {
        self.cached_cycle = None;
        self.cached_combined_haps.clear();
    }

    /// Refresh the cycle cache by querying both patterns and combining.
    fn refresh_cache(&mut self, cycle: i64) {
        self.cached_combined_haps.clear();

        let interval_haps = self
            .params
            .interval_pattern
            .pattern()
            .map(|p| p.query_cycle_all(cycle))
            .unwrap_or_default();

        let add_haps = self
            .params
            .add_pattern
            .pattern()
            .map(|p| p.query_cycle_all(cycle))
            .unwrap_or_default();

        // If both patterns are empty, nothing to do
        if interval_haps.is_empty() && add_haps.is_empty() {
            self.cached_cycle = Some(cycle);
            return;
        }

        // Combine haps: for each interval hap, find overlapping add haps
        for int_hap in &interval_haps {
            let int_degree = int_hap.value.degree();

            // Find add_haps that overlap with this interval hap's whole span
            let overlapping_add: Vec<&DspHap<IntervalValue>> = add_haps
                .iter()
                .filter(|add| {
                    add.whole_begin < int_hap.whole_end && add.whole_end > int_hap.whole_begin
                })
                .collect();

            if overlapping_add.is_empty() {
                // No add hap at this time -> silence
                self.cached_combined_haps.push(CombinedHap {
                    whole_begin: int_hap.whole_begin,
                    whole_end: int_hap.whole_end,
                    part_begin: int_hap.part_begin,
                    part_end: int_hap.part_end,
                    degree: None,
                    has_onset: int_hap.has_onset(),
                    interval_spans: int_hap.get_active_spans(),
                    add_spans: Vec::new(),
                });
            } else {
                for add_hap in overlapping_add {
                    let add_degree = add_hap.value.degree();

                    // Calculate intersection of whole spans
                    let combined_begin = int_hap.whole_begin.max(add_hap.whole_begin);
                    let combined_end = int_hap.whole_end.min(add_hap.whole_end);

                    // Onset at intersection start if either has onset there
                    let has_onset = (int_hap.has_onset()
                        && int_hap.part_begin >= combined_begin
                        && int_hap.part_begin < combined_end)
                        || (add_hap.has_onset()
                            && add_hap.part_begin >= combined_begin
                            && add_hap.part_begin < combined_end);

                    let combined_degree = match (int_degree, add_degree) {
                        (Some(i), Some(a)) => Some(i + a),
                        _ => None, // Either is rest -> silence
                    };

                    self.cached_combined_haps.push(CombinedHap {
                        whole_begin: combined_begin,
                        whole_end: combined_end,
                        part_begin: combined_begin,
                        part_end: combined_end,
                        degree: combined_degree,
                        has_onset,
                        interval_spans: int_hap.get_active_spans(),
                        add_spans: add_hap.get_active_spans(),
                    });
                }
            }
        }

        // Check for add_haps that don't overlap any interval_hap -> silence
        for add_hap in &add_haps {
            let has_interval_overlap = interval_haps.iter().any(|int| {
                int.whole_begin < add_hap.whole_end && int.whole_end > add_hap.whole_begin
            });

            if !has_interval_overlap {
                self.cached_combined_haps.push(CombinedHap {
                    whole_begin: add_hap.whole_begin,
                    whole_end: add_hap.whole_end,
                    part_begin: add_hap.part_begin,
                    part_end: add_hap.part_end,
                    degree: None,
                    has_onset: add_hap.has_onset(),
                    interval_spans: Vec::new(),
                    add_spans: add_hap.get_active_spans(),
                });
            }
        }

        self.cached_cycle = Some(cycle);
    }

    /// Convert a scale degree to V/Oct voltage.
    fn degree_to_voltage(&self, degree: i32) -> f64 {
        if self.scale_intervals.is_empty() {
            // Chromatic fallback
            return midi_to_voct_f64(60.0 + degree as f64);
        }

        let scale_len = self.scale_intervals.len() as i32;

        // Handle negative degrees with proper wrapping
        let (octave, wrapped_degree) = if degree >= 0 {
            (degree / scale_len, (degree % scale_len) as usize)
        } else {
            // For negative: -1 in 7-note scale is degree 6 in octave -1
            let adj_degree = degree + 1;
            let octave = (adj_degree / scale_len) - 1;
            let wrapped = ((degree % scale_len) + scale_len) % scale_len;
            (octave, wrapped as usize)
        };

        // Get semitone offset within octave from scale intervals
        let semitone_in_scale = self
            .scale_intervals
            .get(wrapped_degree)
            .copied()
            .unwrap_or(0) as i32;

        // Total MIDI note: base_midi (root + octave) + degree_octave*12 + semitone_in_scale
        let midi = self.base_midi + (octave * 12) + semitone_in_scale;

        midi_to_voct_f64(midi as f64)
    }

    /// Update cached scale info from params.
    fn update_scale_cache(&mut self) {
        let scale: &ScaleParam = &self.params.scale;
        self.base_midi = scale.base_midi();
        if let Some(snapper) = scale.snapper() {
            self.scale_intervals = snapper.scale_intervals().to_vec();
        } else {
            // Chromatic - all 12 semitones
            self.scale_intervals = (0..12).map(|i| i as i8).collect();
        }
    }
}

impl IntervalSeq {
    fn update(&mut self, _sample_rate: f32) {
        let playhead = self.params.playhead.get(0).get_value() as f64
            + self.params.playhead.get(1).get_value() as f64;

        let num_channels = self.channel_count();

        // Release voices whose haps have ended
        self.release_ended_voices(playhead, num_channels);

        // Check if we have any patterns
        let has_interval = self.params.interval_pattern.pattern().is_some();
        let has_add = self.params.add_pattern.pattern().is_some();

        if !has_interval && !has_add {
            for ch in 0..num_channels {
                self.outputs.cv.set(ch, 0.0);
                self.outputs.gate.set(ch, self.voices[ch].gate.process());
                self.outputs.trig.set(ch, self.voices[ch].trigger.process());
            }
            return;
        }

        // Refresh cache if needed
        let current_cycle = playhead.floor() as i64;
        if self.cached_cycle != Some(current_cycle) {
            self.refresh_cache(current_cycle);
        }

        // Collect events to process (avoids borrow conflicts in the loop)
        let events_to_process: Vec<(
            usize,
            i32,
            f64,
            f64,
            Vec<(usize, usize)>,
            Vec<(usize, usize)>,
        )> = self
            .cached_combined_haps
            .iter()
            .enumerate()
            .filter_map(|(hap_index, combined)| {
                if !combined.has_onset {
                    return None;
                }

                if playhead < combined.part_begin || playhead >= combined.part_end {
                    return None;
                }

                let degree = combined.degree?;

                // Check if already assigned
                let already_assigned = (0..num_channels).any(|i| {
                    if let Some(ref existing) = self.voices[i].cached_hap {
                        existing.hap_index == hap_index && existing.cached_cycle == current_cycle
                    } else {
                        false
                    }
                });

                if already_assigned {
                    return None;
                }

                Some((
                    hap_index,
                    degree,
                    combined.whole_begin,
                    combined.whole_end,
                    combined.interval_spans.clone(),
                    combined.add_spans.clone(),
                ))
            })
            .collect();

        // Process collected events
        for (hap_index, degree, whole_begin, whole_end, interval_spans, add_spans) in
            events_to_process
        {
            // Allocate voice
            let voice_idx = self.allocate_voice(playhead, num_channels);

            // Cache the quantized voltage at allocation time
            let voltage = self.degree_to_voltage(degree);

            let voice = &mut self.voices[voice_idx];
            voice.cached_hap = Some(CachedIntervalHap {
                hap_index,
                cached_cycle: current_cycle,
                whole_begin,
                whole_end,
                interval_spans,
                add_spans,
            });
            voice.cached_voltage = voltage;
            voice.active = true;
            voice
                .gate
                .set_state(TempGateState::Low, TempGateState::High);
            voice
                .trigger
                .set_state(TempGateState::High, TempGateState::Low);
        }

        // Output all voices
        for ch in 0..num_channels {
            let voice = &mut self.voices[ch];

            if voice.active {
                self.outputs.cv.set(ch, voice.cached_voltage as f32);
            }

            self.outputs.gate.set(ch, voice.gate.process());
            self.outputs.trig.set(ch, voice.trigger.process());
        }
    }

    fn allocate_voice(&mut self, playhead: f64, num_channels: usize) -> usize {
        for i in 0..num_channels {
            let voice_idx = (self.next_voice + i) % num_channels;
            if !self.voices[voice_idx].active {
                self.next_voice = (voice_idx + 1) % num_channels;
                self.voices[voice_idx].last_assigned = playhead;
                return voice_idx;
            }
        }

        // Steal oldest
        let mut oldest_idx = 0;
        let mut oldest_time = f64::MAX;
        for i in 0..num_channels {
            if self.voices[i].last_assigned < oldest_time {
                oldest_time = self.voices[i].last_assigned;
                oldest_idx = i;
            }
        }

        self.voices[oldest_idx].active = false;
        self.voices[oldest_idx].cached_hap = None;
        self.voices[oldest_idx].last_assigned = playhead;
        self.next_voice = (oldest_idx + 1) % num_channels;

        oldest_idx
    }

    fn release_ended_voices(&mut self, playhead: f64, num_channels: usize) {
        for i in 0..num_channels {
            if let Some(ref cached) = self.voices[i].cached_hap {
                if !cached.contains(playhead) {
                    self.voices[i].active = false;
                    self.voices[i].cached_hap = None;
                    self.voices[i]
                        .gate
                        .set_state(TempGateState::Low, TempGateState::Low);
                }
            }
        }
    }
}

impl crate::types::StatefulModule for IntervalSeq {
    fn get_state(&self) -> Option<serde_json::Value> {
        let num_channels = self.channel_count().clamp(1, PORT_MAX_CHANNELS);
        let mut interval_spans: Vec<(usize, usize)> = Vec::new();
        let mut add_spans: Vec<(usize, usize)> = Vec::new();
        let mut any_active = false;

        for voice in self.voices.iter().take(num_channels) {
            if voice.active {
                if let Some(ref cached) = voice.cached_hap {
                    any_active = true;
                    interval_spans.extend(cached.interval_spans.iter().cloned());
                    add_spans.extend(cached.add_spans.iter().cloned());
                }
            }
        }

        if !any_active {
            None
        } else {
            // Deduplicate spans
            interval_spans.sort();
            interval_spans.dedup();
            add_spans.sort();
            add_spans.dedup();

            // Generic param_spans format: map of param name -> { spans, source, all_spans }
            // - spans: currently active spans (for highlighting)
            // - source: the evaluated pattern string
            // - all_spans: all leaf spans in the pattern (for creating tracked decorations at patch time)
            Some(serde_json::json!({
                "param_spans": {
                    "intervalPattern": {
                        "spans": interval_spans,
                        "source": self.params.interval_pattern.source(),
                        "all_spans": self.params.interval_pattern.all_spans(),
                    },
                    "addPattern": {
                        "spans": add_spans,
                        "source": self.params.add_pattern.source(),
                        "all_spans": self.params.add_pattern.all_spans(),
                    }
                },
                "num_channels": num_channels,
            }))
        }
    }
}

impl crate::types::PatchUpdateHandler for IntervalSeq {
    fn on_patch_update(&mut self) {
        self.invalidate_cache();
        self.update_scale_cache();
    }
}

message_handlers!(impl IntervalSeq {});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interval_value_from_atom() {
        use crate::pattern_system::mini::ast::AtomValue;
        use crate::pattern_system::mini::convert::FromMiniAtom;

        let v = IntervalValue::from_atom(&AtomValue::Number(5.0)).unwrap();
        assert!(matches!(v, IntervalValue::Degree(5)));

        let v = IntervalValue::from_atom(&AtomValue::Midi(3)).unwrap();
        assert!(matches!(v, IntervalValue::Degree(3)));
    }

    #[test]
    fn test_interval_pattern_parse() {
        let param = IntervalPatternParam::parse("0 1 2 3").unwrap();
        assert!(param.pattern().is_some());

        let param = IntervalPatternParam::parse("").unwrap();
        assert!(param.pattern().is_none());
    }

    #[test]
    fn test_degree_to_voltage_major() {
        let mut seq = IntervalSeq::default();
        seq.scale_intervals = vec![0, 2, 4, 5, 7, 9, 11]; // C major
        seq.base_midi = 60; // C4

        // Degree 0 = C4 = MIDI 60 = 0V
        let v0 = seq.degree_to_voltage(0);
        assert!((v0 - 0.0).abs() < 0.001);

        // Degree 1 = D4 = MIDI 62 = 2/12 V
        let v1 = seq.degree_to_voltage(1);
        assert!((v1 - (2.0 / 12.0)).abs() < 0.001);

        // Degree 7 = C5 = MIDI 72 = 1V
        let v7 = seq.degree_to_voltage(7);
        assert!((v7 - 1.0).abs() < 0.001);

        // Degree -1 = B3 = MIDI 59 = -1/12 V
        let v_neg1 = seq.degree_to_voltage(-1);
        assert!((v_neg1 - (-1.0 / 12.0)).abs() < 0.001);
    }

    #[test]
    fn test_degree_to_voltage_with_octave() {
        let mut seq = IntervalSeq::default();
        seq.scale_intervals = vec![0, 2, 4, 5, 7, 9, 11]; // C major
        seq.base_midi = 48; // C3

        // Degree 0 = C3 = MIDI 48 = -1V
        let v0 = seq.degree_to_voltage(0);
        assert!((v0 - (-1.0)).abs() < 0.001);

        // Degree 7 = C4 = MIDI 60 = 0V
        let v7 = seq.degree_to_voltage(7);
        assert!((v7 - 0.0).abs() < 0.001);

        // D3 root
        seq.base_midi = 50; // D3
        // Degree 0 = D3 = MIDI 50 = -10/12 V
        let v0 = seq.degree_to_voltage(0);
        assert!((v0 - (-10.0 / 12.0)).abs() < 0.001);
    }

    #[test]
    fn test_scale_param_with_octave() {
        use crate::dsp::utilities::quantizer::ScaleParam;

        let scale = ScaleParam::parse_with_octave("C3(major)").unwrap();
        assert_eq!(scale.base_midi(), 48);
        assert!(scale.snapper().is_some());

        let scale = ScaleParam::parse_with_octave("Db3(min)").unwrap();
        assert_eq!(scale.base_midi(), 49);
        assert!(scale.snapper().is_some());

        // Without octave defaults to octave 4
        let scale = ScaleParam::parse_with_octave("C(major)").unwrap();
        assert_eq!(scale.base_midi(), 60);

        let scale = ScaleParam::parse_with_octave("D(major)").unwrap();
        assert_eq!(scale.base_midi(), 62);
    }
}
