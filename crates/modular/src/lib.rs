#![deny(clippy::all)]

mod audio;
mod midi;
mod validation;

use chrono::format;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use napi::bindgen_prelude::Float32Array;
use std::{collections::HashMap, sync::Arc};

use modular_core::{Patch, PatchGraph, dsp::schema, types::ScopeItem};
use napi::Result;
use napi_derive::napi;
use parking_lot::Mutex;

use crate::audio::{ApplyPatchError, AudioBudgetSnapshot, AudioDeviceInfo, AudioState, get_host_by_preference};
use crate::audio::{make_stream, make_input_stream, list_output_devices, list_input_devices, SharedInputBuffer, InputRingBuffer};
use crate::midi::{MidiInputManager, MidiPortInfo};

/// Information about a MIDI input port (for N-API)
#[napi(object)]
pub struct MidiInputInfo {
    pub name: String,
    pub index: u32,
}

#[napi(js_name = "Synthesizer")]
pub struct Synthesizer {
  state: Arc<AudioState>,
  _output_stream: cpal::Stream,
  _input_stream: Option<cpal::Stream>,
  input_buffer: Option<SharedInputBuffer>,
  midi_manager: Arc<MidiInputManager>,
  sample_rate: f32,
  channels: u16,
  input_channels: u16,
  is_recording: bool,
  output_device_name: Option<String>,
  input_device_name: Option<String>,
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
    let output_device_name = device.name().ok();
    let config = device.default_output_config().map_err(|err| {
      napi::Error::from_reason(format!("Failed to get default output config: {}", err))
    })?;
    let sample_rate = config.sample_rate() as f32;
    let channels = config.channels();

    let state = Arc::new(AudioState::new(
      Arc::new(Mutex::new(Patch::new())),
      sample_rate,
      channels,
    ));

    println!("Audio output: {} Hz, {} channels", sample_rate, channels);

    let stream = match config.sample_format() {
      cpal::SampleFormat::I8 => make_stream::<i8>(&device, &config.into(), &state.clone(), None),
      cpal::SampleFormat::I16 => make_stream::<i16>(&device, &config.into(), &state.clone(), None),
      cpal::SampleFormat::I32 => make_stream::<i32>(&device, &config.into(), &state.clone(), None),
      cpal::SampleFormat::F32 => make_stream::<f32>(&device, &config.into(), &state.clone(), None),
      _ => Err(napi::Error::from_reason(format!(
        "Unsupported sample format: {:?}",
        config.sample_format()
      )))?,
    }
    .map_err(|e| napi::Error::from_reason(format!("Failed to create audio stream: {}", e)))?;

    stream
      .play()
      .map_err(|e| napi::Error::from_reason(format!("Failed to start audio stream: {}", e)))?;

    println!("Audio output stream started.");

    // Create MIDI manager
    let midi_manager = Arc::new(MidiInputManager::new());

