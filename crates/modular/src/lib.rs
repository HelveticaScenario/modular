#![deny(clippy::all)]

mod audio;
mod validation;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use napi::bindgen_prelude::Float32Array;
use std::{collections::HashMap, sync::Arc};

use modular_core::{Patch, PatchGraph, dsp::schema, types::ScopeItem};
use napi::Result;
use napi_derive::napi;
use parking_lot::Mutex;

use crate::audio::{ApplyPatchError, AudioState};
use crate::audio::{AudioThreadHealthSnapshot, make_stream};

#[napi(js_name = "Synthesizer")]
pub struct Synthesizer {
  state: Arc<AudioState>,
  _stream: cpal::Stream,
  sample_rate: f32,
  channels: u16,
  is_recording: bool,
}

#[napi]
impl Synthesizer {
  /// Run the audio thread with cpal
  #[napi(constructor)]
  pub fn new() -> Result<Self> {
    let host = cpal::default_host();

    let device = host
      .default_output_device()
      .ok_or_else(|| napi::Error::from_reason(format!("No audio output device found")))?;
    let config = device.default_output_config().map_err(|err| {
      napi::Error::from_reason(format!("Failed to get default output config: {}", err))
    })?;
    let sample_rate = config.sample_rate() as f32;
    let channels = config.channels();

    let state = Arc::new(AudioState::new(
      Arc::new(Mutex::new(Patch::new(HashMap::new()))),
      sample_rate,
      channels,
    ));

    tracing::info!("Audio: {} Hz, {} channels", sample_rate, channels);

    let stream = match config.sample_format() {
      cpal::SampleFormat::I8 => make_stream::<i8>(&device, &config.into(), &state.clone()),
      cpal::SampleFormat::I16 => make_stream::<i16>(&device, &config.into(), &state.clone()),
      cpal::SampleFormat::I32 => make_stream::<i32>(&device, &config.into(), &state.clone()),
      cpal::SampleFormat::F32 => make_stream::<f32>(&device, &config.into(), &state.clone()),
      _ => Err(napi::Error::from_reason(format!(
        "Unsupported sample format: {:?}",
        config.sample_format()
      )))?,
    }
    .map_err(|e| napi::Error::from_reason(format!("Failed to create audio stream: {}", e)))?;

    stream
      .play()
      .map_err(|e| napi::Error::from_reason(format!("Failed to start audio stream: {}", e)))?;

    println!("Audio stream started.");
    Ok(Self {
      state,
      _stream: stream,
      sample_rate,
      channels,
      is_recording: false,
    })
  }

  #[napi]
  pub fn stop(&mut self) {
    self.state.set_stopped(true);
  }

  #[napi]
  pub fn is_stopped(&self) -> bool {
    self.state.is_stopped()
  }

  #[napi]
  pub fn sample_rate(&self) -> f32 {
    self.sample_rate
  }

  #[napi]
  pub fn channels(&self) -> u16 {
    self.channels
  }

  #[napi]
  pub fn get_scopes(&self) -> Vec<(ScopeItem, Float32Array)> {
    self.state.get_audio_buffers()
  }

  #[napi]
  pub fn update_patch(&self, patch: PatchGraph) -> Vec<ApplyPatchError> {
    self.state.handle_set_patch(patch, self.sample_rate)
  }

  #[napi]
  pub fn start_recording(&mut self, path: Option<String>) -> Result<String> {
    if self.is_recording {
      return Err(napi::Error::from_reason(
        "Recording is already in progress".to_string(),
      ));
    }
    match self.state.start_recording(path) {
      Ok(p) => {
        self.is_recording = true;
        Ok(p)
      }
      Err(e) => Err(e),
    }
  }

  #[napi]
  pub fn stop_recording(&mut self) -> Result<Option<String>> {
    if !self.is_recording {
      return Err(napi::Error::from_reason(
        "No recording is in progress".to_string(),
      ));
    }
    match self.state.stop_recording() {
      Ok(p) => {
        self.is_recording = false;
        Ok(p)
      }
      Err(e) => Err(e),
    }
  }

  #[napi]
  pub fn is_recording(&self) -> bool {
    self.is_recording
  }

  #[napi]
  pub fn get_health(&self) -> AudioThreadHealthSnapshot {
    self.state.take_audio_thread_health_snapshot_and_reset()
  }
}

#[napi]
pub fn get_schemas() -> Result<Vec<modular_core::types::ModuleSchema>> {
  Ok(schema())
}
