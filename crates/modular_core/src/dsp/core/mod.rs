use std::collections::HashMap;

use crate::types::{
    ChannelCountDeriver, Module, ModuleSchema, ParamsValidator, SampleableConstructor,
};

pub mod audio_in;
pub mod clock;
pub mod mix;
pub mod signal;
pub mod stereo_mixer;

pub fn install_constructors(map: &mut HashMap<String, SampleableConstructor>) {
    signal::Signal::install_constructor(map);
    mix::Mix::install_constructor(map);
    stereo_mixer::StereoMixer::install_constructor(map);
    clock::Clock::install_constructor(map);
}

pub fn install_param_validators(map: &mut HashMap<String, ParamsValidator>) {
    signal::Signal::install_params_validator(map);
    mix::Mix::install_params_validator(map);
    stereo_mixer::StereoMixer::install_params_validator(map);
    clock::Clock::install_params_validator(map);
}

pub fn install_channel_count_derivers(map: &mut HashMap<String, ChannelCountDeriver>) {
    signal::Signal::install_channel_count_deriver(map);
    mix::Mix::install_channel_count_deriver(map);
    stereo_mixer::StereoMixer::install_channel_count_deriver(map);
    clock::Clock::install_channel_count_deriver(map);
}

pub fn schemas() -> Vec<ModuleSchema> {
    vec![
        signal::Signal::get_schema(),
        mix::Mix::get_schema(),
        stereo_mixer::StereoMixer::get_schema(),
        clock::Clock::get_schema(),
    ]
}
