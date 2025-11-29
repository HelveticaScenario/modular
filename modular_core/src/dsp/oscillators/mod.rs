use std::collections::HashMap;

use crate::types::{Module, ModuleSchema, SampleableConstructor};

pub mod sine;
pub mod saw;
pub mod pulse;

pub fn install_constructors(map: &mut HashMap<String, SampleableConstructor>) {
    sine::SineOscillator::install_constructor(map);
    saw::SawOscillator::install_constructor(map);
    pulse::PulseOscillator::install_constructor(map);
}

pub fn schemas() -> Vec<ModuleSchema> {
    vec![
        sine::SineOscillator::get_schema(),
        saw::SawOscillator::get_schema(),
        pulse::PulseOscillator::get_schema(),
    ]
}
