use crate::types::{
    ChannelCountDeriver, Module, ModuleSchema, ParamsValidator, SampleableConstructor,
};
use std::collections::HashMap;

pub mod bandpass;
pub mod highpass;
pub mod lowpass;

pub fn install_constructors(map: &mut HashMap<String, SampleableConstructor>) {
    lowpass::LowpassFilter::install_constructor(map);
    highpass::HighpassFilter::install_constructor(map);
    bandpass::BandpassFilter::install_constructor(map);
}

pub fn install_param_validators(map: &mut HashMap<String, ParamsValidator>) {
    lowpass::LowpassFilter::install_params_validator(map);
    highpass::HighpassFilter::install_params_validator(map);
    bandpass::BandpassFilter::install_params_validator(map);
}

pub fn install_channel_count_derivers(map: &mut HashMap<String, ChannelCountDeriver>) {
    lowpass::LowpassFilter::install_channel_count_deriver(map);
    highpass::HighpassFilter::install_channel_count_deriver(map);
    bandpass::BandpassFilter::install_channel_count_deriver(map);
}

pub fn schemas() -> Vec<ModuleSchema> {
    vec![
        lowpass::LowpassFilter::get_schema(),
        highpass::HighpassFilter::get_schema(),
        bandpass::BandpassFilter::get_schema(),
    ]
}