    Ok(Self {
      state,
      _output_stream: stream,
      _input_stream: None,
      input_buffer: None,
      midi_manager,
      sample_rate,
      channels,
      input_channels: 0,
      is_recording: false,
      output_device_name,
      input_device_name: None,
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
  pub fn input_channels(&self) -> u16 {
    self.input_channels
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

  // =========================================================================
  // Audio Device Management
  // =========================================================================

  /// List all available audio output devices
  #[napi]
  pub fn list_audio_output_devices(&self) -> Vec<AudioDeviceInfo> {
    list_output_devices()
  }

  /// List all available audio input devices
  #[napi]
  pub fn list_audio_input_devices(&self) -> Vec<AudioDeviceInfo> {
    list_input_devices()
  }

  /// Get the current output device name
  #[napi]
  pub fn get_output_device_name(&self) -> Option<String> {
    self.output_device_name.clone()
  }

  /// Get the current input device name
  #[napi]
  pub fn get_input_device_name(&self) -> Option<String> {
    self.input_device_name.clone()
  }

  /// Set the audio output device by name
  /// This will stop and recreate the audio stream
  #[napi]
  pub fn set_audio_devices(&mut self, input_device_name: String, output_device_name: String) -> Result<()> {
    Ok(())
  }

  pub fn set_audio_output_device(&mut self, device_name: String, patch: Arc<Mutex<Patch>>) -> Result<()> {
    use crate::audio::find_output_device;
    
    let host = get_host_by_preference();
    
    // Find the device
    let device = if device_name == "default" {
      host.default_output_device()
        .ok_or_else(|| napi::Error::from_reason("No default output device found".to_string()))?
    } else {
      host.output_devices()
        .map_err(|e| napi::Error::from_reason(format!("Failed to enumerate devices: {}", e)))?
        .find(|d| d.name().ok().as_deref() == Some(&device_name))
        .ok_or_else(|| napi::Error::from_reason(format!("Device '{}' not found", device_name)))?
    };

    let config = device.default_output_config()
      .map_err(|e| napi::Error::from_reason(format!("Failed to get device config: {}", e)))?;
    
    let new_sample_rate = config.sample_rate() as f32;
    let new_channels = config.channels();


    // Create new state with updated config
    let new_state = Arc::new(AudioState::new(
      patch,
      new_sample_rate,
      new_channels,
    ));

    // Create new stream
    let stream = match config.sample_format() {
      cpal::SampleFormat::I8 => make_stream::<i8>(&device, &config.into(), &new_state, self.input_buffer.clone()),
      cpal::SampleFormat::I16 => make_stream::<i16>(&device, &config.into(), &new_state, self.input_buffer.clone()),
      cpal::SampleFormat::I32 => make_stream::<i32>(&device, &config.into(), &new_state, self.input_buffer.clone()),
      cpal::SampleFormat::F32 => make_stream::<f32>(&device, &config.into(), &new_state, self.input_buffer.clone()),
      _ => Err(napi::Error::from_reason(format!(
        "Unsupported sample format: {:?}",
        config.sample_format()
      )))?,
    }?;

    stream.play()
      .map_err(|e| napi::Error::from_reason(format!("Failed to start stream: {}", e)))?;

    // Update self
    self._output_stream = stream;
    self.state = new_state;
    self.sample_rate = new_sample_rate;
    self.channels = new_channels;
    self.output_device_name = device.name().ok();

    println!("Audio output device changed to: {} ({} Hz, {} channels)",
      self.output_device_name.as_deref().unwrap_or("unknown"),
      self.sample_rate,
      self.channels
    );

    Ok(())
  }

  /// Set the audio input device by name
  /// This will stop and recreate the input stream
  #[napi]
  pub fn set_audio_input_device(&mut self, device_name: Option<String>) -> Result<()> {
    // If None, disable input
    if device_name.is_none() {
      self._input_stream = None;
      self.input_buffer = None;
      self.input_device_name = None;
      self.input_channels = 0;
      println!("Audio input disabled");
      return Ok(());
    }

    let device_name = device_name.unwrap();
    let host = get_host_by_preference();

    // Find the device
    let device = if device_name == "default" {
      host.default_input_device()
        .ok_or_else(|| napi::Error::from_reason("No default input device found".to_string()))?
    } else {
      host.input_devices()
        .map_err(|e| napi::Error::from_reason(format!("Failed to enumerate devices: {}", e)))?
        .find(|d| d.name().ok().as_deref() == Some(&device_name))
        .ok_or_else(|| napi::Error::from_reason(format!("Device '{}' not found", device_name)))?
    };

    let config = device.default_input_config()
      .map_err(|e| napi::Error::from_reason(format!("Failed to get device config: {}", e)))?;

    let input_channels = config.channels() as usize;
    let input_buffer = Arc::new(InputRingBuffer::new());

    // Create input stream
    let stream = match config.sample_format() {
      cpal::SampleFormat::I8 => make_input_stream::<i8>(&device, &config.into(), input_buffer.clone(), input_channels),
      cpal::SampleFormat::I16 => make_input_stream::<i16>(&device, &config.into(), input_buffer.clone(), input_channels),
      cpal::SampleFormat::I32 => make_input_stream::<i32>(&device, &config.into(), input_buffer.clone(), input_channels),
      cpal::SampleFormat::F32 => make_input_stream::<f32>(&device, &config.into(), input_buffer.clone(), input_channels),
      _ => Err(napi::Error::from_reason(format!(
        "Unsupported sample format: {:?}",
        config.sample_format()
      )))?,
    }?;

    stream.play()
      .map_err(|e| napi::Error::from_reason(format!("Failed to start input stream: {}", e)))?;

    self._input_stream = Some(stream);
    self.input_buffer = Some(input_buffer);
    self.input_device_name = device.name().ok();
    self.input_channels = input_channels as u16;

    println!("Audio input device set to: {} ({} channels)",
      self.input_device_name.as_deref().unwrap_or("unknown"),
      self.input_channels
    );

    Ok(())
  }

  // =========================================================================
  // MIDI Device Management
  // =========================================================================

  /// List all available MIDI input ports
  #[napi]
  pub fn list_midi_inputs(&self) -> Vec<MidiInputInfo> {
    MidiInputManager::list_ports()
      .into_iter()
      .map(|p| MidiInputInfo {
        name: p.name,
        index: p.index as u32,
      })
      .collect()
  }

  /// Get the currently connected MIDI input port name
  #[napi]
  pub fn get_midi_input_name(&self) -> Option<String> {
    self.midi_manager.connected_port()
  }

  /// Connect to a MIDI input port by name
  #[napi]
  pub fn set_midi_input(&self, port_name: Option<String>) -> Result<()> {
    match port_name {
      None => {
        self.midi_manager.disconnect();
        println!("MIDI input disconnected");
        Ok(())
      }
      Some(name) => {
        self.midi_manager.connect(&name)
          .map_err(|e| napi::Error::from_reason(e))?;
        println!("MIDI input connected to: {}", name);
        Ok(())
      }
    }
  }

  /// Poll MIDI input and dispatch messages to the audio thread.
  /// Call this periodically (e.g., on each animation frame or timer tick).
  #[napi]
  pub fn poll_midi(&self) {
    let messages = self.midi_manager.take_messages();
    if !messages.is_empty() {
      self.state.queue_midi_messages(messages);
    }
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
