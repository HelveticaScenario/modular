use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::dsp::utils::SchmittTrigger;
use crate::poly::{PORT_MAX_CHANNELS, PolyOutput, PolySignal};
use crate::types::ClockMessages;

#[derive(Deserialize, Default, JsonSchema, Connect, ChannelCount)]
#[serde(default, rename_all = "camelCase")]
struct ClockDividerParams {
    /// division factor (e.g. 2 = output fires every other tick)
    pub division: u32,
    /// clock signal to divide
    pub input: PolySignal,
    /// trigger to reset the counter to 0
    pub reset: PolySignal,
}

#[derive(Outputs, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct ClockDividerOutputs {
    #[output("output", "divided clock output", default)]
    pub output: PolyOutput,
}

#[derive(Default, Clone, Copy)]
struct ChannelState {
    counter: u32,
    reset_schmitt: SchmittTrigger,
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
    channels: [ChannelState; PORT_MAX_CHANNELS],
}

message_handlers!(impl ClockDivider {
    Clock(m) => ClockDivider::on_clock_message,
});

impl ClockDivider {
    fn on_clock_message(&mut self, m: &ClockMessages) -> Result<()> {
        match m {
            ClockMessages::Start => {
                // Reset all channel counters on start
                for state in self.channels.iter_mut() {
                    state.counter = 0;
                }
            }
            ClockMessages::Stop => {
                // No special handling needed on stop
            }
        }
        Ok(())
    }

    fn update(&mut self, _sample_rate: f32) {
        let num_channels = self.channel_count();
        let division = self.params.division.max(1);

        for ch in 0..num_channels {
            let state = &mut self.channels[ch];

            // Reset counter on rising edge of reset trigger
            if state.reset_schmitt.process(self.params.reset.get_value(ch)) {
                state.counter = 0;
            }

            if self.params.input.get_value(ch) > 0.0 {
                state.counter += 1;
                if state.counter >= division {
                    self.outputs.output.set(ch, 5.0); // Trigger output
                    state.counter = 0;
                } else {
                    self.outputs.output.set(ch, 0.0);
                }
            } else {
                self.outputs.output.set(ch, 0.0);
            }
        }
    }
}
