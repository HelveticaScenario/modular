use std::collections::HashMap;

use crate::types::{ModuleSchema, SampleableConstructor};

pub mod consts;
pub mod core;
pub mod oscillators;
pub mod utils;

pub fn get_constructors() -> HashMap<String, SampleableConstructor> {
    let mut map = HashMap::new();
    core::install_constructors(&mut map);
    oscillators::install_constructors(&mut map);
    return map;
}

pub fn schema() -> Vec<&'static ModuleSchema> {
    [core::schemas(), oscillators::schemas()].concat()
}
