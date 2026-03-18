//! Dynamics processing modules.
//!
//! Contains compressor, crossover, and other dynamics processing effects.

use std::collections::HashMap;

use crate::params::ParamsDeserializer;
use crate::types::{Module, ModuleSchema, SampleableConstructor};

pub mod compressor;
pub mod crossover;

pub fn install_constructors(map: &mut HashMap<String, SampleableConstructor>) {
    compressor::Compressor::install_constructor(map);
    crossover::Crossover::install_constructor(map);
}

pub fn install_params_deserializers(map: &mut HashMap<String, ParamsDeserializer>) {
    compressor::Compressor::install_params_deserializer(map);
    crossover::Crossover::install_params_deserializer(map);
}

pub fn schemas() -> Vec<ModuleSchema> {
    vec![
        compressor::Compressor::get_schema(),
        crossover::Crossover::get_schema(),
    ]
}
