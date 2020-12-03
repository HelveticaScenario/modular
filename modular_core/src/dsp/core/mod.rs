use std::collections::HashMap;

use crate::types::{ModuleSchema, SampleableConstructor};

pub mod mix;
pub mod scale_and_shift;
pub mod signal;
pub mod sum;

pub fn install_constructors(map: &mut HashMap<String, SampleableConstructor>) {
    signal::install_constructor(map);
    scale_and_shift::install_constructor(map);
    sum::install_constructor(map);
    mix::install_constructor(map);
}

pub fn schemas() -> Vec<&'static ModuleSchema> {
    vec![
        &signal::SCHEMA,
        &scale_and_shift::SCHEMA,
        &sum::SCHEMA,
        &mix::SCHEMA,
    ]
}
