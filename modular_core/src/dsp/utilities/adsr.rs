use crate::{
    dsp::utils::clamp,
    types::{Signal, smooth_value},
};
use anyhow::{anyhow, Result};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, Default, JsonSchema, Connect)]
#[serde(default)]
struct AdsrParams {
    /// gate input (expects >0V for on)
    gate: Signal,
    /// attack time in seconds
    attack: Signal,
    /// decay time in seconds
    decay: Signal,
    /// sustain level in volts (0-5)
    sustain: Signal,
    /// release time in seconds
    release: Signal,
}

#[derive(Clone, Copy, PartialEq)]
enum EnvelopeStage {
    Idle,
    Attack,
    Decay,
    Sustain,
    Release,
}

impl Default for EnvelopeStage {
    fn default() -> Self {
        EnvelopeStage::Idle
    }
}

#[derive(Module)]
#[module("adsr", "ADSR envelope generator")]
pub struct Adsr {
    outputs: AdsrOutputs,
    stage: EnvelopeStage,
    current_level: f32,
    gate_was_high: bool,
    smoothed_attack: f32,
    smoothed_decay: f32,
    smoothed_release: f32,
    smoothed_sustain: f32,
    params: AdsrParams,
}

#[derive(Outputs, JsonSchema)]
struct AdsrOutputs {
    #[output("output", "envelope output", default)]
    sample: f32,
}

impl Default for Adsr {
    fn default() -> Self {
        Self {
            outputs: AdsrOutputs::default(),
            stage: EnvelopeStage::Idle,
            current_level: 0.0,
            gate_was_high: false,
            smoothed_attack: 0.01,
            smoothed_decay: 0.1,
            smoothed_release: 0.1,
            smoothed_sustain: 3.5,
            params: AdsrParams::default(),
        }
    }
}

impl Adsr {
    fn update(&mut self, sample_rate: f32) -> () {
        // Smooth parameter targets to avoid clicks when values change
        let target_attack = clamp(0.0, 10.0, self.params.attack.get_value_or(0.01));
        let target_decay = clamp(0.0, 10.0, self.params.decay.get_value_or(0.1));
        let target_release = clamp(0.0, 10.0, self.params.release.get_value_or(0.2));
        let target_sustain = clamp(0.0, 5.0, self.params.sustain.get_value_or(3.5));

        self.smoothed_attack = smooth_value(self.smoothed_attack, target_attack);
        self.smoothed_decay = smooth_value(self.smoothed_decay, target_decay);
        self.smoothed_release = smooth_value(self.smoothed_release, target_release);
        self.smoothed_sustain = smooth_value(self.smoothed_sustain, target_sustain);

        let gate_on = self.params.gate.get_value() > 2.5;

        if gate_on && !self.gate_was_high {
            self.stage = EnvelopeStage::Attack;
        } else if !gate_on && self.gate_was_high {
            self.stage = EnvelopeStage::Release;
        }
        self.gate_was_high = gate_on;

        let sustain_level = (self.smoothed_sustain / 5.0).clamp(0.0, 1.0);

        match self.stage {
            EnvelopeStage::Idle => {
                self.current_level = 0.0;
            }
            EnvelopeStage::Attack => {
                if self.smoothed_attack <= 0.0001 {
                    self.current_level = 1.0;
                    self.stage = EnvelopeStage::Decay;
                } else {
                    let step = 1.0 / (self.smoothed_attack * sample_rate);
                    self.current_level += step;
                    if self.current_level >= 1.0 {
                        self.current_level = 1.0;
                        self.stage = EnvelopeStage::Decay;
                    }
                }
            }
            EnvelopeStage::Decay => {
                if self.smoothed_decay <= 0.0001 || self.current_level <= sustain_level {
                    self.current_level = sustain_level;
                    self.stage = EnvelopeStage::Sustain;
                } else {
                    let step = (1.0 - sustain_level) / (self.smoothed_decay * sample_rate);
                    self.current_level = (self.current_level - step).max(sustain_level);
                    if self.current_level <= sustain_level {
                        self.current_level = sustain_level;
                        self.stage = EnvelopeStage::Sustain;
                    }
                }
            }
            EnvelopeStage::Sustain => {
                self.current_level = sustain_level;
                if !gate_on {
                    self.stage = EnvelopeStage::Release;
                }
            }
            EnvelopeStage::Release => {
                if self.smoothed_release <= 0.0001 {
                    self.current_level = 0.0;
                    self.stage = EnvelopeStage::Idle;
                } else {
                    let step = self.current_level / (self.smoothed_release * sample_rate);
                    self.current_level = (self.current_level - step).max(0.0);
                    if self.current_level <= 0.00001 {
                        self.current_level = 0.0;
                        self.stage = if gate_on {
                            EnvelopeStage::Attack
                        } else {
                            EnvelopeStage::Idle
                        };
                    }
                }
            }
        }

        self.outputs.sample = clamp(0.0, 5.0, self.current_level * 5.0);
    }
}

message_handlers!(impl Adsr {});
