use std::collections::HashMap;

use crate::types::{ModuleSchema, SampleableConstructor};

pub mod consts;
pub mod core;
pub mod oscillators;
pub mod filters;
pub mod utilities;
pub mod utils;

#[cfg(test)]
mod test_overlap;

pub fn get_constructors() -> HashMap<String, SampleableConstructor> {
    let mut map = HashMap::new();
    core::install_constructors(&mut map);
    oscillators::install_constructors(&mut map);
    filters::install_constructors(&mut map);
    utilities::install_constructors(&mut map);
    return map;
}

pub fn schema() -> Vec<ModuleSchema> {
    [
        core::schemas(),
        oscillators::schemas(),
        filters::schemas(),
        utilities::schemas(),
    ]
    .concat()
}
