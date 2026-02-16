//! Seq module - A Strudel/TidalCycles style sequencer using the new pattern system.
//!
//! This module sequences pitch values using mini notation patterns with support for:
//! - V/Oct voltage values (pre-converted from MIDI/notes at parse time)
//! - Module signals via `module(id:port:channel)` syntax
//! - Sample-and-hold signals via `module(id:port:channel)=` suffix
//!
//! The sequencer queries the pattern at the current playhead position and outputs:
//! - CV: V/Oct pitch (A0 = 0V)
//! - Gate: High while note is active
//! - Trig: Short pulse at note onset

use std::cmp::Ordering;

use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    MonoSignal, PolySignal,
    dsp::utils::{TempGate, TempGateState},
    pattern_system::{DspHap, Fraction},
    poly::{PORT_MAX_CHANNELS, PolyOutput},
};

use super::seq_value::{SeqPatternParam, SeqValue};

/// Cached hap with pre-sampled values.
///
/// This caches the current hap being played, including:
/// - The DspHap from the pattern query
/// - Pre-sampled voltage value for S&H signals
#[derive(Clone, Debug)]
struct CachedHap {
    /// The underlying hap with timing and value.
    hap: DspHap<SeqValue>,

    /// Index of this hap in the cached_haps vector.
    /// Used to uniquely identify hap instances for voice assignment.
    hap_index: usize,

    /// Pre-sampled voltage for sample-and-hold signals.
    /// None for continuous signals (read each tick) or non-signal values.
    sampled_voltage: Option<f64>,

    /// The cycle this hap was cached for.
    cached_cycle: i64,
}

impl CachedHap {
    /// Create a new cached hap from a DspHap.
    fn new(hap: DspHap<SeqValue>, hap_index: usize, cached_cycle: i64) -> Self {
        // Sample S&H signals at creation time
        let sampled_voltage = match &hap.value {
            SeqValue::Signal {
                signal,
                sample_and_hold: true,
            } => {
                // Sample the signal voltage directly
                Some(signal.get_value() as f64)
            }
            _ => None,
        };

        Self {
            hap,
            hap_index,
            sampled_voltage,
            cached_cycle,
        }
    }

    /// Check if the playhead is within this hap's bounds.
    fn contains(&self, playhead: f64) -> bool {
        playhead >= self.hap.whole_begin && playhead < self.hap.whole_end
    }

    /// Get the CV output for this hap.
    /// Returns voltage directly (no MIDI conversion needed).
    fn get_cv(&self) -> Option<f64> {
        match &self.hap.value {
            SeqValue::Voltage(v) => Some(*v),
            SeqValue::Signal {
                signal,
                sample_and_hold,
            } => {
                if *sample_and_hold {
                    // Use pre-sampled voltage
                    self.sampled_voltage
                } else {
                    // Read signal voltage continuously
                    Some(signal.get_value() as f64)
                }
            }
            SeqValue::Rest => None,
        }
    }

    /// Check if this is a rest hap.
    fn is_rest(&self) -> bool {
        self.hap.value.is_rest()
    }
}

/// Per-voice state for polyphonic sequencer.
#[derive(Clone)]
struct VoiceState {
    /// Cached hap for this voice's current playhead position.
    cached_hap: Option<CachedHap>,
    /// Gate generator for this voice.
    gate: TempGate,
    /// Trigger generator for this voice.
    trigger: TempGate,
    /// Whether this voice is currently active (playing a note).
    active: bool,
    /// Timestamp when this voice was last assigned (for LRU stealing).
    last_assigned: f64,
}

