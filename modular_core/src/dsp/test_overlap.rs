// Test module to verify that overlapping parameter and output names
// produce a runtime panic when the schema is created.

use anyhow::{Result, anyhow};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::types::Signal;

#[derive(Default, Deserialize, JsonSchema, Connect)]
#[serde(default)]
struct TestOverlapParams {
    /// this conflicts with the output name
    output: Signal,
}

#[derive(Default, Module)]
#[module("test-overlap", "Test module with overlapping names")]
pub struct TestOverlap {
    outputs: TestOverlapOutputs,
    params: TestOverlapParams,
}

#[derive(Outputs, JsonSchema)]
struct TestOverlapOutputs {
    #[output("output", "this conflicts with the param name", default)]
    output: f32,
}

impl TestOverlap {
    fn update(&mut self, _sample_rate: f32) {
        self.outputs.output = 1.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Module;

    #[test]
    #[should_panic(expected = "Parameters and outputs must have unique names")]
    fn test_overlapping_names_panics() {
        // This should panic when get_schema() is called
        let _schema = TestOverlap::get_schema();
    }
}
