//! Phase modules category.
//!
//! Contains phase generators and phase-distortion effects.
//! Phase modules operate on 0-to-1 phase signals rather than audio waveforms.

use std::collections::HashMap;

use crate::types::{
    ChannelCountDeriver, Module, ModuleSchema, ParamsValidator, SampleableConstructor,
};

pub mod crush;
pub mod feedback;
pub mod pulsar;
pub mod ramp;

pub fn install_constructors(map: &mut HashMap<String, SampleableConstructor>) {
    crush::Crush::install_constructor(map);
    feedback::Feedback::install_constructor(map);
    pulsar::Pulsar::install_constructor(map);
    ramp::Ramp::install_constructor(map);
}

pub fn install_param_validators(map: &mut HashMap<String, ParamsValidator>) {
    crush::Crush::install_params_validator(map);
    feedback::Feedback::install_params_validator(map);
    pulsar::Pulsar::install_params_validator(map);
    ramp::Ramp::install_params_validator(map);
}

pub fn install_channel_count_derivers(map: &mut HashMap<String, ChannelCountDeriver>) {
    crush::Crush::install_channel_count_deriver(map);
    feedback::Feedback::install_channel_count_deriver(map);
    pulsar::Pulsar::install_channel_count_deriver(map);
    ramp::Ramp::install_channel_count_deriver(map);
}

pub fn schemas() -> Vec<ModuleSchema> {
    vec![
        crush::Crush::get_schema(),
        feedback::Feedback::get_schema(),
        pulsar::Pulsar::get_schema(),
        ramp::Ramp::get_schema(),
    ]
}
