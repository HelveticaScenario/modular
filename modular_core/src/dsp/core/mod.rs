use std::collections::HashMap;

use crate::types::SampleableConstructor;

pub mod signal;
pub mod scale_and_shift;
pub mod sum;
pub mod mix;


pub fn install_constructors(map: &mut HashMap<String, SampleableConstructor>) {
    signal::install_constructor(map);
    scale_and_shift::install_constructor(map);
    sum::install_constructor(map);
    mix::install_constructor(map);
}