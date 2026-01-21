//! Seq module - A Strudel/TidalCycles style sequencer using the new pattern system.
//!
//! This module sequences pitch values using mini notation patterns with support for:
//! - MIDI note numbers (with cents precision)
//! - Musical notes (c4, bb3, etc.) with optional octave (defaults to 4)
//! - Module signals via `module(id:port)` syntax
//! - Sample-and-hold signals via `module(id:port)=` suffix
//! - Scale snapping via the `scale` operator
//!
//! The sequencer queries the pattern at the current playhead position and outputs:
//! - CV: V/Oct pitch (A0 = 0V)
//! - Gate: High while note is active
//! - Trig: Short pulse at note onset

use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    dsp::utils::{midi_to_voct_f64, TempGate, TempGateState},
    pattern_system::DspHap,
    types::{Connect, Signal},
    Patch,
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

#[derive(Default, JsonSchema)]
#[serde(default)]
struct SeqParams {
    /// Strudel/tidalcycles style pattern string
    pattern: SeqPatternParam,
    /// playhead control signal
    playhead: Signal,
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
            playhead: Signal,
        }

        impl Default for SeqParamsHelper {
            fn default() -> Self {
                Self {
                    pattern: SeqPatternParam::default(),
                    playhead: Signal::default(),
                }
            }
        }

        let helper = SeqParamsHelper::deserialize(deserializer)?;
        Ok(SeqParams {
            pattern: helper.pattern,
            playhead: helper.playhead,
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
    cv: f32,
    #[output("gate", "gate output")]
    gate: f32,
    #[output("trig", "trigger output")]
    trig: f32,
}

#[derive(Module)]
#[module("seq", "A strudel/tidalcycles style sequencer")]
#[args(pattern, playhead?)]
#[stateful]
pub struct Seq {
    outputs: SeqOutputs,
    params: SeqParams,
    trigger: TempGate,
    gate: TempGate,
    /// Cached hap for the current playhead position.
    cached_hap: Option<CachedHap>,
}

impl Default for Seq {
    fn default() -> Self {
        Self {
            outputs: SeqOutputs::default(),
            params: SeqParams::default(),
            trigger: TempGate::new(TempGateState::Low, 0.0, 1.0),
            gate: TempGate::new(TempGateState::Low, 0.0, 5.0),
            cached_hap: None,
        }
    }
}

impl Seq {
    fn update(&mut self, _sample_rate: f32) {
        let playhead = f64::from(self.params.playhead.get_value());

        // Check if we're still within the cached hap
        if let Some(ref cached) = self.cached_hap {
            if cached.contains(playhead) {
                // Use cached value
                self.process_cached_hap(playhead);
                return;
            }
        }

        // Need to query the pattern for a new hap
        let pattern = match self.params.pattern.pattern() {
            Some(p) => p,
            None => {
                // No pattern - output silence
                self.outputs.cv = 0.0;
                self.outputs.gate = 0.0;
                self.outputs.trig = 0.0;
                return;
            }
        };

        // Query pattern at playhead
        if let Some(dsp_hap) = pattern.query_at_dsp(playhead) {
            // Create cached hap with operators
            let operators = self.params.pattern.operators.clone();
            let cached = CachedHap::new(dsp_hap, operators);

            // Set gate/trigger states for new note onset
            if !cached.is_rest() {
                self.gate
                    .set_state(TempGateState::Low, TempGateState::High);
                self.trigger
                    .set_state(TempGateState::High, TempGateState::Low);
            } else {
                self.gate
                    .set_state(TempGateState::Low, TempGateState::Low);
                self.trigger
                    .set_state(TempGateState::Low, TempGateState::Low);
            }

            self.cached_hap = Some(cached);
            self.process_cached_hap(playhead);
        } else {
            // No hap at this time - output silence
            self.cached_hap = None;
            self.gate
                .set_state(TempGateState::Low, TempGateState::Low);
            self.trigger
                .set_state(TempGateState::Low, TempGateState::Low);
            self.outputs.cv = 0.0;
            self.outputs.gate = self.gate.process();
            self.outputs.trig = self.trigger.process();
        }
    }

    /// Process the cached hap and update outputs.
    fn process_cached_hap(&mut self, playhead: f64) {
        let cached = self.cached_hap.as_ref().unwrap();

        if let Some(cv) = cached.get_cv(playhead) {
            self.outputs.cv = cv as f32;
        }

        self.outputs.gate = self.gate.process();
        self.outputs.trig = self.trigger.process();
    }
}

impl crate::types::StatefulModule for Seq {
    fn get_state(&self) -> Option<serde_json::Value> {
        self.cached_hap.as_ref().map(|cached| {
            serde_json::json!({
                "active_hap": {
                    "begin": cached.hap.whole_begin,
                    "end": cached.hap.whole_end,
                    "is_rest": cached.is_rest(),
                },
                "source_spans": cached.hap.get_active_spans(),
            })
        })
    }
}

message_handlers!(impl Seq {});
