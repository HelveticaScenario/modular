use crate::{
    dsp::utils::clamp,
    types::{ChannelBuffer, InternalParam, NUM_CHANNELS, smooth_value},
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
    sample: ChannelBuffer,
    stage: [EnvelopeStage; NUM_CHANNELS],
    current_value: ChannelBuffer,
    target: ChannelBuffer,
    rate: ChannelBuffer,
    scale: ChannelBuffer,
    gate_was_high: [bool; NUM_CHANNELS],
    smoothed_attack: ChannelBuffer,
    smoothed_decay: ChannelBuffer,
    params: AdParams,
}

impl Default for Ad {
    fn default() -> Self {
        Self {
            sample: ChannelBuffer::default(),
            stage: [EnvelopeStage::Idle; NUM_CHANNELS],
            current_value: ChannelBuffer::default(),
            target: ChannelBuffer::default(),
            rate: ChannelBuffer::default(),
            scale: ChannelBuffer::default(),
            gate_was_high: [false; NUM_CHANNELS],
            smoothed_attack: [0.01; NUM_CHANNELS],
            smoothed_decay: [0.1; NUM_CHANNELS],
            params: AdParams::default(),
        }
    }
}

impl Ad {
    fn update(&mut self, sample_rate: f32) -> () {
        let mut target_attack = [0.01; NUM_CHANNELS];
        let mut target_decay = [0.1; NUM_CHANNELS];
        let mut gate = ChannelBuffer::default();

        self.params
            .attack
            .get_value_or(&mut target_attack, &[0.01; NUM_CHANNELS]);
        self.params
            .decay
            .get_value_or(&mut target_decay, &[0.1; NUM_CHANNELS]);
        self.params.gate.get_value(&mut gate);

        for i in 0..NUM_CHANNELS {
            target_attack[i] = clamp(0.0, 10.0, target_attack[i]);
            target_decay[i] = clamp(0.0, 10.0, target_decay[i]);
            self.smoothed_attack[i] = smooth_value(self.smoothed_attack[i], target_attack[i]);
            self.smoothed_decay[i] = smooth_value(self.smoothed_decay[i], target_decay[i]);

            let gate_high = gate[i] > 2.5;
            let rising_edge = gate_high && !self.gate_was_high[i];
            self.gate_was_high[i] = gate_high;

            let attack_rate = 1.0 / (100.0 * self.smoothed_attack[i] + 0.01);
            let decay_rate = 1.0 / (100.0 * self.smoothed_decay[i] + 0.01);

            if self.stage[i] == EnvelopeStage::Idle {
                if rising_edge {
                    self.stage[i] = EnvelopeStage::Attack;
                    self.scale[i] = clamp(0.0, 1.0, gate[i] / 5.0);
                }
                self.rate[i] = decay_rate;
                self.target[i] = 0.0;
            }

            if self.stage[i] == EnvelopeStage::Attack {
                if !gate_high || self.current_value[i] > 1024.0 {
                    self.stage[i] = EnvelopeStage::Idle;
                }
                self.rate[i] = attack_rate;
                self.target[i] = 1.2 * 1024.0;
            }

            let rate_scale = 48000.0 / sample_rate.max(1.0);
            let step = (self.target[i] - self.current_value[i]) * self.rate[i] * 0.004 * rate_scale;
            self.current_value[i] += step;

            let normalized = clamp(0.0, 1.0, self.current_value[i] / 1024.0);
            self.sample[i] = clamp(0.0, 5.0, normalized * self.scale[i] * 5.0);
        }
    }
}
