//! Seq module - A Strudel/TidalCycles style sequencer using the new pattern system.
//!
//! This module sequences pitch values using mini notation patterns with support for:
//! - MIDI note numbers (with cents precision)
//! - Musical notes (c4, bb3, etc.) with optional octave (defaults to 4)
//! - Module signals via `module(id:port:channel)` syntax
//! - Sample-and-hold signals via `module(id:port:channel)=` suffix
//! - Scale snapping via the `scale` operator
//!
//! The sequencer queries the pattern at the current playhead position and outputs:
//! - CV: V/Oct pitch (A0 = 0V)
//! - Gate: High while note is active
//! - Trig: Short pulse at note onset

use mi_plaits_dsp::fm::voice;
use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    Patch, PolySignal,
    dsp::utils::{TempGate, TempGateState, midi_to_voct_f64},
    pattern_system::DspHap,
    poly::{PORT_MAX_CHANNELS, PolyOutput},
    types::Connect,
};

use super::seq_operators::CachedOperator;
use super::seq_value::{SeqPatternParam, SeqValue};

/// Cached hap with pre-sampled values and resolved operators.
///
/// This caches the current hap being played, including:
/// - The DspHap from the pattern query
/// - Pre-sampled value for S&H signals
/// - Resolved operators (with dynamic scale roots evaluated at onset)
#[derive(Clone, Debug)]
struct CachedHap {
    /// The underlying hap with timing and value.
    hap: DspHap<SeqValue>,

    /// Pre-sampled MIDI value for sample-and-hold signals.
    /// None for continuous signals (read each tick) or non-signal values.
    sampled_midi: Option<f64>,

    /// Operators to apply at runtime (for signal values).
    /// Cloned from the pattern param, with dynamic scale roots resolved.
    operators: Vec<CachedOperator>,
}

impl CachedHap {
    /// Create a new cached hap from a DspHap.
    fn new(hap: DspHap<SeqValue>, operators: Vec<CachedOperator>) -> Self {
        // Sample S&H signals at creation time
        let sampled_midi = match &hap.value {
            SeqValue::Signal {
                signal,
                sample_and_hold: true,
            } => {
                // Sample the signal now and convert to MIDI
                let voct = signal.get_value() as f64;
                Some(voct * 12.0 + 33.0) // V/Oct to MIDI
            }
            _ => None,
        };

        Self {
            hap,
            sampled_midi,
            operators,
        }
    }

    /// Check if the playhead is within this hap's bounds.
    fn contains(&self, playhead: f64) -> bool {
        playhead >= self.hap.whole_begin && playhead < self.hap.whole_end
    }

    /// Get the CV output for this hap at the given playhead time.
    /// Applies cached operators to signal values.
    fn get_cv(&self, playhead: f64) -> Option<f64> {
        let midi = match &self.hap.value {
            SeqValue::Midi(m) => Some(*m),
            SeqValue::Note { .. } => self.hap.value.to_midi(),
            SeqValue::Signal {
                signal,
                sample_and_hold,
            } => {
                if *sample_and_hold {
                    // Use pre-sampled value
                    self.sampled_midi
                } else {
                    // Read signal continuously and convert to MIDI
                    let voct = signal.get_value() as f64;
                    let mut midi = voct * 12.0 + 33.0;

                    // Apply operators
                    for op in &self.operators {
                        midi = op.apply(midi, playhead);
                    }

                    Some(midi)
                }
            }
            SeqValue::Rest => None,
        }?;

        // For non-signal values, operators were already applied at parse time
        // Convert MIDI to V/Oct
        Some(midi_to_voct_f64(midi))
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
            gate: TempGate::new(TempGateState::Low, 0.0, 5.0),
            trigger: TempGate::new(TempGateState::Low, 0.0, 1.0),
            active: false,
            last_assigned: 0.0,
        }
    }
}

fn default_channels() -> usize {
    4
}

#[derive(Default, ChannelCount, JsonSchema)]
#[serde(default)]
struct SeqParams {
    /// Strudel/tidalcycles style pattern string
    pattern: SeqPatternParam,
    /// 2 channel control signal, sums the first 2 channels
    playhead: PolySignal,
    /// Number of polyphonic voices (1-16, default 4)
    #[serde(default = "default_channels")]
    channels: usize,
}

