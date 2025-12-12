use crate::{
    dsp::utils::clamp,
    types::{ChannelBuffer, InternalParam, NUM_CHANNELS, smooth_value},
};
use anyhow::{Result, anyhow};

#[derive(Default, Params)]
struct AdsrParams {
    #[param("gate", "gate input (expects >0V for on)")]
    gate: InternalParam,
    #[param("attack", "attack time in seconds")]
    attack: InternalParam,
    #[param("decay", "decay time in seconds")]
    decay: InternalParam,
    #[param("sustain", "sustain level in volts (0-5)")]
    sustain: InternalParam,
    #[param("release", "release time in seconds")]
    release: InternalParam,
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
    #[output("output", "envelope output", default)]
    sample: ChannelBuffer,
    stage: [EnvelopeStage; NUM_CHANNELS],
    current_level: ChannelBuffer,
    gate_was_high: [bool; NUM_CHANNELS],
    smoothed_attack: ChannelBuffer,
    smoothed_decay: ChannelBuffer,
    smoothed_release: ChannelBuffer,
    smoothed_sustain: ChannelBuffer,
    params: AdsrParams,
}

impl Default for Adsr {
    fn default() -> Self {
        Self {
            sample: ChannelBuffer::default(),
            stage: [EnvelopeStage::Idle; NUM_CHANNELS],
            current_level: ChannelBuffer::default(),
            gate_was_high: [false; NUM_CHANNELS],
            smoothed_attack: [0.01; NUM_CHANNELS],
            smoothed_decay: [0.1; NUM_CHANNELS],
            smoothed_release: [0.1; NUM_CHANNELS],
            smoothed_sustain: [3.5; NUM_CHANNELS],
            params: AdsrParams::default(),
        }
    }
}

impl Adsr {
    fn update(&mut self, sample_rate: f32) -> () {
        let mut target_attack = [0.01; NUM_CHANNELS];
        let mut target_decay = [0.1; NUM_CHANNELS];
        let mut target_release = [0.2; NUM_CHANNELS];
        let mut target_sustain = [3.5; NUM_CHANNELS];
        let mut gate = ChannelBuffer::default();

        self.params
            .attack
            .get_value_or(&mut target_attack, &[0.01; NUM_CHANNELS]);
        self.params
            .decay
            .get_value_or(&mut target_decay, &[0.1; NUM_CHANNELS]);
        self.params
            .release
            .get_value_or(&mut target_release, &[0.2; NUM_CHANNELS]);
        self.params
            .sustain
            .get_value_or(&mut target_sustain, &[3.5; NUM_CHANNELS]);
        self.params.gate.get_value(&mut gate);

        let sr = sample_rate.max(1.0);
        for i in 0..NUM_CHANNELS {
            let ta = clamp(0.0, 10.0, target_attack[i]);
            let td = clamp(0.0, 10.0, target_decay[i]);
            let tr = clamp(0.0, 10.0, target_release[i]);
            let ts = clamp(0.0, 5.0, target_sustain[i]);

            self.smoothed_attack[i] = smooth_value(self.smoothed_attack[i], ta);
            self.smoothed_decay[i] = smooth_value(self.smoothed_decay[i], td);
            self.smoothed_release[i] = smooth_value(self.smoothed_release[i], tr);
            self.smoothed_sustain[i] = smooth_value(self.smoothed_sustain[i], ts);

            let gate_on = gate[i] > 2.5;
            if gate_on && !self.gate_was_high[i] {
                self.stage[i] = EnvelopeStage::Attack;
            } else if !gate_on && self.gate_was_high[i] {
                self.stage[i] = EnvelopeStage::Release;
            }
            self.gate_was_high[i] = gate_on;

            let sustain_level = (self.smoothed_sustain[i] / 5.0).clamp(0.0, 1.0);
            match self.stage[i] {
                EnvelopeStage::Idle => {
                    self.current_level[i] = 0.0;
                }
                EnvelopeStage::Attack => {
                    if self.smoothed_attack[i] <= 0.0001 {
                        self.current_level[i] = 1.0;
                        self.stage[i] = EnvelopeStage::Decay;
                    } else {
                        let step = 1.0 / (self.smoothed_attack[i] * sr);
                        self.current_level[i] += step;
                        if self.current_level[i] >= 1.0 {
                            self.current_level[i] = 1.0;
                            self.stage[i] = EnvelopeStage::Decay;
                        }
                    }
                }
                EnvelopeStage::Decay => {
                    if self.smoothed_decay[i] <= 0.0001 || self.current_level[i] <= sustain_level {
                        self.current_level[i] = sustain_level;
                        self.stage[i] = EnvelopeStage::Sustain;
                    } else {
                        let step = (1.0 - sustain_level) / (self.smoothed_decay[i] * sr);
                        self.current_level[i] = (self.current_level[i] - step).max(sustain_level);
                        if self.current_level[i] <= sustain_level {
                            self.current_level[i] = sustain_level;
                            self.stage[i] = EnvelopeStage::Sustain;
                        }
                    }
                }
                EnvelopeStage::Sustain => {
                    self.current_level[i] = sustain_level;
                    if !gate_on {
                        self.stage[i] = EnvelopeStage::Release;
                    }
                }
                EnvelopeStage::Release => {
                    if self.smoothed_release[i] <= 0.0001 {
                        self.current_level[i] = 0.0;
                        self.stage[i] = EnvelopeStage::Idle;
                    } else {
                        let step = self.current_level[i] / (self.smoothed_release[i] * sr);
                        self.current_level[i] = (self.current_level[i] - step).max(0.0);
                        if self.current_level[i] <= 0.00001 {
                            self.current_level[i] = 0.0;
                            self.stage[i] = if gate_on {
                                EnvelopeStage::Attack
                            } else {
                                EnvelopeStage::Idle
                            };
                        }
                    }
                }
            }

            self.sample[i] = clamp(0.0, 5.0, self.current_level[i] * 5.0);
        }
    }
}
