//! Effects (FX) modules category.
//!
//! Contains waveshaping and distortion effects adapted from
//! the 4ms Ensemble Oscillator warp and twist modes.
//! Copyright 4ms Company. Used under GPL v3.

use std::collections::HashMap;

use crate::types::{
    ChannelCountDeriver, Module, ModuleSchema, ParamsValidator, SampleableConstructor,
};

pub mod enosc_tables;

pub mod cheby;
pub mod fold;
pub mod segment;

pub fn install_constructors(map: &mut HashMap<String, SampleableConstructor>) {
    fold::Fold::install_constructor(map);
    cheby::Cheby::install_constructor(map);
    segment::Segment::install_constructor(map);
}

pub fn install_param_validators(map: &mut HashMap<String, ParamsValidator>) {
    fold::Fold::install_params_validator(map);
    cheby::Cheby::install_params_validator(map);
    segment::Segment::install_params_validator(map);
}

pub fn install_channel_count_derivers(map: &mut HashMap<String, ChannelCountDeriver>) {
    fold::Fold::install_channel_count_deriver(map);
    cheby::Cheby::install_channel_count_deriver(map);
    segment::Segment::install_channel_count_deriver(map);
}

pub fn schemas() -> Vec<ModuleSchema> {
    vec![
        fold::Fold::get_schema(),
        cheby::Cheby::get_schema(),
        segment::Segment::get_schema(),
    ]
}
