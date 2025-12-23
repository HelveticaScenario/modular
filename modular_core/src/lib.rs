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
extern crate mi_plaits_dsp;

pub mod dsp;
pub mod patch;
pub mod pattern;
pub mod types;

// Re-export commonly used items
pub use patch::Patch;
pub use pattern::{Condition, MiniError, Pattern, PatternExpr, PatternState, PatternTransform, PatternValue, Span, TickResult, ValueOp, parse_mini};
pub use types::{
	DataParamSchema, DataParamType, DataParamValue, InternalDataParam, InternalParam, InternalTrack,
	Keyframe, Module, ModuleSchema, ModuleState, Param, Params, PatchGraph, Sampleable,
	SignalParamSchema,
	SampleableConstructor, SampleableMap, Track, TrackMap, ROOT_ID, ROOT_OUTPUT_PORT,
};
