use crate::{
    poly::{PolyOutput, PolySignal},
    PORT_MAX_CHANNELS,
};
use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default)]
struct LagProcessorParams {
    input: PolySignal,
    /// rise time in seconds (default 0.01s)
    rise: PolySignal,
    /// fall time in seconds (default 0.01s)
    fall: PolySignal,
}

#[derive(Outputs, JsonSchema)]
struct LagProcessorOutputs {
    #[output("output", "output", default)]
    sample: PolyOutput,
}

#[derive(Default, Clone, Copy)]
struct SlewChannelState {
    current_value: f32,
}

#[derive(Module)]
#[module("slew", "Lag Processor (Slew Limiter)")]
#[args(input)]
pub struct LagProcessor {
    outputs: LagProcessorOutputs,
    params: LagProcessorParams,
    channels: [SlewChannelState; PORT_MAX_CHANNELS],
}

impl Default for LagProcessor {
    fn default() -> Self {
        Self {
            outputs: Default::default(),
            params: Default::default(),
            channels: [SlewChannelState::default(); PORT_MAX_CHANNELS],
        }
    }
}

impl LagProcessor {
    pub fn update(&mut self, sample_rate: f32) {
        let num_channels = self.channel_count();
        self.outputs.sample.set_channels(num_channels as u8);

        for ch in 0..num_channels {
            let state = &mut self.channels[ch];
            let input = self.params.input.get_value_or(ch, 0.0);

            let fall_time = self.params.fall.get_value_or(ch, 0.01).max(0.001);
            let rise_time = if self.params.rise.is_disconnected() {
                fall_time
            } else {
                self.params.rise.get_value_or(ch, 0.01).max(0.001)
            };

            // Calculate max change per sample
            // time is seconds for 10V change (full scale)
            // Slew rate = 10.0 / time (V/s)
            // Max delta per sample = Slew rate / sample_rate
            let max_rise = 10.0 / (rise_time * sample_rate);
            let max_fall = 10.0 / (fall_time * sample_rate);

            let diff = input - state.current_value;

            let change = if diff > 0.0 {
                diff.min(max_rise)
            } else {
                diff.max(-max_fall)
            };

            state.current_value += change;
            self.outputs.sample.set(ch, state.current_value);
        }
    }
}

message_handlers!(impl LagProcessor {});
