use std::collections::HashMap;

use crate::types::{
    ChannelCountDeriver, Module, ModuleSchema, ParamsValidator, SampleableConstructor,
};

pub mod p_pulse;
pub mod p_saw;
pub mod p_sine;
pub mod noise;
pub mod plaits;
pub mod pulse;
pub mod saw;
pub mod sine;

pub fn install_constructors(map: &mut HashMap<String, SampleableConstructor>) {
    sine::SineOscillator::install_constructor(map);
    saw::SawOscillator::install_constructor(map);
    pulse::PulseOscillator::install_constructor(map);
    p_sine::PSineOscillator::install_constructor(map);
    p_saw::PSawOscillator::install_constructor(map);
    p_pulse::PPulseOscillator::install_constructor(map);
    noise::Noise::install_constructor(map);
    plaits::Plaits::install_constructor(map);
}

pub fn install_param_validators(map: &mut HashMap<String, ParamsValidator>) {
    sine::SineOscillator::install_params_validator(map);
    saw::SawOscillator::install_params_validator(map);
    pulse::PulseOscillator::install_params_validator(map);
    p_sine::PSineOscillator::install_params_validator(map);
    p_saw::PSawOscillator::install_params_validator(map);
    p_pulse::PPulseOscillator::install_params_validator(map);
    noise::Noise::install_params_validator(map);

    plaits::Plaits::install_params_validator(map);
}

pub fn install_channel_count_derivers(map: &mut HashMap<String, ChannelCountDeriver>) {
    sine::SineOscillator::install_channel_count_deriver(map);
    saw::SawOscillator::install_channel_count_deriver(map);
    pulse::PulseOscillator::install_channel_count_deriver(map);
    p_sine::PSineOscillator::install_channel_count_deriver(map);
    p_saw::PSawOscillator::install_channel_count_deriver(map);
    p_pulse::PPulseOscillator::install_channel_count_deriver(map);
    noise::Noise::install_channel_count_deriver(map);
    plaits::Plaits::install_channel_count_deriver(map);
}

pub fn schemas() -> Vec<ModuleSchema> {
    vec![
        sine::SineOscillator::get_schema(),
        saw::SawOscillator::get_schema(),
        pulse::PulseOscillator::get_schema(),
        p_sine::PSineOscillator::get_schema(),
        p_saw::PSawOscillator::get_schema(),
        p_pulse::PPulseOscillator::get_schema(),
        noise::Noise::get_schema(),
        plaits::Plaits::get_schema(),
    ]
}
