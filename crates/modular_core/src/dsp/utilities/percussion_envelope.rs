use crate::poly::{PolyOutput, PolySignal, PORT_MAX_CHANNELS};
use crate::types::Clickless;
use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, Default, JsonSchema, Connect)]
#[serde(default)]
struct PercussionEnvelopeParams {
    /// trigger input (rising edge triggers envelope)
    trigger: PolySignal,
    /// decay time in seconds
    decay: PolySignal,
    range: (PolySignal, PolySignal),
}

#[derive(Outputs, JsonSchema)]
struct PercussionEnvelopeOutputs {
    #[output("output", "envelope output", default)]
    sample: PolyOutput,
}

/// Per-channel envelope state
#[derive(Default, Clone, Copy)]
struct ChannelState {
    current_level: f32,
    last_trigger: f32,
    decay: Clickless,
}

#[derive(Module)]
#[module("perc", "Percussion envelope with exponential decay")]
#[args(trigger)]
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
            channels: std::array::from_fn(|_| ChannelState {
                current_level: 0.0,
                last_trigger: 0.0,
                decay: 0.1.into(),
            }),
        }
    }
}

impl PercussionEnvelope {
    fn update(&mut self, sample_rate: f32) {
        // Determine channel count from trigger input
        let num_channels = self.params.trigger.channels().max(1) as usize;

        let mut output = PolyOutput::default();
        output.set_channels(num_channels as u8);

        for ch in 0..num_channels {
            let state = &mut self.channels[ch];

            // Smooth decay parameter (in seconds)
            state
                .decay
                .update(self.params.decay.get_value_or(ch, 0.1).max(0.001));

            let decay_time = *state.decay;

            // Detect rising edge of trigger
            let trigger = self.params.trigger.get_value(ch);
            if trigger > 2.5 && state.last_trigger <= 2.5 {
                // Trigger detected - reset envelope to peak
                state.current_level = 1.0;
            }
            state.last_trigger = trigger;

            // Exponential decay
            if state.current_level > 0.00001 {
                // Calculate decay coefficient for exponential decay
                // Time constant tau = decay_time, we want level to reach ~0.001 after decay_time
                // Using e^(-t/tau) where tau = decay_time / 6.9 (ln(1000) ≈ 6.9)
                let tau = decay_time / 6.9;
                let decay_coeff = (-1.0 / (tau * sample_rate)).exp();
                state.current_level *= decay_coeff;
            } else {
                state.current_level = 0.0;
            }

            // Output 0-5V
            let min = self.params.range.0.get_value_or(ch, 0.0);
            let max = self.params.range.1.get_value_or(ch, 5.0);
            output.set(
                ch,
                crate::dsp::utils::map_range(state.current_level, 0.0, 1.0, min, max),
            );
        }

        self.outputs.sample = output;
    }
}

message_handlers!(impl PercussionEnvelope {});
