//! MIDI DSP modules for converting MIDI messages to control voltages.
//!
//! This module provides modules that read from the shared MIDI state
//! and output appropriate control voltages for use in the patch graph.

use std::collections::HashMap;

use crate::types::{ChannelCountDeriver, Module, ModuleSchema, ParamsValidator, SampleableConstructor};

pub mod midi_cv;
pub mod midi_cc;
pub mod midi_gate;

pub fn install_constructors(map: &mut HashMap<String, SampleableConstructor>) {
    midi_cv::MidiCv::install_constructor(map);
    midi_cc::MidiCc::install_constructor(map);
    midi_gate::MidiGate::install_constructor(map);
}

pub fn install_param_validators(map: &mut HashMap<String, ParamsValidator>) {
    midi_cv::MidiCv::install_params_validator(map);
    midi_cc::MidiCc::install_params_validator(map);
    midi_gate::MidiGate::install_params_validator(map);
}

pub fn install_channel_count_derivers(map: &mut HashMap<String, ChannelCountDeriver>) {
    midi_cv::MidiCv::install_channel_count_deriver(map);
    midi_cc::MidiCc::install_channel_count_deriver(map);
    midi_gate::MidiGate::install_channel_count_deriver(map);
}

pub fn schemas() -> Vec<ModuleSchema> {
    vec![
        midi_cv::MidiCv::get_schema(),
        midi_cc::MidiCc::get_schema(),
        midi_gate::MidiGate::get_schema(),
    ]
}
