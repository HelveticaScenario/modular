use std::collections::HashMap;

use crate::types::SampleableConstructor;

pub mod sine;

pub fn install_constructors(map: &mut HashMap<String, SampleableConstructor>) {
    sine::install_constructor(map);
}