impl<'de> Deserialize<'de> for SeqParams {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(default)]
        struct SeqParamsHelper {
            pattern: SeqPatternParam,
            playhead: PolySignal,
            #[serde(default = "default_channels")]
            channels: usize,
        }

        impl Default for SeqParamsHelper {
            fn default() -> Self {
                Self {
                    pattern: SeqPatternParam::default(),
                    playhead: PolySignal::default(),
                    channels: default_channels(),
                }
            }
        }

        let helper = SeqParamsHelper::deserialize(deserializer)?;
        Ok(SeqParams {
            pattern: helper.pattern,
            playhead: helper.playhead,
            channels: helper.channels.clamp(1, PORT_MAX_CHANNELS),
        })
    }
}

impl Connect for SeqParams {
    fn connect(&mut self, patch: &Patch) {
        Connect::connect(&mut self.playhead, patch);
        Connect::connect(&mut self.pattern, patch);
    }
}

#[derive(Outputs, JsonSchema)]
struct SeqOutputs {
    #[output("cv", "control voltage output", default)]
    cv: PolyOutput,
    #[output("gate", "gate output")]
    gate: PolyOutput,
    #[output("trig", "trigger output")]
    trig: PolyOutput,
}

#[derive(Module)]
#[module(
    "seq",
    "A strudel/tidalcycles style sequencer",
    channels_param = "channels",
    channels_param_default = 4
)]
#[args(pattern, playhead?, channels?)]
#[stateful]
#[patch_update]
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

impl Default for Seq {
    fn default() -> Self {
        Self {
            outputs: SeqOutputs::default(),
            params: SeqParams::default(),
            voices: std::array::from_fn(|_| VoiceState::default()),
            next_voice: 0,
            cached_cycle: None,
            cached_haps: Vec::new(),
        }
    }
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
        let playhead = self.params.playhead.get(0).get_value() as f64
            + self.params.playhead.get(1).get_value() as f64;

        let num_channels = self.channel_count();

        // Set output channel counts
        self.outputs.cv.set_channels(num_channels as u8);
        self.outputs.gate.set_channels(num_channels as u8);
        self.outputs.trig.set_channels(num_channels as u8);

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
        let operators = self.params.pattern.operators.clone();
        for hap in self.cached_haps.iter() {
            if !hap.has_onset() || !hap.part_contains(playhead) {
                continue;
            }

            // Convert DspHap to CachedHap for voice assignment
            let cached = CachedHap::new(hap.clone(), operators.clone());

            if cached.is_rest() {
                continue; // Don't allocate voices for rests
            }

            // Check if this exact hap is already assigned to a voice
            // Compare both timing AND value - for chords (stacks), notes have same timing but different values
            let cached_cv = cached.get_cv(playhead);
            let already_assigned = (0..num_channels).any(|i| {
                if let Some(ref existing) = self.voices[i].cached_hap {
                    // Compare by timing
                    let same_timing = (existing.hap.whole_begin - cached.hap.whole_begin).abs() < 1e-9
                        && (existing.hap.whole_end - cached.hap.whole_end).abs() < 1e-9;
                    
                    if !same_timing {
                        return false;
                    }
                    
                    // Also compare by value (CV) to distinguish chord notes
                    let existing_cv = existing.get_cv(playhead);
                    match (existing_cv, cached_cv) {
                        (Some(e), Some(c)) => (e - c).abs() < 1e-9,
                        (None, None) => true, // Both rests or no CV
                        _ => false,
                    }
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

            if let Some(ref cached) = voice.cached_hap {
                if let Some(cv) = cached.get_cv(playhead) {
                    self.outputs.cv.set(ch, cv as f32);
                }
            }

            self.outputs.gate.set(ch, voice.gate.process());
            self.outputs.trig.set(ch, voice.trigger.process());
        }
    }

    /// Check for notes that have ended and mark voices as inactive.
    fn release_ended_voices(&mut self, playhead: f64, num_channels: usize) {
        for i in 0..num_channels {
            if let Some(ref cached) = self.voices[i].cached_hap {
                if !cached.contains(playhead) {
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
}

impl crate::types::StatefulModule for Seq {
    fn get_state(&self) -> Option<serde_json::Value> {
        let num_channels = self.channel_count();

        // Collect all source spans from all active voices
        let mut all_source_spans: Vec<(usize, usize)> = Vec::new();
        let mut any_non_rest = false;

        for voice in self.voices.iter().take(num_channels) {
            if let Some(ref cached) = voice.cached_hap {
                if !cached.is_rest() {
                    any_non_rest = true;
                    all_source_spans.extend(cached.hap.get_active_spans());
                }
            }
        }

        if all_source_spans.is_empty() && !any_non_rest {
            None
        } else {
            // Deduplicate spans (same span could be in multiple voices for stacked patterns)
            all_source_spans.sort();
            all_source_spans.dedup();

            Some(serde_json::json!({
                "source_spans": all_source_spans,
                "pattern_source": self.params.pattern.source(),
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
