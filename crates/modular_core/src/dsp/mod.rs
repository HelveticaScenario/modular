use std::collections::HashMap;

use crate::types::{ChannelCountDeriver, ModuleSchema, ParamsValidator, SampleableConstructor};

pub mod consts;
pub mod core;
pub mod oscillators;
pub mod filters;
pub mod utilities;
pub mod utils;
pub mod seq;
pub mod midi;

// #[cfg(test)]
mod test_overlap;

pub fn get_constructors() -> HashMap<String, SampleableConstructor> {
    let mut map = HashMap::new();
    core::install_constructors(&mut map);
    oscillators::install_constructors(&mut map);
    filters::install_constructors(&mut map);
    utilities::install_constructors(&mut map);
    seq::install_constructors(&mut map);
    midi::install_constructors(&mut map);
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
    seq::install_param_validators(&mut map);
    midi::install_param_validators(&mut map);
    map
}

/// Returns a map of `module_type` -> channel count deriver function.
///
/// A channel count deriver derives the output channel count from a module's params JSON.
pub fn get_channel_count_derivers() -> HashMap<String, ChannelCountDeriver> {
    let mut map = HashMap::new();
    core::install_channel_count_derivers(&mut map);
    oscillators::install_channel_count_derivers(&mut map);
    filters::install_channel_count_derivers(&mut map);
    utilities::install_channel_count_derivers(&mut map);
    seq::install_channel_count_derivers(&mut map);
    midi::install_channel_count_derivers(&mut map);
    map
}

pub fn schema() -> Vec<ModuleSchema> {
    [
        core::schemas(),
        oscillators::schemas(),
        filters::schemas(),
        utilities::schemas(),
        seq::schemas(),
        midi::schemas(),
    ]
    .concat()
}
