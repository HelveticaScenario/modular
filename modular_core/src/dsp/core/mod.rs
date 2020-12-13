use std::collections::HashMap;

use crate::types::{ModuleSchema, SampleableConstructor, Module};

pub mod mix;
pub mod scale_and_shift;
pub mod signal;
pub mod sum;

pub fn install_constructors(map: &mut HashMap<String, SampleableConstructor>) {
    signal::Signal::install_constructor(map);
    scale_and_shift::ScaleAndShift::install_constructor(map);
    sum::Sum::install_constructor(map);
    mix::Mix::install_constructor(map);
}

pub fn schemas() -> Vec<ModuleSchema> {
    vec![
        signal::Signal::get_schema(),
        scale_and_shift::ScaleAndShift::get_schema(),
        sum::Sum::get_schema(),
        mix::Mix::get_schema(),
    ]
}
