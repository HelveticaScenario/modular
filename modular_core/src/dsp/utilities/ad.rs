use crate::{
    dsp::utils::clamp,
    types::{InternalParam, smooth_value},
};

use anyhow::{Result, anyhow};

#[derive(Default, Params)]
struct AdParams {
    #[param("gate", "gate input (expects >0V for on)")]
    gate: InternalParam,
    #[param("attack", "attack time (approximate seconds)")]
    attack: InternalParam,
    #[param("decay", "decay time (approximate seconds)")]
    decay: InternalParam,
}

#[derive(Clone, Copy, PartialEq)]
enum EnvelopeStage {
    Idle,
    Attack,
}

impl Default for EnvelopeStage {
    fn default() -> Self {
        EnvelopeStage::Idle
    }
}

#[derive(Module)]
#[module("ad", "Attack-decay envelope generator")]
pub struct Ad {
    #[output("output", "envelope output", default)]
    sample: f32,
    stage: EnvelopeStage,
    current_value: f32,
    target: f32,
    rate: f32,
    scale: f32,
    gate_was_high: bool,
    smoothed_attack: f32,
    smoothed_decay: f32,
    params: AdParams,
}

impl Default for Ad {
    fn default() -> Self {
        Self {
            sample: 0.0,
            stage: EnvelopeStage::Idle,
            current_value: 0.0,
            target: 0.0,
            rate: 0.0,
            scale: 0.0,
            gate_was_high: false,
            smoothed_attack: 0.01,
            smoothed_decay: 0.1,
            params: AdParams::default(),
        }
    }
}

impl Ad {
    fn update(&mut self, sample_rate: f32) -> () {
        let target_attack = clamp(0.0, 10.0, self.params.attack.get_value_or(0.01));
        let target_decay = clamp(0.0, 10.0, self.params.decay.get_value_or(0.1));

        self.smoothed_attack = smooth_value(self.smoothed_attack, target_attack);
        self.smoothed_decay = smooth_value(self.smoothed_decay, target_decay);

        let gate = self.params.gate.get_value_or(0.0);
        let gate_high = gate > 0.0;
        let rising_edge = gate_high && !self.gate_was_high;
        self.gate_was_high = gate_high;

        let attack_rate = 1.0 / (100.0 * self.smoothed_attack + 0.01);
        let decay_rate = 1.0 / (100.0 * self.smoothed_decay + 0.01);

        if self.stage == EnvelopeStage::Idle {
            if rising_edge {
                self.stage = EnvelopeStage::Attack;
                self.scale = clamp(0.0, 1.0, gate / 5.0);
            }
            self.rate = decay_rate;
            self.target = 0.0;
        }

        if self.stage == EnvelopeStage::Attack {
            if !gate_high || self.current_value > 1024.0 {
                self.stage = EnvelopeStage::Idle;
            }
            self.rate = attack_rate;
            self.target = 1.2 * 1024.0;
        }

        let rate_scale = 48000.0 / sample_rate.max(1.0);
        let step = (self.target - self.current_value) * self.rate * 0.004 * rate_scale;
        self.current_value += step;

        let normalized = clamp(0.0, 1.0, self.current_value / 1024.0);
        self.sample = clamp(0.0, 5.0, normalized * self.scale * 5.0);
    }
}
