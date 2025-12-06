//! Modular synthesizer core library
//! 
//! This crate provides the core DSP functionality for a modular synthesizer.
//! It is a pure library with no I/O, protocol handling, or serialization concerns.
//! Those responsibilities belong in the server layer.

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate modular_derive;

extern crate anyhow;
extern crate parking_lot;
extern crate serde;
extern crate serde_json;

pub mod dsp;
pub mod patch;
pub mod types;

// Re-export commonly used items
pub use patch::Patch;
pub use types::{
    InternalParam, InternalTrack, Keyframe, Module, ModuleSchema, ModuleState, Param, Params,
    PatchGraph, Playmode, ParamSchema, Sampleable, SampleableConstructor, SampleableMap, Track,
    TrackMap, TrackUpdate, ROOT_ID, ROOT_OUTPUT_PORT,
};
