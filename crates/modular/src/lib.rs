#![deny(clippy::all)]

mod audio;
mod validation;

use chrono::format;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use napi::bindgen_prelude::Float32Array;
use std::{collections::HashMap, sync::Arc};

use modular_core::{Patch, PatchGraph, dsp::schema, types::ScopeItem};
use napi::Result;
use napi_derive::napi;
use parking_lot::Mutex;

use crate::audio::{ApplyPatchError, AudioBudgetSnapshot, AudioState, get_host_by_preference};
use crate::audio::make_stream;

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
    let host = get_host_by_preference();

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

    println!("Audio: {} Hz, {} channels", sample_rate, channels);

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
  pub fn get_health(&self) -> AudioBudgetSnapshot {
    self.state.take_audio_thread_budget_snapshot_and_reset()
  }

  #[napi]
  pub fn get_module_states(&self) -> HashMap<String, serde_json::Value> {
    self.state.get_module_states()
  }
}

#[napi]
pub fn get_schemas() -> Result<Vec<modular_core::types::ModuleSchema>> {
  Ok(schema())
}

/// Parse a mini notation pattern and return all leaf spans.
/// 
/// This is used by the Monaco editor to create tracked decorations
/// that move with text edits.
#[napi]
pub fn get_mini_leaf_spans(source: String) -> Result<Vec<Vec<u32>>> {
  use modular_core::pattern_system::mini::{parse_ast, collect_leaf_spans};
  
  let ast = parse_ast(&source)
    .map_err(|e| napi::Error::from_reason(e.to_string()))?;
  
  let spans = collect_leaf_spans(&ast);
  
  // Convert to Vec<Vec<u32>> for N-API (since tuples aren't directly supported)
  Ok(spans.into_iter().map(|(start, end)| vec![start as u32, end as u32]).collect())
}

/// Analyze a mini notation pattern and return the maximum polyphony needed.
/// 
/// Queries 300 cycles (10 min at 120 BPM) and counts the maximum number of simultaneous haps,
/// capping at 16 (the poly voice limit). Logs timing for profiling.
#[napi]
pub fn get_pattern_polyphony(source: String) -> Result<u32> {
  use modular_core::pattern_system::{Fraction, mini::parse};
  use modular_core::dsp::seq::SeqValue;
  use std::time::Instant;
  use std::cmp::Ordering;
  
  let start = Instant::now();
  
  // Parse using SeqValue - handles notes, numbers, module references, etc.
  let pattern: modular_core::pattern_system::Pattern<SeqValue> = parse(&source)
    .map_err(|e| napi::Error::from_reason(e.to_string()))?;
  
  let parse_time = start.elapsed();
  let query_start = Instant::now();
  
  const NUM_CYCLES: i64 = 300;
  const MAX_POLYPHONY: u32 = 16;
  
  // Query all cycles at once
  let haps = pattern.query_arc(
    Fraction::from_integer(0),
    Fraction::from_integer(NUM_CYCLES),
  );
  
  // Sweep line algorithm: create +1 events at start, -1 events at end
  // Event: (time, delta) where delta is +1 for start, -1 for end
  let mut events: Vec<(Fraction, i32)> = Vec::with_capacity(haps.len() * 2);
  
  for hap in &haps {
    if hap.value.is_rest() {
      continue;
    }
    events.push((hap.part.begin.clone(), 1));  // +1 at start
    events.push((hap.part.end.clone(), -1));   // -1 at end
  }
  
  // Sort by time, with ends (-1) before starts (+1) at same time
  // This ensures a note ending exactly when another starts doesn't count as overlap
  events.sort_by(|a, b| {
    match a.0.cmp(&b.0) {
      Ordering::Equal => a.1.cmp(&b.1), // -1 comes before +1
      other => other,
    }
  });
  
  // Sweep through events tracking current and max polyphony
  let mut current: u32 = 0;
  let mut max_simultaneous: u32 = 0;
  
  for (_time, delta) in events {
    if delta > 0 {
      current += 1;
      max_simultaneous = max_simultaneous.max(current);
      // Early exit if we hit the cap
      if max_simultaneous >= MAX_POLYPHONY {
        let query_time = query_start.elapsed();
        println!(
          "Pattern polyphony analysis: parse={:?}, query={:?}, max_poly={} (capped)",
          parse_time, query_time, MAX_POLYPHONY
        );
        return Ok(MAX_POLYPHONY);
      }
    } else {
      current = current.saturating_sub(1);
    }
  }
  
  let query_time = query_start.elapsed();
  println!(
    "Pattern polyphony analysis: parse={:?}, query={:?}, haps={}, max_poly={}",
    parse_time, query_time, haps.len(), max_simultaneous
  );
  
  Ok(max_simultaneous.max(1)) // At least 1 channel
}
