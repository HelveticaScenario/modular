use std::collections::HashMap;

use crate::types::{Module, ModuleSchema, ParamsValidator, SampleableConstructor};

pub mod mix;
pub mod poly_mix;
pub mod scale_and_shift;
pub mod signal;
pub mod sum;
pub mod track;

pub fn install_constructors(map: &mut HashMap<String, SampleableConstructor>) {
    signal::Signal::install_constructor(map);
    scale_and_shift::ScaleAndShift::install_constructor(map);
    sum::Sum::install_constructor(map);
    mix::Mix::install_constructor(map);
    track::Track::install_constructor(map);
    poly_mix::PolyMix::install_constructor(map);
}

pub fn install_param_validators(map: &mut HashMap<String, ParamsValidator>) {
    signal::Signal::install_params_validator(map);
    scale_and_shift::ScaleAndShift::install_params_validator(map);
    sum::Sum::install_params_validator(map);
    mix::Mix::install_params_validator(map);
    track::Track::install_params_validator(map);
    poly_mix::PolyMix::install_params_validator(map);
}

pub fn schemas() -> Vec<ModuleSchema> {
    vec![
        signal::Signal::get_schema(),
        scale_and_shift::ScaleAndShift::get_schema(),
        sum::Sum::get_schema(),
        mix::Mix::get_schema(),
        track::Track::get_schema(),
        poly_mix::PolyMix::get_schema(),
    ]
}
