use napi::Result;
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
    dsp::utils::{SchmittState, SchmittTrigger},
    types::Signal,
};

#[derive(Deserialize, Default, JsonSchema, Connect)]
#[serde(default)]
struct SchmittTriggerModuleParams {
    /// Input signal to process
    pub input: Signal,
    /// Low threshold - signal must fall below this to switch to low state
    pub low_threshold: Signal,
    /// High threshold - signal must rise above this to switch to high state
    pub high_threshold: Signal,
}

#[derive(Outputs, JsonSchema)]
struct SchmittTriggerModuleOutputs {
    #[output("output", "binary output (0V or 5V)", default)]
    pub output: f32,
}

#[derive(Default, Module)]
#[module(
    "schmittTrigger",
    "Schmitt trigger with hysteresis - outputs high (5V) when input rises above high threshold, outputs low (0V) when input falls below low threshold"
)]
pub struct SchmittTriggerModule {
    params: SchmittTriggerModuleParams,
    outputs: SchmittTriggerModuleOutputs,
    trigger: SchmittTrigger,
}

message_handlers!(impl SchmittTriggerModule {});

impl SchmittTriggerModule {
    fn update(&mut self, _sample_rate: f32) {
        let input = self.params.input.get_value_or(0.0);
        let low_threshold = self.params.low_threshold.get_value_or(-1.0);
        let high_threshold = self.params.high_threshold.get_value_or(1.0);

        self.trigger.set_thresholds(low_threshold, high_threshold);
        let state = self.trigger.process(input);

        match state {
            SchmittState::Low => self.outputs.output = 0.0,
            SchmittState::High => self.outputs.output = 5.0,
            SchmittState::Uninitialized => self.outputs.output = 0.0, // Default to low on uninitialized
        }
    }
}
