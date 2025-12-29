#![deny(clippy::all)]

mod audio;
mod validation;

use cpal::FromSample;
use cpal::SizedSample;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use modular_core::dsp::schema;
use std::{collections::HashMap, sync::Arc};

use modular_core::{
  ModuleState, Patch, PatchGraph,
  types::{ScopeItem, TrackProxy},
};
use napi::{
  Env, Result,
  bindgen_prelude::{Object, ObjectRef},
};
use napi_derive::napi;
use parking_lot::Mutex;
use serde::Deserialize;

use crate::audio::AudioState;
use crate::audio::make_stream;

#[derive(Deserialize)]
struct Foo {
  pub v1: String,
  pub v2: u32,
  pub v3: serde_json::Value,
}

#[napi(js_name = "Synthesizer")]
pub struct Synthesizer {
  state: Arc<AudioState>,
  stream: Option<cpal::Stream>,
}

#[napi]
impl Synthesizer {
  #[napi(constructor)]
  pub fn new() -> Self {
    // value
    Self {
      state: Arc::new(AudioState::new(
        Arc::new(Mutex::new(Patch::new(HashMap::new(), HashMap::new()))),
        44100.0,
      )),
      stream: None,
    }
  }

  #[napi]
  pub fn stop(&mut self) {
    // Take ownership of the stream and drop it
    self.stream.take();
  }

  /// Run the audio thread with cpal
  #[napi]
  pub fn start(&mut self) -> Result<()> {
    let host = cpal::default_host();
    let device = host
      .default_output_device()
      .ok_or_else(|| napi::Error::from_reason(format!("No audio output device found")))?;
    let config = device.default_output_config().map_err(|err| {
      napi::Error::from_reason(format!("Failed to get default output config: {}", err))
    })?;
    let sample_rate = config.sample_rate() as f32;
    let channels = config.channels() as usize;

    tracing::info!("Audio: {} Hz, {} channels", sample_rate, channels);

    let stream = match config.sample_format() {
      cpal::SampleFormat::I8 => make_stream::<i8>(&device, &config.into(), &self.state.clone()),
      cpal::SampleFormat::I16 => make_stream::<i16>(&device, &config.into(), &self.state.clone()),
      cpal::SampleFormat::I32 => make_stream::<i32>(&device, &config.into(), &self.state.clone()),
      cpal::SampleFormat::F32 => make_stream::<f32>(&device, &config.into(), &self.state.clone()),
      _ => Err(napi::Error::from_reason(format!(
        "Unsupported sample format: {:?}",
        config.sample_format()
      )))?,
    }
    .map_err(|e| napi::Error::from_reason(format!("Failed to create audio stream: {}", e)))?;

    stream
      .play()
      .map_err(|e| napi::Error::from_reason(format!("Failed to start audio stream: {}", e)))?;

    self.stream = Some(stream);
    println!("Audio stream started.");
    Ok(())
  }
}

#[napi]
pub fn get_schemas() -> Result<Vec<modular_core::types::ModuleSchema>> {
  Ok(schema())
}
