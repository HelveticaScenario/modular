use std::collections::HashMap;

use crate::types::{ChannelCountDeriver, Module, ModuleSchema, ParamsValidator, SampleableConstructor};

pub mod adsr;
pub mod clock;
pub mod clock_divider;
pub mod lag;
pub mod logic;
pub mod math;
pub mod percussion_envelope;
pub mod quantizer;
pub mod remap;
pub mod sample_and_hold;
pub mod scale;
pub mod stereo_mixer;

// Re-export useful types
pub use crate::dsp::utils::SchmittTrigger;
pub use scale::{FixedRoot, ScaleSnapper, validate_scale_type};

pub fn install_constructors(map: &mut HashMap<String, SampleableConstructor>) {
    adsr::Adsr::install_constructor(map);
    clock::Clock::install_constructor(map);
    clock_divider::ClockDivider::install_constructor(map);
    lag::LagProcessor::install_constructor(map);
    logic::RisingEdgeDetector::install_constructor(map);
    logic::FallingEdgeDetector::install_constructor(map);
    math::Math::install_constructor(map);
    remap::Remap::install_constructor(map);
    sample_and_hold::SampleAndHold::install_constructor(map);
    sample_and_hold::TrackAndHold::install_constructor(map);
    percussion_envelope::PercussionEnvelope::install_constructor(map);
    quantizer::Quantizer::install_constructor(map);
    stereo_mixer::StereoMixer::install_constructor(map);
}

pub fn install_param_validators(map: &mut HashMap<String, ParamsValidator>) {
    adsr::Adsr::install_params_validator(map);
    clock::Clock::install_params_validator(map);
    clock_divider::ClockDivider::install_params_validator(map);
    lag::LagProcessor::install_params_validator(map);
    logic::RisingEdgeDetector::install_params_validator(map);
    logic::FallingEdgeDetector::install_params_validator(map);
    math::Math::install_params_validator(map);
    remap::Remap::install_params_validator(map);
    sample_and_hold::SampleAndHold::install_params_validator(map);
    sample_and_hold::TrackAndHold::install_params_validator(map);
    percussion_envelope::PercussionEnvelope::install_params_validator(map);
    quantizer::Quantizer::install_params_validator(map);
    stereo_mixer::StereoMixer::install_params_validator(map);
}

pub fn install_channel_count_derivers(map: &mut HashMap<String, ChannelCountDeriver>) {
    adsr::Adsr::install_channel_count_deriver(map);
    clock::Clock::install_channel_count_deriver(map);
    clock_divider::ClockDivider::install_channel_count_deriver(map);
    lag::LagProcessor::install_channel_count_deriver(map);
    logic::RisingEdgeDetector::install_channel_count_deriver(map);
    logic::FallingEdgeDetector::install_channel_count_deriver(map);
    math::Math::install_channel_count_deriver(map);
    remap::Remap::install_channel_count_deriver(map);
    sample_and_hold::SampleAndHold::install_channel_count_deriver(map);
    sample_and_hold::TrackAndHold::install_channel_count_deriver(map);
    percussion_envelope::PercussionEnvelope::install_channel_count_deriver(map);
    quantizer::Quantizer::install_channel_count_deriver(map);
    stereo_mixer::StereoMixer::install_channel_count_deriver(map);
}

pub fn schemas() -> Vec<ModuleSchema> {
    vec![
        adsr::Adsr::get_schema(),
        clock::Clock::get_schema(),
        clock_divider::ClockDivider::get_schema(),
        lag::LagProcessor::get_schema(),
        logic::RisingEdgeDetector::get_schema(),
        logic::FallingEdgeDetector::get_schema(),
        math::Math::get_schema(),
        remap::Remap::get_schema(),
        sample_and_hold::SampleAndHold::get_schema(),
        sample_and_hold::TrackAndHold::get_schema(),
        percussion_envelope::PercussionEnvelope::get_schema(),
        quantizer::Quantizer::get_schema(),
        stereo_mixer::StereoMixer::get_schema(),
    ]
}
