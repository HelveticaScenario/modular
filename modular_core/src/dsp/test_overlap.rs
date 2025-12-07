// Test module to verify that overlapping parameter and output names
// produce a runtime panic when the schema is created.

use anyhow::{anyhow, Result};
use crate::types::InternalParam;

#[derive(Default, Params)]
struct TestOverlapParams {
    #[param("output", "this conflicts with the output name")]
    output: InternalParam,
}

#[derive(Default, Module)]
#[module("test-overlap", "Test module with overlapping names")]
pub struct TestOverlap {
    #[output("output", "this conflicts with the param name", default)]
    output: f32,
    params: TestOverlapParams,
}

impl TestOverlap {
    fn update(&mut self, _sample_rate: f32) {
        self.output = 1.0;
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

