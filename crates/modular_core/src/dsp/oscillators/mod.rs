use std::collections::HashMap;

use crate::types::{Module, ModuleSchema, ParamsValidator, SampleableConstructor};

pub mod d_pulse;
pub mod d_saw;
pub mod d_sine;
pub mod mi;
pub mod noise;
pub mod pulse;
pub mod saw;
pub mod sine;

pub fn install_constructors(map: &mut HashMap<String, SampleableConstructor>) {
    sine::SineOscillator::install_constructor(map);
    saw::SawOscillator::install_constructor(map);
    pulse::PulseOscillator::install_constructor(map);
    d_sine::DSineOscillator::install_constructor(map);
    d_saw::DSawOscillator::install_constructor(map);
    d_pulse::DPulseOscillator::install_constructor(map);
    noise::Noise::install_constructor(map);
    mi::install_constructors(map);
}

pub fn install_param_validators(map: &mut HashMap<String, ParamsValidator>) {
    sine::SineOscillator::install_params_validator(map);
    saw::SawOscillator::install_params_validator(map);
    pulse::PulseOscillator::install_params_validator(map);
    d_sine::DSineOscillator::install_params_validator(map);
    d_saw::DSawOscillator::install_params_validator(map);
    d_pulse::DPulseOscillator::install_params_validator(map);
    noise::Noise::install_params_validator(map);

    mi::install_param_validators(map);
}

pub fn schemas() -> Vec<ModuleSchema> {
    vec![
        vec![
            sine::SineOscillator::get_schema(),
            saw::SawOscillator::get_schema(),
            pulse::PulseOscillator::get_schema(),
            d_sine::DSineOscillator::get_schema(),
            d_saw::DSawOscillator::get_schema(),
            d_pulse::DPulseOscillator::get_schema(),
            noise::Noise::get_schema(),
        ],
        mi::schemas(),
    ]
    .into_iter()
    .flatten()
    .collect()
}
