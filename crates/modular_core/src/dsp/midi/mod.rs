//! MIDI DSP modules for converting MIDI messages to control voltages.
//!
//! This module provides modules that read from the shared MIDI state
//! and output appropriate control voltages for use in the patch graph.

use std::collections::HashMap;

use crate::params::ParamsDeserializer;
use crate::types::{Module, ModuleSchema, SampleableConstructor};

pub mod midi_cc;
pub mod midi_cv;

pub fn install_constructors(map: &mut HashMap<String, SampleableConstructor>) {
    midi_cv::MidiCv::install_constructor(map);
    midi_cc::MidiCc::install_constructor(map);
}

pub fn install_params_deserializers(map: &mut HashMap<String, ParamsDeserializer>) {
    midi_cv::MidiCv::install_params_deserializer(map);
    midi_cc::MidiCc::install_params_deserializer(map);
}

pub fn schemas() -> Vec<ModuleSchema> {
    vec![midi_cv::MidiCv::get_schema(), midi_cc::MidiCc::get_schema()]
}
