use std::collections::HashMap;

use crate::types::{ModuleSchema, ParamsValidator, SampleableConstructor};

pub mod consts;
pub mod core;
pub mod oscillators;
pub mod filters;
pub mod utilities;
pub mod utils;

// #[cfg(test)]
mod test_overlap;

pub fn get_constructors() -> HashMap<String, SampleableConstructor> {
    let mut map = HashMap::new();
    core::install_constructors(&mut map);
    oscillators::install_constructors(&mut map);
    filters::install_constructors(&mut map);
    utilities::install_constructors(&mut map);
    return map;
}

/// Returns a map of `module_type` -> typed params validator.
///
/// A typed params validator attempts to deserialize a module's `ModuleState.params` JSON
/// into that module's concrete `*Params` struct.
pub fn get_param_validators() -> HashMap<String, ParamsValidator> {
    let mut map = HashMap::new();
    core::install_param_validators(&mut map);
    oscillators::install_param_validators(&mut map);
    filters::install_param_validators(&mut map);
    utilities::install_param_validators(&mut map);
    map
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
