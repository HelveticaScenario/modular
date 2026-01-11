use crate::types::{Clickless, Signal};
use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, Default, JsonSchema, Connect)]
#[serde(default)]
struct PercussionEnvelopeParams {
    /// trigger input (rising edge triggers envelope)
    trigger: Signal,
    /// decay time in seconds
    decay: Signal,
}

#[derive(Outputs, JsonSchema)]
struct PercussionEnvelopeOutputs {
    #[output("output", "envelope output", default)]
    sample: f32,
}

#[derive(Module)]
#[module("percEnv", "Percussion envelope with exponential decay")]
#[args(trigger)]
pub struct PercussionEnvelope {
    outputs: PercussionEnvelopeOutputs,
    params: PercussionEnvelopeParams,
    current_level: f32,
    last_trigger: f32,
    decay: Clickless,
}

impl Default for PercussionEnvelope {
    fn default() -> Self {
        Self {
            outputs: PercussionEnvelopeOutputs { sample: 0.0 },
            params: PercussionEnvelopeParams::default(),
            current_level: 0.0,
            last_trigger: 0.0,
            decay: 0.1.into(),
        }
    }
}

impl PercussionEnvelope {
    fn update(&mut self, sample_rate: f32) {
        // Smooth decay parameter (in seconds)
        self.decay
            .update(self.params.decay.get_value_or(0.1).max(0.001));

        let decay_time = *self.decay;

        // Detect rising edge of trigger
        let trigger = self.params.trigger.get_value();
        if trigger > 2.5 && self.last_trigger <= 2.5 {
            // Trigger detected - reset envelope to peak
            self.current_level = 1.0;
        }
        self.last_trigger = trigger;

        // Exponential decay
        if self.current_level > 0.00001 {
            // Calculate decay coefficient for exponential decay
            // Time constant tau = decay_time, we want level to reach ~0.001 after decay_time
            // Using e^(-t/tau) where tau = decay_time / 6.9 (ln(1000) ≈ 6.9)
            let tau = decay_time / 6.9;
            let decay_coeff = (-1.0 / (tau * sample_rate)).exp();
            self.current_level *= decay_coeff;
        } else {
            self.current_level = 0.0;
        }

        // Output 0-5V
        self.outputs.sample = (self.current_level * 5.0).clamp(0.0, 5.0);
    }
}

message_handlers!(impl PercussionEnvelope {});
