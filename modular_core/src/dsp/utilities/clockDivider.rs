use anyhow::{Result, anyhow};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::types::{ClockMessages, Signal};

#[derive(Deserialize, Default, JsonSchema, Connect)]
#[serde(default)]
struct ClockDividerParams {
    pub division: u32,
    pub input: Signal,
}

#[derive(Outputs, JsonSchema)]
struct ClockDividerOutputs {
    #[output("output", "divided clock output", default)]
    pub output: f32,
}

#[derive(Default, Module)]
#[module(
    "clockDivider",
    "Divides an incoming clock signal by a specified integer value"
)]
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
