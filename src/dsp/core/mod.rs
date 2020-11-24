use std::collections::HashMap;

use crate::types::SampleableConstructor;

pub mod signal_source;
pub mod signal_destination;


pub fn install_constructors(map: &mut HashMap<String, SampleableConstructor>) {
    signal_source::install_constructor(map);
    signal_destination::install_constructor(map);
}