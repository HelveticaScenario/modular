use crate::types::{Clickless, Signal};

use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, Default, JsonSchema, Connect)]
#[serde(default)]
struct AdParams {
    /// gate input (expects >0V for on)
    gate: Signal,
    /// attack time (approximate seconds)
    attack: Signal,
    /// decay time (approximate seconds)
    decay: Signal,
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
    outputs: AdOutputs,
    stage: EnvelopeStage,
    current_value: f32,
    target: f32,
    rate: f32,
    scale: f32,
    gate_was_high: bool,
    attack: Clickless,
    decay: Clickless,
    params: AdParams,
}

#[derive(Outputs, JsonSchema)]
struct AdOutputs {
    #[output("output", "envelope output", default)]
    sample: f32,
}

impl Default for Ad {
    fn default() -> Self {
        Self {
            outputs: AdOutputs::default(),
            stage: EnvelopeStage::Idle,
            current_value: 0.0,
            target: 0.0,
            rate: 0.0,
            scale: 0.0,
            gate_was_high: false,
            attack: 0.01.into(),
            decay: 0.1.into(),
            params: AdParams::default(),
        }
    }
}

impl Ad {
    fn update(&mut self, sample_rate: f32) -> () {
        self.attack
            .update(self.params.attack.get_value_or(0.01).clamp(0.0, 10.0));
        self.decay
            .update(self.params.decay.get_value_or(0.1).clamp(0.0, 10.0));

        let gate = self.params.gate.get_value_or(0.0);
        let gate_high = gate > 2.5;
        let rising_edge = gate_high && !self.gate_was_high;
        self.gate_was_high = gate_high;

        let attack_rate = 1.0 / (100.0 * *self.attack + 0.01);
        let decay_rate = 1.0 / (100.0 * *self.decay + 0.01);

        if self.stage == EnvelopeStage::Idle {
            if rising_edge {
                self.stage = EnvelopeStage::Attack;
                self.scale = (gate / 5.0).clamp(0.0, 1.0);
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

        let normalized = (self.current_value / 1024.0).clamp(0.0, 1.0);
        self.outputs.sample = (normalized * self.scale * 5.0).clamp(0.0, 5.0);
    }
}

message_handlers!(impl Ad {});
