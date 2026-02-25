use crate::params::ParamsDeserializer;
use crate::types::{Module, ModuleSchema, ParamsValidator, SampleableConstructor};
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

pub fn install_params_deserializers(map: &mut HashMap<String, ParamsDeserializer>) {
    lowpass::LowpassFilter::install_params_deserializer(map);
    highpass::HighpassFilter::install_params_deserializer(map);
    bandpass::BandpassFilter::install_params_deserializer(map);
}

pub fn schemas() -> Vec<ModuleSchema> {
    vec![
        lowpass::LowpassFilter::get_schema(),
        highpass::HighpassFilter::get_schema(),
        bandpass::BandpassFilter::get_schema(),
    ]
}
