use std::collections::HashMap;

use crate::types::SampleableConstructor;

pub mod core;
pub mod oscillators;
pub mod utils;
pub mod consts;


pub fn get_constructors() -> HashMap<String, SampleableConstructor> {
    let mut map = HashMap::new();
    oscillators::install_constructors(&mut map);
    return map;
}