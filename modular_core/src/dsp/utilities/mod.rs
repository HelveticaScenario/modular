use std::collections::HashMap;

use crate::types::{Module, ModuleSchema, SampleableConstructor};

pub mod adsr;
pub mod ad;
pub mod clock;

pub fn install_constructors(map: &mut HashMap<String, SampleableConstructor>) {
    ad::Ad::install_constructor(map);
    adsr::Adsr::install_constructor(map);
    clock::Clock::install_constructor(map);
}

pub fn schemas() -> Vec<ModuleSchema> {
    vec![ad::Ad::get_schema(), adsr::Adsr::get_schema(), clock::Clock::get_schema()]
}
