use std::collections::HashMap;
use crate::types::{Module, ModuleSchema, SampleableConstructor};

pub mod lowpass;
pub mod highpass;
pub mod bandpass;
pub mod notch;
pub mod allpass;
pub mod state_variable;
pub mod moog_ladder;
pub mod tb303;
pub mod sem;
pub mod ms20;
pub mod formant;
pub mod sallen_key;

pub fn install_constructors(map: &mut HashMap<String, SampleableConstructor>) {
    lowpass::LowpassFilter::install_constructor(map);
    highpass::HighpassFilter::install_constructor(map);
    bandpass::BandpassFilter::install_constructor(map);
    notch::NotchFilter::install_constructor(map);
    allpass::AllpassFilter::install_constructor(map);
    state_variable::StateVariableFilter::install_constructor(map);
    moog_ladder::MoogLadderFilter::install_constructor(map);
    tb303::TB303Filter::install_constructor(map);
    sem::SEMFilter::install_constructor(map);
    ms20::MS20Filter::install_constructor(map);
    formant::FormantFilter::install_constructor(map);
    sallen_key::SallenKeyFilter::install_constructor(map);
}

pub fn schemas() -> Vec<ModuleSchema> {
    vec![
        lowpass::LowpassFilter::get_schema(),
        highpass::HighpassFilter::get_schema(),
        bandpass::BandpassFilter::get_schema(),
        notch::NotchFilter::get_schema(),
        allpass::AllpassFilter::get_schema(),
        state_variable::StateVariableFilter::get_schema(),
        moog_ladder::MoogLadderFilter::get_schema(),
        tb303::TB303Filter::get_schema(),
        sem::SEMFilter::get_schema(),
        ms20::MS20Filter::get_schema(),
        formant::FormantFilter::get_schema(),
        sallen_key::SallenKeyFilter::get_schema(),
    ]
}
