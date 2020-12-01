use std::collections::HashMap;

use crate::types::{ModuleSchema, SampleableConstructor};

pub mod sine;
pub mod ramp;

pub fn install_constructors(map: &mut HashMap<String, SampleableConstructor>) {
    sine::install_constructor(map);
    ramp::install_constructor(map);
}

pub fn schemas() -> Vec<&'static ModuleSchema> {
    vec![
        &sine::SCHEMA,
        &ramp::SCHEMA
    ]
}