use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::poly::MonoSignal;
use crate::types::ClockMessages;

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default, rename_all = "camelCase")]
struct ClockDividerParams {
    /// division factor (e.g. 2 = output fires every other tick)
    pub division: u32,
    /// clock signal to divide
    pub input: MonoSignal,
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ClockDividerOutputs {
    #[output("output", "divided clock output", default)]
    pub output: f32,
}

/// Divides an incoming clock signal so it fires less often.
///
/// Feed it a clock and set **division** to an integer — the output will
/// tick once every *n* input ticks. Useful for creating slower rhythmic
/// subdivisions from a master clock.
///
/// ```js
/// // Pulses every other bar of the root clock:
/// $clockDivider($rootClock.barTrigger, 2)
/// ```
#[module(name = "$clockDivider", args(input, division))]
#[derive(Default)]
pub struct ClockDivider {
    params: ClockDividerParams,
    outputs: ClockDividerOutputs,
    counter: u32,
}

message_handlers!(impl ClockDivider {
    Clock(m) => ClockDivider::on_clock_message,
});

impl ClockDivider {
    fn on_clock_message(&mut self, m: &ClockMessages) -> Result<()> {
        match m {
            ClockMessages::Start => {
                // Reset counter on start
                self.counter = 0;
            }
            ClockMessages::Stop => {
                // No special handling needed on stop
            }
        }
        Ok(())
    }

    fn update(&mut self, _sample_rate: f32) {
        if self.params.input.get_value() > 0.0 {
            self.counter += 1;
            if self.counter >= self.params.division.max(1) {
                self.outputs.output = 5.0; // Trigger output
                self.counter = 0;
            } else {
                self.outputs.output = 0.0;
            }
        } else {
            self.outputs.output = 0.0;
        }
    }
}
