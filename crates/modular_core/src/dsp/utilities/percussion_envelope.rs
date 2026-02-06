use crate::dsp::utils::SchmittTrigger;
use crate::poly::{PORT_MAX_CHANNELS, PolyOutput, PolySignal};
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default, rename_all = "camelCase")]
struct PercussionEnvelopeParams {
    /// trigger input (rising edge triggers envelope)
    trigger: PolySignal,
    /// decay time in seconds
    decay: PolySignal,
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct PercussionEnvelopeOutputs {
    #[output("output", "envelope output", default, range = (0.0, 5.0))]
    sample: PolyOutput,
}

/// Per-channel envelope state
#[derive(Default, Clone, Copy)]
struct ChannelState {
    current_level: f32,
    trigger_schmitt: SchmittTrigger,
    in_attack: bool,
}

#[module(name = "env.perc", description = "Percussion envelope with exponential decay", args(trigger))]
pub struct PercussionEnvelope {
    outputs: PercussionEnvelopeOutputs,
    params: PercussionEnvelopeParams,
    channels: [ChannelState; PORT_MAX_CHANNELS],
}

impl Default for PercussionEnvelope {
    fn default() -> Self {
        Self {
            outputs: PercussionEnvelopeOutputs::default(),
            params: PercussionEnvelopeParams::default(),
            channels: std::array::from_fn(|_| ChannelState::default()),
        }
    }
}

impl PercussionEnvelope {
    fn update(&mut self, sample_rate: f32) {
        let num_channels = self.channel_count();

        self.outputs.sample.set_channels(num_channels);

        for ch in 0..num_channels {
            let state = &mut self.channels[ch];


            let decay_time = self.params.decay.get_value_or(ch, 0.1).max(0.001);

            // Detect rising edge of trigger using Schmitt trigger for noise immunity
            let trigger = self.params.trigger.get_value(ch);
            if state.trigger_schmitt.process(trigger) {
                // Trigger detected - start attack phase (continue from current level for smooth re-trigger)
                state.in_attack = true;
            }

            // Attack phase: 1ms linear ramp to peak
            if state.in_attack {
                const ATTACK_TIME: f32 = 0.001; // 1ms
                let step = 1.0 / (ATTACK_TIME * sample_rate);
                state.current_level += step;
                if state.current_level >= 1.0 {
                    state.current_level = 1.0;
                    state.in_attack = false;
                }
            } else if state.current_level > 0.00001 {
                // Exponential decay
                // Calculate decay coefficient for exponential decay
                // Time constant tau = decay_time, we want level to reach ~0.001 after decay_time
                // Using e^(-t/tau) where tau = decay_time / 6.9 (ln(1000) ≈ 6.9)
                let tau = decay_time / 6.9;
                let decay_coeff = (-1.0 / (tau * sample_rate)).exp();
                state.current_level *= decay_coeff;
            } else {
                state.current_level = 0.0;
            }

            self.outputs.sample.set(ch, state.current_level * 5.0);
        }
    }
}

message_handlers!(impl PercussionEnvelope {});
