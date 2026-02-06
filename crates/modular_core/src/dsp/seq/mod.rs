//! Sequencer modules for the modular synthesizer.
//!
//! This module provides:
//! - `Seq`: A Strudel/TidalCycles style sequencer using the new pattern system
//! - `IntervalSeq`: A scale-degree sequencer with two additive patterns

use std::collections::HashMap;

use crate::types::{
    ChannelCountDeriver, Module, ModuleSchema, ParamsValidator, SampleableConstructor,
};

pub mod interval_seq;
pub mod scale;
pub mod seq;
pub mod seq_value;
pub mod track;

pub use interval_seq::{IntervalPatternParam, IntervalSeq, IntervalValue};
pub use scale::{FixedRoot, ScaleRoot, ScaleSnapper};
pub use seq_value::{SeqPatternParam, SeqValue};

pub fn install_constructors(map: &mut HashMap<String, SampleableConstructor>) {
    seq::Seq::install_constructor(map);
    track::Track::install_constructor(map);
    interval_seq::IntervalSeq::install_constructor(map);
}

pub fn install_param_validators(map: &mut HashMap<String, ParamsValidator>) {
    seq::Seq::install_params_validator(map);
    track::Track::install_params_validator(map);
    interval_seq::IntervalSeq::install_params_validator(map);
}

pub fn install_channel_count_derivers(map: &mut HashMap<String, ChannelCountDeriver>) {
    seq::Seq::install_channel_count_deriver(map);
    track::Track::install_channel_count_deriver(map);
    interval_seq::IntervalSeq::install_channel_count_deriver(map);
}

pub fn schemas() -> Vec<ModuleSchema> {
    vec![
        seq::Seq::get_schema(),
        track::Track::get_schema(),
        interval_seq::IntervalSeq::get_schema(),
    ]
}
