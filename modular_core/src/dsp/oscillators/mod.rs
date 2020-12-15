use std::collections::HashMap;

use crate::types::{Module, ModuleSchema, SampleableConstructor};

pub mod ramp;
pub mod sine;

pub fn install_constructors(map: &mut HashMap<String, SampleableConstructor>) {
    sine::SineOscillator::install_constructor(map);
    ramp::RampOscillator::install_constructor(map);
}

pub fn schemas() -> Vec<ModuleSchema> {
    vec![
        sine::SineOscillator::get_schema(),
        ramp::RampOscillator::get_schema(),
    ]
}
