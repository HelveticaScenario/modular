use crate::types::{Clickless, Signal};
use napi::{Result, sys::ThreadsafeFunctionReleaseMode::release};
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
#[args(gate)]
pub struct Adsr {
    outputs: AdsrOutputs,
    stage: EnvelopeStage,
    current_level: f32,
    gate_was_high: bool,
    attack: Clickless,
    decay: Clickless,
    release: Clickless,
    sustain: Clickless,
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
            attack: 0.01.into(),
            decay: 0.1.into(),
            release: 0.1.into(),
            sustain: 3.5.into(),
            params: AdsrParams::default(),
        }
    }
}

impl Adsr {
    fn update(&mut self, sample_rate: f32) -> () {
        // Smooth parameter targets to avoid clicks when values change (times in seconds)
        self.attack.update(self.params.attack.get_value_or(0.01).max(0.001));
        self.decay.update(self.params.decay.get_value_or(0.1).max(0.001));
        self.release.update(self.params.release.get_value_or(0.1).max(0.001));
        self.sustain
            .update(self.params.sustain.get_value_or(5.).max(0.0));

        let attack = *self.attack;
        let decay = *self.decay;
        let release_var = *self.release;

        let gate_on = self.params.gate.get_value() > 2.5;

        if gate_on && !self.gate_was_high {
            self.stage = EnvelopeStage::Attack;
        } else if !gate_on && self.gate_was_high {
            self.stage = EnvelopeStage::Release;
        }
        self.gate_was_high = gate_on;

        let sustain_level = (*self.sustain / 5.0).clamp(0.0, 1.0);

        match self.stage {
            EnvelopeStage::Idle => {
                self.current_level = 0.0;
            }
            EnvelopeStage::Attack => {
                if attack < 0.0001 {
                    self.current_level = 1.0;
                    self.stage = EnvelopeStage::Decay;
                } else {
                    let step = 1.0 / (attack * sample_rate);
                    self.current_level += step;
                    if self.current_level >= 1.0 {
                        self.current_level = 1.0;
                        self.stage = EnvelopeStage::Decay;
                    }
                }
            }
            EnvelopeStage::Decay => {
                if decay <= 0.0001 || self.current_level <= sustain_level {
                    self.current_level = sustain_level;
                    self.stage = EnvelopeStage::Sustain;
                } else {
                    let step = (1.0 - sustain_level) / (decay * sample_rate);
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
                if release_var <= 0.0001 {
                    self.current_level = 0.0;
                    self.stage = EnvelopeStage::Idle;
                } else {
                    let step = self.current_level / (release_var * sample_rate);
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

        self.outputs.sample = (self.current_level * 5.0).clamp(0.0, 5.0);
    }
}

message_handlers!(impl Adsr {});
