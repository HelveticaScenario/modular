use crate::types::{Clickless, Signal};
use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default)]
struct LagProcessorParams {
    input: Signal,
    rise: Signal,
    fall: Signal,
}

#[derive(Outputs, JsonSchema)]
struct LagProcessorOutputs {
    #[output("output", "output", default)]
    sample: f32,
}

#[derive(Module)]
#[module("slew", "Lag Processor (Slew Limiter)")]
#[args(input)]
pub struct LagProcessor {
    outputs: LagProcessorOutputs,
    params: LagProcessorParams,
    current_value: f32,
    rise: Clickless,
    fall: Clickless,
}

impl Default for LagProcessor {
    fn default() -> Self {
        Self {
            outputs: LagProcessorOutputs { sample: 0.0 },
            params: Default::default(),
            current_value: 0.0,
            rise: 0.0.into(),
            fall: 0.0.into(),
        }
    }
}

impl LagProcessor {
    pub fn update(&mut self, sample_rate: f32) {
        let fall_val = self.params.fall.get_value();
        let rise_val = if self.params.rise.is_disconnected() { fall_val } else { self.params.rise.get_value() };

        self.rise.update(rise_val);
        self.fall.update(fall_val);

        let input = self.params.input.get_value();

        let rise_cv = *self.rise;
        let fall_cv = *self.fall;

        let rise_time = 55.0f32 * 2.0f32.powf(rise_cv);
        let fall_time = 55.0f32 * 2.0f32.powf(fall_cv);

        // Calculate max change per sample
        // Assuming time is seconds for 10V change (full scale)
        // Slew rate = 10.0 / time (V/s)
        // Max delta per sample = Slew rate / sample_rate

        let rise_rate = 10.0 / rise_time;
        let fall_rate = 10.0 / fall_time;

        let max_rise = rise_rate / sample_rate;
        let max_fall = fall_rate / sample_rate;

        let diff = input - self.current_value;

        let change = if diff > 0.0 {
            diff.min(max_rise)
        } else {
            diff.max(-max_fall)
        };

        self.current_value += change;
        self.outputs.sample = self.current_value;
    }
}

message_handlers!(impl LagProcessor {});