impl Default for VoiceState {
    fn default() -> Self {
        Self {
            cached_hap: None,
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
pub struct SeqParams {
    /// pattern string in mini-notation
    pattern: SeqPatternParam,
    /// playhead position (driven by the global clock)
    #[default_connection(module = RootClock, port = "playhead", channels = [0, 1])]
    playhead: MonoSignal,
    /// Number of polyphonic voices (1-16)
    pub channels: Option<usize>,
    /// The pattern string (used for serialization)
    #[serde(skip)]
    #[schemars(skip)]
    pub pattern_source: String,
}

/// Channel count derivation for Seq.
///
/// Analyzes the pattern to determine maximum polyphony by running 90 cycles
/// of the pattern and counting maximum simultaneous haps.
///
/// This is called by TypeScript to derive channel count from params.
/// Inside Seq::update(), we read params.channels directly (which TypeScript
/// will have already set based on this analysis, or user explicitly set).
pub fn seq_derive_channel_count(params: &SeqParams) -> usize {
    // If channels was explicitly set (non-default), use that
    if let Some(channels) = params.channels {
        return channels.clamp(1, PORT_MAX_CHANNELS);
    }

    // Otherwise, analyze pattern polyphony
    let Some(pattern) = params.pattern.pattern() else {
        return default_channels();
    };

    const NUM_CYCLES: i64 = 90;
    const MAX_POLYPHONY: usize = 16;

    // Query all cycles at once
    let haps = pattern.query_arc(
        Fraction::from_integer(0),
        Fraction::from_integer(NUM_CYCLES),
    );

    // Sweep line algorithm: create +1 events at start, -1 events at end
    let mut events: Vec<(Fraction, i32)> = Vec::with_capacity(haps.len() * 2);

    for hap in &haps {
        if hap.value.is_rest() {
            continue;
        }
        events.push((hap.part.begin.clone(), 1)); // +1 at start
        events.push((hap.part.end.clone(), -1)); // -1 at end
    }

    // Sort by time, with ends (-1) before starts (+1) at same time
    events.sort_by(|a, b| {
        match a.0.cmp(&b.0) {
            Ordering::Equal => a.1.cmp(&b.1), // -1 comes before +1
            other => other,
        }
    });

    // Sweep through events tracking current and max polyphony
    let mut current: usize = 0;
    let mut max_simultaneous: usize = 0;

    for (_time, delta) in events {
        if delta > 0 {
            current += 1;
            max_simultaneous = max_simultaneous.max(current);
            // Early exit if we hit the cap
            if max_simultaneous >= MAX_POLYPHONY {
                return MAX_POLYPHONY;
            }
        } else {
            current = current.saturating_sub(1);
        }
    }
    max_simultaneous.max(1) // At least 1 channel
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct SeqOutputs {
    #[output("cv", "pitch output in V/Oct", default)]
    cv: PolyOutput,
    #[output("gate", "high (5 V) while a note is active, low (0 V) otherwise", range = (0.0, 5.0))]
    gate: PolyOutput,
    #[output("trig", "short pulse (5 V) at the start of each note", range = (0.0, 5.0))]
    trig: PolyOutput,
}

/// Pattern sequencer using mini-notation strings.
///
/// Write rhythmic and melodic patterns using a compact text syntax ported
/// from TidalCycles/Strudel. The pattern loops each **cycle** and supports
/// polyphony — overlapping notes are automatically allocated to separate
/// output channels.
///
/// ## Cycles
///
/// A **cycle** is one full traversal of the pattern. The playhead position
/// determines timing: its integer part selects the current cycle number and
/// the fractional part selects the position within that cycle. Space-separated
/// values divide the cycle into equal time slots.
///
/// ## Values
///
/// | Syntax | Meaning | Example |
/// |--------|---------|---------|
/// | Note name | Pitch (octave defaults to 3) | `'c4'`, `'a#3'`, `'db5'` |
/// | Bare number | MIDI note number | `60`, `72` |
/// | `Xhz` | Frequency | `'440hz'` |
/// | `Xv` | Explicit voltage | `'0v'`, `'1v'`, `'-0.5v'` |
/// | `~` | Rest (gate low, no change in CV) | `'c4 ~ e4 ~'` |
///
/// Bare numbers are MIDI note numbers (A0 = MIDI 33 = 0 V).
///
/// ## Grouping
///
/// - **`[a b c]`** — fast subsequence: subdivides the parent time slot so all
///   elements play within it.
/// - **`<a b c>`** — slow / alternating: plays one element per cycle,
///   advancing each time the pattern loops.
///
/// ```js
/// $cycle("c4 [d4 e4]")   // c4 for half the cycle, d4 & e4 share the other half
/// $cycle("<c4 g4> e4")   // cycle 1: c4 e4, cycle 2: g4 e4, …
/// ```
///
/// ## Stacks
///
/// **`a b, c d`** — comma-separated patterns play **simultaneously** (layered).
/// Each sub-pattern has its own independent timing.
///
/// ```js
/// $cycle("c4 e4, g4 b4")   // two patterns layered on top of each other
/// $cycle("c4 d4 e4, g3")   // three-note melody over a pedal tone
/// ```
///
/// ## Random choice
///
/// **`a|b|c`** — randomly selects one option each time the slot is reached.
///
/// ```js
/// $cycle("c4|d4|e4 g4")  // first slot is a random pick each cycle
/// ```
///
/// ## Nesting
///
/// Grouping, stacks, and random choice nest arbitrarily:
///
/// ```js
/// $cycle("<c4 [d4 e4]> [f4|g4 a4]")  // slow + fast + random combined
/// $cycle("[c4 e4, g4] a4")            // stack inside a fast subsequence
/// ```
///
/// ## Per-element modifiers
///
/// Modifiers attach directly to an element (no spaces). Multiple modifiers
/// can be chained in any order.
///
/// | Modifier | Syntax | Meaning |
/// |----------|--------|---------|
/// | Weight | `@n` | Relative duration within a sequence (default 1). `c4@2 e4` gives c4 twice the time. |
/// | Speed up | `*n` | Repeat/subdivide `n` times within the slot. `c4*3` plays c4 three times. |
/// | Slow down | `/n` | Stretch over `n` cycles. `c4/2` plays every other cycle. |
/// | Replicate | `!n` | Duplicate the element `n` times (default 2). `c4!3` is equivalent to `c4 c4 c4`. |
/// | Degrade | `?` or `?n` | Randomly drop the element. `c4?` drops ~50 % of the time; `c4?0.8` drops 80 %. |
/// | Euclidean | `(k,n)` or `(k,n,offset)` | Distribute `k` pulses over `n` steps using the Bjorklund algorithm. Optional `offset` rotates the pattern. |
///
/// ```js
/// $cycle("c4*2 e4 g4")        // c4 plays twice in its slot
/// $cycle("c4@3 e4 g4")        // c4 gets 3/5 of the cycle, e4 and g4 get 1/5 each
/// $cycle("c4? e4 g4")         // c4 randomly drops out ~50 % of the time
/// $cycle("c4(3,8) e4")        // Euclidean: 3 hits spread over 8 steps
/// $cycle("[c4 d4 e4 f4](3,8)") // Euclidean applied to a subpattern
/// ```
///
/// Modifier operands can also be subpatterns: `c4*[2 3]` alternates between
/// doubling and tripling each slot.
///
/// ## Outputs
///
/// - **cv** — V/Oct pitch (C4 = 0 V).
/// - **gate** — 5 V while a note is active, 0 V otherwise.
/// - **trig** — single-sample 5 V pulse at each note onset.
#[module(
    name = "$cycle",
    description = "Pattern sequencer using mini-notation strings",
    channels_derive = seq_derive_channel_count,
    args(pattern),
    stateful,
    patch_update,
)]
#[derive(Default)]
pub struct Seq {
    outputs: SeqOutputs,
    params: SeqParams,
    /// Per-voice state array
    voices: [VoiceState; PORT_MAX_CHANNELS],
    /// Round-robin voice index for allocation
    next_voice: usize,
    /// Cached cycle number (integer part of playhead)
    cached_cycle: Option<i64>,
    /// Cached haps for the current cycle (all haps intersecting the cycle)
    cached_haps: Vec<DspHap<SeqValue>>,
}

impl Seq {
    /// Invalidate the cycle cache, forcing a refresh on next update.
    fn invalidate_cache(&mut self) {
        self.cached_cycle = None;
        self.cached_haps.clear();
    }

    /// Refresh the cycle cache for the given cycle.
    fn refresh_cache(&mut self, cycle: i64) {
        if let Some(pattern) = self.params.pattern.pattern() {
            self.cached_haps = pattern.query_cycle_all(cycle);
            self.cached_cycle = Some(cycle);
        } else {
            self.cached_haps.clear();
            self.cached_cycle = None;
        }
    }

    fn update(&mut self, _sample_rate: f32) {
        let playhead = self.params.playhead.get_value_f64();

        // Use precomputed channel count from _channel_count (set by try_update_params)
        let num_channels = self.channel_count();
        // Set output channel counts

        // Release voices whose haps have ended
        self.release_ended_voices(playhead, num_channels);

        // Get pattern - if no pattern, output silence
        if self.params.pattern.pattern().is_none() {
            for ch in 0..num_channels {
                self.outputs.cv.set(ch, 0.0);
                self.outputs.gate.set(ch, self.voices[ch].gate.process());
                self.outputs.trig.set(ch, self.voices[ch].trigger.process());
            }
            return;
        }

        // Check if we need to refresh the cache (cycle boundary crossed or cache invalid)
        let current_cycle = playhead.floor() as i64;
        if self.cached_cycle != Some(current_cycle) {
            self.refresh_cache(current_cycle);
        }

        // Process new onsets
        for (hap_index, hap) in self.cached_haps.iter().enumerate() {
            if !hap.has_onset() || !hap.part_contains(playhead) {
                continue;
            }

            // Convert DspHap to CachedHap for voice assignment
            let cached = CachedHap::new(hap.clone(), hap_index, current_cycle);

            if cached.is_rest() {
                continue; // Don't allocate voices for rests
            }

            // Check if this exact hap instance is already assigned to a voice
            // Use hap_index + cycle to uniquely identify each hap instance
            // This allows identical notes in chords (e.g., 'g,g,g') to each get their own voice
            let already_assigned = (0..num_channels).any(|i| {
                if let Some(ref existing) = self.voices[i].cached_hap {
                    existing.hap_index == cached.hap_index
                        && existing.cached_cycle == cached.cached_cycle
                } else {
                    false
                }
            });

            if already_assigned {
                continue;
            }

            let mut allocate_voice = || {
                // First pass: look for inactive voices starting from next_voice
                for i in 0..num_channels {
                    let voice_idx = (self.next_voice + i) % num_channels;
                    if !self.voices[voice_idx].active {
                        self.next_voice = (voice_idx + 1) % num_channels;
                        self.voices[voice_idx].last_assigned = playhead;
                        return voice_idx;
                    }
                }

                // All voices active - steal the oldest (LRU)
                let mut oldest_idx = 0;
                let mut oldest_time = f64::MAX;
                for i in 0..num_channels {
                    if self.voices[i].last_assigned < oldest_time {
                        oldest_time = self.voices[i].last_assigned;
                        oldest_idx = i;
                    }
                }

                // Reset the stolen voice
                self.voices[oldest_idx].active = false;
                self.voices[oldest_idx].cached_hap = None;
                self.voices[oldest_idx].last_assigned = playhead;
                self.next_voice = (oldest_idx + 1) % num_channels;

                oldest_idx
            };

            // Find the next available voice using round-robin with LRU voice stealing.
            let voice_idx = allocate_voice();
            let voice = &mut self.voices[voice_idx];
            voice.cached_hap = Some(cached);
            voice.active = true;
            voice
                .gate
                .set_state(TempGateState::Low, TempGateState::High);
            voice
                .trigger
                .set_state(TempGateState::High, TempGateState::Low);
        }

        // Process all voices and update outputs
        for ch in 0..num_channels {
            let voice = &mut self.voices[ch];

            if let Some(ref cached) = voice.cached_hap
                && let Some(cv) = cached.get_cv()
            {
                self.outputs.cv.set(ch, cv as f32);
            }

            self.outputs.gate.set(ch, voice.gate.process());
            self.outputs.trig.set(ch, voice.trigger.process());
        }
    }

    /// Check for notes that have ended and mark voices as inactive.
    fn release_ended_voices(&mut self, playhead: f64, num_channels: usize) {
        for i in 0..num_channels {
            if let Some(ref cached) = self.voices[i].cached_hap
                && !cached.contains(playhead)
            {
                self.voices[i].active = false;
                self.voices[i].cached_hap = None;
                // Gate goes low
                self.voices[i]
                    .gate
                    .set_state(TempGateState::Low, TempGateState::Low);
            }
        }
    }
}

impl crate::types::StatefulModule for Seq {
    fn get_state(&self) -> Option<serde_json::Value> {
        let num_channels = self.channel_count().clamp(1, PORT_MAX_CHANNELS);
        // Collect all source spans from all active voices
        let mut active_spans: Vec<(usize, usize)> = Vec::new();
        let mut any_non_rest = false;

        for voice in self.voices.iter().take(num_channels) {
            if let Some(ref cached) = voice.cached_hap
                && !cached.is_rest()
            {
                any_non_rest = true;
                active_spans.extend(cached.hap.get_active_spans());
            }
        }

        if active_spans.is_empty() && !any_non_rest {
            None
        } else {
            // Deduplicate spans (same span could be in multiple voices for stacked patterns)
            active_spans.sort();
            active_spans.dedup();

            // Generic param_spans format: map of param name -> { spans, source, all_spans }
            // - spans: currently active spans (for highlighting)
            // - source: the evaluated pattern string
            // - all_spans: all leaf spans in the pattern (for creating tracked decorations at patch time)
            Some(serde_json::json!({
                "param_spans": {
                    "pattern": {
                        "spans": active_spans,
                        "source": self.params.pattern.source(),
                        "all_spans": self.params.pattern.all_spans(),
                    }
                },
                "num_channels": num_channels,
            }))
        }
    }
}

impl crate::types::PatchUpdateHandler for Seq {
    fn on_patch_update(&mut self) {
        // Invalidate cache so it refreshes on next update with new pattern
        self.invalidate_cache();
    }
}

message_handlers!(impl Seq {});
