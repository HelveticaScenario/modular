use crate::{
    poly::{PolyOutput, PolySignal},
    types::{Clickless, Signal},
    PORT_MAX_CHANNELS,
};
use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default)]
struct StereoMixerParams {
    /// Polyphonic input signal to mix down to stereo
    input: PolySignal,
    /// Pan position for each channel (-5 = left, 0 = center, +5 = right).
    /// Cycles across input channels if fewer pan channels are provided.
    pan: PolySignal,
    /// Stereo width (0 = no spread, 5 = full spread across voices).
    /// Voices are auto-panned relative to their base pan position.
    width: Signal,
}

#[derive(Outputs, JsonSchema)]
struct StereoMixerOutputs {
    /// Stereo output (channel 0 = left, channel 1 = right)
    #[output("output", "stereo output", default, range = (-5.0, 5.0))]
    sample: PolyOutput,
}

#[derive(Default, Clone, Copy)]
struct ChannelState {
    pan: Clickless,
}

#[derive(Module)]
#[module("stereoMixer", "Mix polyphonic signal to stereo", channels = 2)]
#[args(input)]
pub struct StereoMixer {
    outputs: StereoMixerOutputs,
    params: StereoMixerParams,
    channel_state: [ChannelState; PORT_MAX_CHANNELS],
}

impl Default for StereoMixer {
    fn default() -> Self {
        Self {
            outputs: Default::default(),
            params: Default::default(),
            channel_state: [ChannelState::default(); PORT_MAX_CHANNELS],
        }
    }
}

impl StereoMixer {
    pub fn update(&mut self, _sample_rate: f32) {
        let input_channels = self.params.input.channels() as usize;

        // Output is always 2 channels (stereo)
        self.outputs.sample.set_channels(2);

        // Width: 0 = no spread, 5 = full ±5V spread across voices
        let width = self.params.width.get_value_or(0.0).clamp(0.0, 5.0);

        let mut left_sum = 0.0f32;
        let mut right_sum = 0.0f32;

        for ch in 0..input_channels {
            let input = self.params.input.get_value(ch);

            // Base pan from cycling PolySignal (-5 to +5 range, 0 = center)
            let base_pan = self.params.pan.get_value(ch).clamp(-5.0, 5.0);

            // Calculate width spread offset:
            // Voices spread from -width to +width relative to base pan
            // Voice 0 -> -width, last voice -> +width
            let spread_offset = if input_channels > 1 {
                let voice_pos = ch as f32 / (input_channels - 1) as f32; // 0.0 to 1.0
                (voice_pos - 0.5) * 2.0 * width // -width to +width
            } else {
                0.0 // Single voice stays at base pan
            };

            // Final pan position, clamped to valid range
            let final_pan = (base_pan + spread_offset).clamp(-5.0, 5.0);

            // Smooth pan changes to avoid clicks
            self.channel_state[ch].pan.update(final_pan);
            let pan = *self.channel_state[ch].pan;

            // Convert -5..+5 to 0..1 (0 = full left, 1 = full right)
            let pan_norm = (pan + 5.0) / 10.0;

            // Equal power panning
            let left_gain = (1.0 - pan_norm).sqrt();
            let right_gain = pan_norm.sqrt();

            left_sum += input * left_gain;
            right_sum += input * right_gain;
        }

        self.outputs.sample.set(0, left_sum); // Left
        self.outputs.sample.set(1, right_sum); // Right
    }
}

message_handlers!(impl StereoMixer {});
