use cpal::FromSample;
use cpal::Host;
use cpal::HostId;
use cpal::Sample;
use cpal::SizedSample;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use core::num;
use hound::{WavSpec, WavWriter};
use modular_core::PORT_MAX_CHANNELS;
use modular_core::PatchGraph;
use modular_core::dsp::get_constructors;
use modular_core::dsp::schema;
use modular_core::dsp::utils::SchmittTrigger;
use modular_core::types::ClockMessages;
use modular_core::types::Message;
use modular_core::types::Scope;
use modular_core::types::WellKnownModule;
use napi::Result;
use napi::bindgen_prelude::Float32Array;
use napi_derive::napi;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;
use std::sync::OnceLock;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::{AtomicBool, Ordering};

use modular_core::patch::Patch;
use modular_core::types::{ROOT_OUTPUT_PORT, ScopeItem};
use std::time::Instant;

// ============================================================================
// Audio Device Information
// ============================================================================

/// Information about an audio device
#[derive(Debug, Clone)]
#[napi(object)]
pub struct AudioDeviceInfo {
  /// Stable Device ID
  pub id: String,
  /// Device name
  pub name: String,
  /// Number of input channels (0 if output-only)
  pub input_channels: u16,
  /// Number of output channels (0 if input-only)
  pub output_channels: u16,
  /// Whether this is the default device
  pub is_default: bool,
}

/// List all available audio output devices
pub fn list_output_devices() -> Vec<AudioDeviceInfo> {
  let host = get_host_by_preference();
  let default_device_id = host.default_output_device().and_then(|d| d.id().ok());

  host
    .devices()
    .map(|devices| {
      devices
        .filter_map(|device| {
          let id = device.id().ok()?;
          let config = device.default_output_config().ok()?;
          Some(AudioDeviceInfo {
            is_default: default_device_id.as_ref() == Some(&id),

            id: id.to_string(),
            name: device.description().ok()?.name().to_owned(),
            input_channels: 0,
            output_channels: config.channels(),
          })
        })
        .collect()
    })
    .unwrap_or_default()
}

/// List all available audio input devices
pub fn list_input_devices() -> Vec<AudioDeviceInfo> {
  let host = get_host_by_preference();
  let default_device_id = host.default_input_device().and_then(|d| d.id().ok());

  host
    .input_devices()
    .map(|devices| {
      devices
        .filter_map(|device| {
          let id = device.id().ok()?;
          let config = device.default_input_config().ok()?;
          Some(AudioDeviceInfo {
            is_default: default_device_id.as_ref() == Some(&id),
            id: id.to_string(),
            name: device.description().ok()?.name().to_owned(),
            input_channels: config.channels(),
            output_channels: 0,
          })
        })
        .collect()
    })
    .unwrap_or_default()
}

/// Find an output device by id
pub fn find_output_device(id: &str) -> Option<cpal::Device> {
  let host = get_host_by_preference();
  host
    .output_devices()
    .ok()?
    .find(|d| d.id().ok() == cpal::DeviceId::from_str(id).ok())
}

/// Find an input device by id
pub fn find_input_device(id: &str) -> Option<cpal::Device> {
  let host = get_host_by_preference();
  host
    .input_devices()
    .ok()?
    .find(|d| d.id().ok() == cpal::DeviceId::from_str(id).ok())
}

// ============================================================================
// Audio Input Ring Buffer
// ============================================================================

/// Ring buffer size for audio input (in samples per channel)
const INPUT_RING_BUFFER_SIZE: usize = 4096;

/// Total size of the flat audio input buffer
const INPUT_BUFFER_TOTAL_SIZE: usize = INPUT_RING_BUFFER_SIZE * PORT_MAX_CHANNELS;

/// Thread-safe ring buffer for audio input
/// Buffer layout: flat array [sample_0_ch_0, sample_0_ch_1, ..., sample_1_ch_0, ...]
pub struct InputRingBuffer {
  /// Flat buffer data: sample_idx * PORT_MAX_CHANNELS + channel
  buffer: Mutex<[f32; INPUT_BUFFER_TOTAL_SIZE]>,
  /// Write position in frames
  write_pos: Mutex<usize>,
}

impl InputRingBuffer {
  pub fn new() -> Self {
    Self {
      buffer: Mutex::new([0.0; INPUT_BUFFER_TOTAL_SIZE]),
      write_pos: Mutex::new(0),
    }
  }

  /// Write samples from input callback (interleaved)
  /// `channels` specifies how many channels are in the interleaved data
  pub fn write_interleaved(&mut self, data: &[f32], channels: usize) {
    if channels == 0 {
      return;
    }

    let frames = data.len() / channels;
    let mut buffer = self.buffer.lock();
    let mut write_pos = self.write_pos.lock();

    for frame_idx in 0..frames {
      let buf_idx = *write_pos % INPUT_RING_BUFFER_SIZE;
      let base_offset = buf_idx * PORT_MAX_CHANNELS;
      let channels_to_write = channels.min(PORT_MAX_CHANNELS);

      buffer[base_offset..][..channels_to_write]
        .copy_from_slice(&data[frame_idx * channels..][..channels_to_write]);
      buffer[base_offset + channels_to_write..][..PORT_MAX_CHANNELS - channels_to_write].fill(0.0);

      *write_pos += 1;
    }
  }

  /// Read the most recent sample frame (all channels)
  pub fn read_latest(&self) -> [f32; PORT_MAX_CHANNELS] {
    let buffer = self.buffer.lock();
    let write_pos = self.write_pos.lock();
    let pos = *write_pos;
    let idx = if pos == 0 { 0 } else { (pos - 1) % INPUT_RING_BUFFER_SIZE };
    let base_offset = idx * PORT_MAX_CHANNELS;

    let mut result = [0.0; PORT_MAX_CHANNELS];
    result.copy_from_slice(&buffer[base_offset..][..PORT_MAX_CHANNELS]);
    result
  }
}

/// Shared input buffer type
pub type SharedInputBuffer = Arc<InputRingBuffer>;


// ============================================================================
// Multi-Channel Output Buffer
// ============================================================================

/// Output buffer for multi-channel audio
/// Each DSP module can write to specific channels
pub struct OutputBuffer {
  /// Sample values per channel for current frame
  samples: [f32; PORT_MAX_CHANNELS],
  /// Number of active channels
  channels: u16,
}

impl OutputBuffer {
  pub fn new(channels: u16) -> Self {
    Self {
      samples: [0.0; PORT_MAX_CHANNELS],
      channels,
    }
  }

  /// Clear all samples to zero
  pub fn clear(&mut self) {
    for s in &mut self.samples[..self.channels as usize] {
      *s = 0.0;
    }
  }

  /// Add a sample to a specific channel (mixing)
  pub fn add(&mut self, channel: usize, value: f32) {
    if channel < self.channels as usize {
      self.samples[channel] += value;
    }
  }

  /// Set a sample for a specific channel (replacing)
  pub fn set(&mut self, channel: usize, value: f32) {
    if channel < self.channels as usize {
      self.samples[channel] = value;
    }
  }

  /// Get sample for a channel
  pub fn get(&self, channel: usize) -> f32 {
    if channel < self.channels as usize {
      self.samples[channel]
    } else {
      0.0
    }
  }

  pub fn channels(&self) -> u16 {
    self.channels
  }
}

fn apply_patch_debug_enabled() -> bool {
  static ENABLED: OnceLock<bool> = OnceLock::new();
  *ENABLED.get_or_init(|| match std::env::var("MODULAR_DEBUG_LOG") {
    Ok(v) => {
      let v = v.trim().to_ascii_lowercase();
      v == "1" || v == "true" || v == "yes" || v == "on"
    }
    Err(_) => false,
  })
}

fn format_id_set_sample(set: &HashSet<String>, max: usize) -> String {
  if set.is_empty() {
    return "(empty)".to_string();
  }

  let mut ids: Vec<&String> = set.iter().collect();
  ids.sort();

  let shown: Vec<&str> = ids.iter().take(max).map(|s| s.as_str()).collect();

  if set.len() <= max {
    format!("{}", shown.join(", "))
  } else {
    format!("{} …(+{})", shown.join(", "), set.len().saturating_sub(max))
  }
}

macro_rules! patch_dbg {
  ($($arg:tt)*) => {
    if apply_patch_debug_enabled() {
      eprintln!($($arg)*);
    }
  };
}

#[napi(object)]
pub struct ApplyPatchError {
  pub message: String,
  pub errors: Option<Vec<ValidationError>>,
}

use crate::validation::ValidationError;
use crate::validation::validate_patch;

/// Attenuation factor applied to audio output to prevent clipping.
/// DSP modules output signals in the range [-5, 5] volts (modular synth convention).
/// This factor brings the output into a reasonable range for audio output.
const AUDIO_OUTPUT_ATTENUATION: f32 = 0.2;

const SCOPE_CAPACITY: u32 = 1024;

// Adapted from https://github.com/VCVRack/Fundamental/blob/e819498fd388755efcb876b37d1e33fddf4a29ac/src/Scope.cpp
pub struct ScopeBuffer {
  sample_counter: u32,
  skip_rate: u32,
  trigger_threshold: Option<f32>,
  trigger: SchmittTrigger,
  buffer: [f32; SCOPE_CAPACITY as usize],
  buffer_idx: usize,
}

fn ms_to_samples(ms: u32, sample_rate: f32) -> u32 {
  ((ms as f32 / 1000.0) * sample_rate) as u32
}

// A function that calculates the skip rate needed to capture target samples over total samples
fn calculate_skip_rate(total_samples: u32) -> u32 {
  total_samples / SCOPE_CAPACITY
}

impl ScopeBuffer {
  pub fn new(scope: &Scope, sample_rate: f32) -> Self {
    let mut sb = Self {
      buffer: [0.0; SCOPE_CAPACITY as usize],
      sample_counter: 0,
      skip_rate: 0,
      trigger_threshold: None,
      trigger: SchmittTrigger::new(0.0, 0.0),
      buffer_idx: 0,
    };

    sb.update(scope, sample_rate);
    sb.trigger = SchmittTrigger::new(
      sb.trigger_threshold.unwrap_or(0.0),
      sb.trigger_threshold.unwrap_or(0.0) + 0.001,
    );

    sb
  }

  fn update_trigger_threshold(&mut self, threshold: Option<i32>) {
    let threshold = threshold.map(|t| (t as f32) / 1000.0);
    self.trigger_threshold = threshold;
    if let Some(thresh) = threshold {
      self.trigger.set_thresholds(thresh, thresh + 0.001);
      self.trigger.reset();
    }
  }

  fn update_skip_rate(&mut self, ms_per_frame: u32, sample_rate: f32) {
    self.skip_rate = calculate_skip_rate(ms_to_samples(ms_per_frame, sample_rate));
  }

  pub fn push(&mut self, value: f32) {
    if self.buffer_idx >= SCOPE_CAPACITY as usize {
      let mut triggered = false;

      if self.trigger_threshold.is_none() {
        triggered = true;
      } else {
        if self.trigger.process(value) {
          triggered = true;
        }
      }

      if triggered {
        self.trigger.reset();
        self.buffer_idx = 0;
      }
    }

    if self.buffer_idx < SCOPE_CAPACITY as usize {
      if self.sample_counter == 0 {
        self.buffer[self.buffer_idx] = value;
        if (self.buffer_idx + 1) <= SCOPE_CAPACITY as usize {
          self.buffer_idx += 1;
        }
      }
      self.sample_counter += 1;
      if self.sample_counter > self.skip_rate {
        self.sample_counter = 0;
      }
    }
  }

  pub fn update(&mut self, scope: &Scope, sample_rate: f32) {
    self.update_trigger_threshold(scope.trigger_threshold);
    self.update_skip_rate(scope.ms_per_frame, sample_rate);
  }
}

impl From<&ScopeBuffer> for Float32Array {
  fn from(scope_buffer: &ScopeBuffer) -> Self {
    Float32Array::new(scope_buffer.buffer.to_vec())
  }
}

/// Shared audio state between audio thread and server
pub struct AudioState {
  patch: Arc<Mutex<Patch>>,
  stopped: Arc<AtomicBool>,
  scope_collection: Arc<Mutex<HashMap<ScopeItem, ScopeBuffer>>>,
  recording_writer: Arc<Mutex<Option<WavWriter<BufWriter<File>>>>>,
  recording_path: Arc<Mutex<Option<PathBuf>>>,
  /// Pending MIDI messages to dispatch
  midi_messages: Arc<Mutex<Vec<Message>>>,
  sample_rate: f32,
  channels: u16,
  audio_budget_meter: AudioBudgetMeter,
}

#[derive(Default)]
struct AudioThreadHealth {
  estimated_frame_budget_usage_max: AtomicU64,
}

#[derive(Debug, Clone, Copy)]
#[napi(object)]
pub struct AudioThreadHealthSnapshot {
  pub estimated_frame_budget_usage_max: f64,
}

impl AudioState {
  pub fn new(patch: Arc<Mutex<Patch>>, sample_rate: f32, channels: u16) -> Self {
    Self {
      patch,
      stopped: Arc::new(AtomicBool::new(true)),
      scope_collection: Arc::new(Mutex::new(HashMap::new())),
      recording_writer: Arc::new(Mutex::new(None)),
      recording_path: Arc::new(Mutex::new(None)),
      midi_messages: Arc::new(Mutex::new(Vec::with_capacity(256))),
      sample_rate,
      channels,
      audio_budget_meter: AudioBudgetMeter::default(),
    }
  }

  /// Queue MIDI messages to be dispatched on the audio thread
  pub fn queue_midi_messages(&self, messages: Vec<Message>) {
    if messages.is_empty() {
      return;
    }
    let mut midi = self.midi_messages.lock();
    midi.extend(messages);
  }

  /// Dispatch pending MIDI messages to the patch (called from audio thread)
  fn dispatch_midi_messages(&self) {
    // Take messages with minimal lock time
    let messages: Vec<Message> = {
      let mut midi = self.midi_messages.lock();
      std::mem::take(&mut *midi)
    };

    if messages.is_empty() {
      return;
    }

    // Dispatch to patch
    let mut patch = self.patch.lock();
    for msg in messages {
      let _ = patch.dispatch_message(&msg);
    }
  }

  pub fn take_audio_thread_budget_snapshot_and_reset(&self) -> AudioBudgetSnapshot {
    self
      .audio_budget_meter
      .take_snapshot(self.sample_rate as f64, self.channels as f64)
  }

  pub fn set_stopped(&self, stopped: bool) {
    self.stopped.store(stopped, Ordering::SeqCst);
  }

  pub fn is_stopped(&self) -> bool {
    self.stopped.load(Ordering::SeqCst)
  }

  pub fn start_recording(&self, filename: Option<String>) -> Result<String> {
    let filename =
      filename.unwrap_or_else(|| format!("recording_{}.wav", chrono_simple_timestamp()));
    let path = PathBuf::from(&filename);

    let spec = WavSpec {
      channels: 1,
      sample_rate: self.sample_rate as u32,
      bits_per_sample: 32,
      sample_format: hound::SampleFormat::Float,
    };

    let writer = WavWriter::create(&path, spec)
      .map_err(|e| napi::Error::from_reason(format!("Failed to start file write: {}", e)))?;
    *self.recording_writer.lock() = Some(writer);
    *self.recording_path.lock() = Some(path);

    Ok(filename)
  }

  pub fn stop_recording(&self) -> Result<Option<String>> {
    let writer = self.recording_writer.lock().take();
    let path = self.recording_path.lock().take();

    if let Some(w) = writer {
      w.finalize()
        .map_err(|e| napi::Error::from_reason(format!("Failed to finalize file writer: {}", e)))?;
    }

    Ok(path.map(|p| p.to_string_lossy().to_string()))
  }
  pub fn get_audio_buffers(&self) -> Vec<(ScopeItem, Float32Array)> {
    // Skip emitting audio buffers entirely when stopped
    if self.is_stopped() {
      return Vec::new();
    }

    let scope_collection = match self.scope_collection.try_lock() {
      Some(subscription_collection) => subscription_collection,
      None => return Vec::new(), // Skip if locked
    };
    scope_collection
      .iter()
      .map(|(scope_item, scope_buffer)| (scope_item.clone(), Float32Array::from(scope_buffer)))
      .collect()
  }

  pub fn get_module_states(&self) -> HashMap<String, serde_json::Value> {
    let patch = self.patch.lock();
    let mut states = HashMap::new();
    for (id, module) in patch.sampleables.iter() {
      if let Some(state) = module.get_state() {
        states.insert(id.clone(), state);
      }
    }
    states
  }

  pub fn apply_patch(&self, desired_graph: PatchGraph, sample_rate: f32) -> Result<()> {
    let PatchGraph {
      modules,
      module_id_remaps,
      scopes,
      ..
    } = desired_graph;

    let mut patch_lock = self.patch.lock();

    // If the JS main process provided remap hints, rename existing module ids
    // (current id `from` -> desired id `to`) before computing the patch diff.
    // This preserves module instances while keeping the patch state aligned
    // with the desired patch's ids.
    let module_id_remaps = module_id_remaps.unwrap_or_default();
    if !module_id_remaps.is_empty() {
      patch_dbg!(
        "[apply_patch] module_id_remaps count={} (current->desired)",
        module_id_remaps.len()
      );

      // Two-phase rename to support chained remaps in one update.
      // Example: sine-1 -> sine-2 and sine-2 -> sine-3.
      // If we rename sequentially with a naive "collision" check, the first rename would see
      // sine-2 already exists and incorrectly skip.
      let mut moved: HashMap<String, Arc<Box<dyn modular_core::types::Sampleable>>> =
        HashMap::new();

      let mut remap_skipped_reserved = 0usize;
      let mut remap_skipped_identity = 0usize;
      let mut remap_missing_source = 0usize;
      let mut remap_overwrites = 0usize;

      // Phase 1: remove all sources (`from`) we plan to rename.
      for remap in &module_id_remaps {
        // Never touch reserved ids.
        if remap.from == WellKnownModule::RootOutput.id()
          || remap.from == WellKnownModule::RootClock.id()
          || remap.from == WellKnownModule::RootInput.id()
          || remap.from == WellKnownModule::HiddenAudioIn.id()
        {
          remap_skipped_reserved += 1;
          continue;
        }
        if remap.to == WellKnownModule::RootOutput.id()
          || remap.to == WellKnownModule::RootClock.id()
          || remap.to == WellKnownModule::RootInput.id()
          || remap.to == WellKnownModule::HiddenAudioIn.id()
        {
          remap_skipped_reserved += 1;
          continue;
        }
        if remap.from == remap.to {
          remap_skipped_identity += 1;
          continue;
        }

        if let Some(existing) = patch_lock.sampleables.remove(&remap.from) {
          patch_dbg!("[apply_patch] remap move {} -> {}", remap.from, remap.to);
          moved.insert(remap.to.clone(), existing);
        } else {
          remap_missing_source += 1;
          patch_dbg!(
            "[apply_patch] remap source missing (no-op) {} -> {}",
            remap.from,
            remap.to
          );
        }
      }

      // Phase 2: insert under destination (`to`) ids.
      // IMPORTANT: a remap is authoritative about which instance should live at `to`.
      // This must support "shift down" cases like:
      //   sine-2 -> sine-1, sine-3 -> sine-2 (old sine-1 is intentionally dropped)
      // So we *remove any existing `to`* first (unless reserved), then insert.
      if moved.len() != 0 {
        for to_id in moved.keys() {
          if to_id == WellKnownModule::RootOutput.id()
            || to_id == WellKnownModule::RootClock.id()
            || to_id == WellKnownModule::RootInput.id()
            || to_id == WellKnownModule::HiddenAudioIn.id()
          {
            continue;
          }
          if patch_lock.sampleables.remove(to_id).is_some() {
            remap_overwrites += 1;
            patch_dbg!(
              "[apply_patch] remap overwrote existing destination id={}",
              to_id
            );
          }
        }

        for (to_id, module) in moved {
          patch_lock.sampleables.insert(to_id, module);
        }

        // Keep message routing in sync with any renames.
        patch_lock.rebuild_message_listeners();

        patch_dbg!(
          "[apply_patch] remap applied moved={} overwrites={} skipped_reserved={} skipped_identity={} missing_source={}",
          module_id_remaps.len() - remap_skipped_reserved - remap_skipped_identity,
          remap_overwrites,
          remap_skipped_reserved,
          remap_skipped_identity,
          remap_missing_source
        );
      }
    }
    // Build maps for efficient lookup
    let desired_modules: HashMap<String, _> = modules.iter().map(|m| (m.id.clone(), m)).collect();

    let current_ids: HashSet<String> = patch_lock.sampleables.keys().cloned().collect();
    let desired_ids: HashSet<String> = desired_modules.keys().cloned().collect();
    patch_dbg!(
      "[apply_patch] modules current={} desired={} current_sample=[{}] desired_sample=[{}]",
      current_ids.len(),
      desired_ids.len(),
      format_id_set_sample(&current_ids, 12),
      format_id_set_sample(&desired_ids, 12)
    );

    // Find modules to delete (in current but not in desired), excluding root
    let mut to_delete: Vec<String> = current_ids
      .difference(&desired_ids)
      .filter(|id| {
        *id != WellKnownModule::RootOutput.id()
          && *id != WellKnownModule::RootClock.id()
          && *id != WellKnownModule::RootInput.id()
          && *id != WellKnownModule::HiddenAudioIn.id()
      })
      .cloned()
      .collect();
    if apply_patch_debug_enabled() {
      let mut sample = to_delete.clone();
      sample.sort();
      sample.truncate(12);
      patch_dbg!(
        "[apply_patch] delete candidates={} sample=[{}]",
        to_delete.len(),
        sample.join(", ")
      );
    }

    // Find modules where type changed (same ID but different module_type)
    // These need to be deleted and recreated
    let mut to_recreate: Vec<String> = Vec::new();
    for id in current_ids.intersection(&desired_ids) {
      if id == WellKnownModule::RootOutput.id()
        || id == WellKnownModule::RootClock.id()
        || id == WellKnownModule::RootInput.id()
        || id == WellKnownModule::HiddenAudioIn.id()
      {
        continue; // Never recreate root_output, root_clock, root_input, or hidden_audio_in
      }
      if let (Some(current_module), Some(desired_module)) =
        (patch_lock.sampleables.get(id), desired_modules.get(id))
      {
        if current_module.get_module_type() != &desired_module.module_type {
          to_recreate.push(id.clone());
          to_delete.push(id.clone());
        }
      }
    }

    patch_dbg!(
      "[apply_patch] delete final={} recreate={} ",
      to_delete.len(),
      to_recreate.len()
    );

    // Find modules to create (in desired but not in current, plus recreated modules)
    let mut to_create: Vec<String> = desired_ids.difference(&current_ids).cloned().collect();
    to_create.extend(to_recreate);

    if apply_patch_debug_enabled() {
      let mut create_sample = to_create.clone();
      create_sample.sort();
      create_sample.truncate(12);
      patch_dbg!(
        "[apply_patch] create count={} sample=[{}]",
        to_create.len(),
        create_sample.join(", ")
      );
    } else {
      // Keep a minimal signal for normal operation.
      // (No stdout spam; only visible when explicitly enabled.)
    }

    // Delete modules
    for id in to_delete {
      patch_lock.sampleables.remove(&id);
    }

    // Create new modules
    let constructors = get_constructors();
    for id in &to_create {
      if let Some(desired_module) = desired_modules.get(id) {
        if let Some(constructor) = constructors.get(&desired_module.module_type) {
          match constructor(id, sample_rate) {
            Ok(module) => {
              patch_lock.sampleables.insert(id.clone(), module);
            }
            Err(err) => {
              return Err(napi::Error::from_reason(format!(
                "Failed to create module {}: {}",
                id, err
              )));
            }
          }
        } else {
          return Err(napi::Error::from_reason(format!(
            "{} is not a valid module type",
            desired_module.module_type
          )));
        }
      }
    }

    // Keep message routing in sync with current modules.
    patch_lock.rebuild_message_listeners();

    // ===== SCOPE LIFECYCLE =====
    {
      let mut scope_collection = self.scope_collection.lock();
      let current_scope_items: HashSet<ScopeItem> = scope_collection.keys().cloned().collect();
      let desired_scopes: HashMap<ScopeItem, Scope> =
        scopes.into_iter().map(|s| (s.item.clone(), s)).collect();
      let desired_scope_items: HashSet<ScopeItem> = desired_scopes.keys().cloned().collect();
      // Remove scopes that are in current but not in desired
      let scopes_to_remove: Vec<ScopeItem> = current_scope_items
        .difference(&desired_scope_items)
        .cloned()
        .collect();

      patch_dbg!(
        "[apply_patch] scopes remove count={}",
        scopes_to_remove.len()
      );

      for scope_item in scopes_to_remove {
        scope_collection.remove(&scope_item);
      }

      // Add scopes that are in desired but not in current
      let scopes_to_add: Vec<Scope> = desired_scope_items
        .difference(&current_scope_items)
        .filter_map(|item| desired_scopes.get(item))
        .cloned()
        .collect();

      patch_dbg!("[apply_patch] scopes add count={}", scopes_to_add.len());
      const SCOPE_SIZE: u32 = 256;
      for scope in scopes_to_add {
        scope_collection.insert(scope.item.clone(), ScopeBuffer::new(&scope, sample_rate));
      }

      let scopes_to_update: Vec<Scope> = desired_scope_items
        .intersection(&current_scope_items)
        .filter_map(|item| desired_scopes.get(item))
        .cloned()
        .collect();

      // Update existing scopes' parameters
      for scope in scopes_to_update {
        if let Some(existing_scope) = scope_collection.get_mut(&scope.item) {
          existing_scope.update(&scope, sample_rate);
        }
      }

      patch_dbg!("[apply_patch] scopes active={}", scope_collection.len());
    }

    // Update parameters for all desired modules (both new and existing)
    // Note: params are now a single JSON object, deserialized into each module's
    // strongly-typed Params struct by `Sampleable::try_update_params`.
    for id in desired_ids.iter() {
      if let Some(desired_module) = desired_modules.get(id) {
        if let Some(module) = patch_lock.sampleables.get(id) {
          if let Err(err) = module.try_update_params(desired_module.params.clone()) {
            return Err(napi::Error::from_reason(format!(
              "Failed to update params for {}: {}",
              id, err
            )));
          }
        }
      }
    }
    for sampleable in patch_lock.sampleables.values() {
      sampleable.connect(&patch_lock);
    }

    for sampleable in patch_lock.sampleables.values() {
      sampleable.on_patch_update();
    }

    Ok(())
  }

  pub fn handle_set_patch(&self, patch: PatchGraph, sample_rate: f32) -> Vec<ApplyPatchError> {
    // Validate patch
    let schemas = schema();
    if let Err(errors) = validate_patch(&patch, &schemas) {
      return vec![ApplyPatchError {
        message: "Validation failed".to_string(),
        errors: Some(errors),
      }];
    }

    // If stopped, fully recreate patch state to avoid reusing module instances.
    if self.is_stopped() {
      {
        let mut patch_lock = self.patch.lock();
        patch_lock.sampleables.clear();
        patch_lock.rebuild_message_listeners();
      }

      let mut scope_collection = self.scope_collection.lock();
      scope_collection.clear();
    }

    // Apply patch
    if let Err(e) = self.apply_patch(patch, sample_rate) {
      return vec![ApplyPatchError {
        message: format!("Failed to apply patch: {}", e),
        errors: None,
      }];
    }
    let mut responses: Vec<ApplyPatchError> = vec![];
    // Auto-unmute on SetPatch to match prior imperative flow
    if self.is_stopped() {
      self.set_stopped(false);
      let message = Message::Clock(ClockMessages::Start);
      if let Err(e) = self.patch.lock().dispatch_message(&message) {
        responses.push(ApplyPatchError {
          message: format!("Failed to dispatch start: {}", e),
          errors: None,
        })
      }
    }
    return responses;
  }
}

fn chrono_simple_timestamp() -> String {
  use std::time::{SystemTime, UNIX_EPOCH};
  let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
  format!("{}", duration.as_secs())
}

pub fn make_stream<T>(
  device: &cpal::Device,
  config: &cpal::StreamConfig,
  audio_state: &Arc<AudioState>,
  input_buffer: Option<SharedInputBuffer>,
) -> Result<cpal::Stream>
where
  T: SizedSample + FromSample<f32> + hound::Sample,
{
  let num_channels = config.channels as usize;
  let _sample_rate_hz = config.sample_rate as f64;

  let err_fn = |err| eprintln!("Error building output sound stream: {err}");

  let time_at_start = std::time::Instant::now();
  println!("Time at start: {time_at_start:?}");
  let audio_state = audio_state.clone();

  let mut final_state_processor = FinalStateProcessor::new(num_channels);

  let stream = device
    .build_output_stream(
      config,
      move |output: &mut [T], _info: &cpal::OutputCallbackInfo| {
        let callback_start = Instant::now();

        // Dispatch any pending MIDI messages to the patch
        audio_state.dispatch_midi_messages();

        for frame in output.chunks_mut(num_channels) {
          {
            let patch = audio_state.patch.lock();
            let mut audio_in = patch.audio_in.lock();
            let input_samples = input_buffer
              .as_ref()
              .map(|ib| ib.read_latest())
              .unwrap_or([0f32; PORT_MAX_CHANNELS]);
            for i in 0..num_channels.min(PORT_MAX_CHANNELS) {
              audio_in.set(i, input_samples[i]);
            }
          }

          // Process frame and get multi-channel output
          let samples =
            final_state_processor.process_frame_multichannel(&audio_state, num_channels);

          for (ch, s) in frame.iter_mut().enumerate() {
            if ch < samples.len() {
              *s = T::from_sample(samples[ch]);
            } else {
              *s = T::from_sample(0.0);
            }
          }

          // Record if enabled (use try_lock to avoid blocking audio)
          // For multi-channel, record first channel (mono mix could be added later)
          if let Some(mut writer_guard) = audio_state.recording_writer.try_lock() {
            if let Some(ref mut writer) = *writer_guard {
              let _ = writer.write_sample(T::from_sample(samples[0]));
            }
          }
        }

        let elapsed_ns = callback_start.elapsed().as_nanos() as u64;

        audio_state
          .audio_budget_meter
          .record_chunk(output.len() as u64, elapsed_ns);
      },
      err_fn,
      None,
    )
    .map_err(|e| napi::Error::from_reason(format!("Failed to build output stream: {}", e)))?;

  Ok(stream)
}

/// Build an input stream that writes to a shared ring buffer
pub fn make_input_stream<T>(
  device: &cpal::Device,
  config: &cpal::StreamConfig,
  input_buffer: SharedInputBuffer,
  channels: usize,
) -> Result<cpal::Stream>
where
  T: SizedSample + cpal::Sample,
  f32: FromSample<T>,
{
  let err_fn = |err| eprintln!("Error building input sound stream: {err}");

  let stream = device
    .build_input_stream(
      config,
      move |data: &[T], _info: &cpal::InputCallbackInfo| {
        // Convert to f32 and write to ring buffer
        let f32_data: Vec<f32> = data.iter().map(|&s| f32::from_sample(s)).collect();
        input_buffer.write_interleaved(&f32_data, channels);
      },
      err_fn,
      None,
    )
    .map_err(|e| napi::Error::from_reason(format!("Failed to build input stream: {}", e)))?;

  Ok(stream)
}

/// Process a single frame and return samples for all output channels
fn process_frame_multichannel(
  audio_state: &Arc<AudioState>,
  num_channels: usize,
) -> [f32; PORT_MAX_CHANNELS] {
  use modular_core::poly::PORT_MAX_CHANNELS;
  use modular_core::types::ROOT_ID;

  let mut output = [0.0f32; PORT_MAX_CHANNELS];

  let patch_guard = audio_state.patch.lock();

  // Update sampleables
  for (_, module) in patch_guard.sampleables.iter() {
    module.update();
  }

  // Tick sampleables
  for (_, module) in patch_guard.sampleables.iter() {
    module.tick();
  }

  // Capture audio for scopes
  for (scope, scope_buffer) in audio_state.scope_collection.lock().iter_mut() {
    match scope {
      ScopeItem::ModuleOutput {
        module_id,
        port_name,
        ..
      } => {
        if let Some(module) = patch_guard.sampleables.get(module_id) {
          if let Ok(poly) = module.get_poly_sample(&port_name) {
            scope_buffer.push(poly.get(0));
          }
        }
      }
    }
  }

  // Get output from root module
  if let Some(root) = patch_guard.sampleables.get(&*ROOT_ID) {
    if let Ok(poly) = root.get_poly_sample(&ROOT_OUTPUT_PORT) {
      // Multi-channel: map poly channels to output channels
      for ch in 0..num_channels.min(PORT_MAX_CHANNELS) {
        output[ch] = poly.get(ch) * AUDIO_OUTPUT_ATTENUATION;
      }
    }
  }

  output
}

fn process_frame(audio_state: &Arc<AudioState>) -> f32 {
  use modular_core::types::ROOT_ID;

  // Try to acquire patch lock - if we can't, skip this frame to avoid blocking audio
  // let patch_guard = match audio_state.patch.try_lock() {
  //   Some(guard) => guard,
  //   None => {
  //     audio_state
  //       .audio_thread_health
  //       .patch_lock_misses
  //       .fetch_add(1, Ordering::Relaxed);
  //     return 0.0;
  //   }
  // };

  let patch_guard = audio_state.patch.lock();

  // Update sampleables
  for (_, module) in patch_guard.sampleables.iter() {
    module.update();
  }

  // Tick sampleables
  for (_, module) in patch_guard.sampleables.iter() {
    module.tick();
  }

  // Capture audio for scopes
  for (scope, scope_buffer) in audio_state.scope_collection.lock().iter_mut() {
    match scope {
      ScopeItem::ModuleOutput {
        module_id,
        port_name,
        ..
      } => {
        if let Some(module) = patch_guard.sampleables.get(module_id) {
          if let Ok(poly) = module.get_poly_sample(&port_name) {
            scope_buffer.push(poly.get(0));
          }
        }
      }
    }
  }

  // Get output sample before dropping lock
  let output_sample = if let Some(root) = patch_guard.sampleables.get(&*ROOT_ID) {
    root
      .get_poly_sample(&ROOT_OUTPUT_PORT)
      .map(|p| p.get(0))
      .unwrap_or(0.0)
  } else {
    0.0
  };

  output_sample
}

pub fn get_host_by_preference() -> Host {
  #[cfg(target_os = "windows")]
  {
    if let Ok(asio_host) = cpal::host_from_id(HostId::Asio) {
      println!("Using ASIO");
      return asio_host;
    }

    // Fall back to WASAPI
    if let Ok(wasapi) = cpal::host_from_id(HostId::Wasapi) {
      println!("Using WASAPI");
      return wasapi;
    }
  }

  #[cfg(target_os = "macos")]
  {
    // Try CoreAudio on macOS
    if let Ok(coreaudio_host) = cpal::host_from_id(HostId::CoreAudio) {
      println!("Using CoreAudio");
      return coreaudio_host;
    }
  }

  #[cfg(target_os = "linux")]
  {
    if let Ok(jack_host) = cpal::host_from_id(HostId::Jack) {
      println!("Using JACK");
      return jack_host;
    }

    // Try ALSA on Linux
    if let Ok(alsa_host) = cpal::host_from_id(HostId::Alsa) {
      println!("Using ALSA");
      return alsa_host;
    }
  }

  // Fallback to the default host
  let default_host = cpal::default_host();
  println!("Using default host: {:?}", default_host.id());
  default_host
}

/// Get the sample rate from the default audio device
pub fn get_sample_rate() -> Result<f32> {
  let host = get_host_by_preference();
  let device = host
    .default_output_device()
    .ok_or_else(|| napi::Error::from_reason("No audio output device found".to_string()))?;
  let config = device
    .default_output_config()
    .map_err(|e| napi::Error::from_reason(format!("Failed to get default output config: {}", e)))?;
  Ok(config.sample_rate() as f32)
}

enum VolumeChange {
  Decrease,
  None,
}
struct FinalStateProcessor {
  attenuation_factor: f32,
  volume_change: VolumeChange,
  prev_is_stopped: bool,
  num_channels: usize,
}

impl FinalStateProcessor {
  fn new(num_channels: usize) -> Self {
    Self {
      attenuation_factor: 0.0,
      volume_change: VolumeChange::None,
      prev_is_stopped: true,
      num_channels,
    }
  }

  /// Process frame and return multi-channel output
  fn process_frame_multichannel(
    &mut self,
    audio_state: &Arc<AudioState>,
    num_channels: usize,
  ) -> [f32; PORT_MAX_CHANNELS] {
    let is_stopped = audio_state.is_stopped();
    match (self.prev_is_stopped, is_stopped) {
      (true, false) => {
        self.volume_change = VolumeChange::None;
        self.attenuation_factor = 1.0;
      }
      (false, true) => {
        self.volume_change = VolumeChange::Decrease;
      }
      _ => {}
    }
    self.prev_is_stopped = is_stopped;

    match self.volume_change {
      VolumeChange::Decrease => {
        self.attenuation_factor *= 0.999;
        if self.attenuation_factor < 0.0001 {
          self.attenuation_factor = 0.0;
          self.volume_change = VolumeChange::None;
        }
      }
      VolumeChange::None => {}
    }

    let mut output = [0.0f32; PORT_MAX_CHANNELS];

    if self.attenuation_factor < f32::EPSILON {
      return output;
    }

    let raw_output = process_frame_multichannel(audio_state, num_channels);

    // Apply attenuation and soft clipping to all channels
    let mut any_audible = false;
    for ch in 0..num_channels.min(PORT_MAX_CHANNELS) {
      let sample = (raw_output[ch] * self.attenuation_factor).tanh();
      output[ch] = sample;
      if sample.abs() >= 0.0005 {
        any_audible = true;
      }
    }

    // When stopped and all channels are silent, fully mute
    if is_stopped && !any_audible {
      self.attenuation_factor = 0.0;
      self.volume_change = VolumeChange::None;
      return [0.0f32; PORT_MAX_CHANNELS];
    }

    output
  }
}

#[derive(Debug, Clone)]
#[napi(object)]
pub struct AudioBudgetSnapshot {
  pub total_samples: napi::bindgen_prelude::BigInt,
  pub total_time_ns: napi::bindgen_prelude::BigInt,

  /// Average nanoseconds per sample over snapshot window
  pub avg_ns_per_sample: f64,

  /// Average real-time usage (1.0 == real-time)
  pub avg_usage: f64,

  /// Worst-case nanoseconds per sample (peak density)
  pub peak_ns_per_sample: f64,

  /// Worst-case real-time usage (1.0 == real-time)
  pub peak_usage: f64,
}

#[derive(Debug, Default)]
pub struct AudioBudgetMeter {
  total_samples: AtomicU64,
  total_time_ns: AtomicU64,

  /// Q32 fixed-point: (ns / sample)
  max_ns_per_sample_q32: AtomicU64,
}

impl AudioBudgetMeter {
  pub const fn new() -> Self {
    Self {
      total_samples: AtomicU64::new(0),
      total_time_ns: AtomicU64::new(0),
      max_ns_per_sample_q32: AtomicU64::new(0),
    }
  }

  /// Call from audio callback
  #[inline(always)]
  pub fn record_chunk(&self, samples: u64, time_ns: u64) {
    if samples == 0 {
      return;
    }

    self.total_samples.fetch_add(samples, Ordering::Relaxed);
    self.total_time_ns.fetch_add(time_ns, Ordering::Relaxed);

    let ns_per_sample_q32 = (time_ns << 32) / samples;

    let mut prev = self.max_ns_per_sample_q32.load(Ordering::Relaxed);

    while ns_per_sample_q32 > prev {
      match self.max_ns_per_sample_q32.compare_exchange_weak(
        prev,
        ns_per_sample_q32,
        Ordering::Relaxed,
        Ordering::Relaxed,
      ) {
        Ok(_) => break,
        Err(v) => prev = v,
      }
    }
  }

  /// Call from non-audio thread
  pub fn take_snapshot(&self, sample_rate: f64, channels: f64) -> AudioBudgetSnapshot {
    let total_samples = self.total_samples.swap(0, Ordering::Relaxed);
    let total_time_ns = self.total_time_ns.swap(0, Ordering::Relaxed);
    let max_q32 = self.max_ns_per_sample_q32.swap(0, Ordering::Relaxed);

    let budget_ns_per_sample = 1e9 / (sample_rate * channels);

    let avg_ns_per_sample = if total_samples > 0 {
      total_time_ns as f64 / total_samples as f64
    } else {
      0.0
    };

    let peak_ns_per_sample = (max_q32 as f64) / (1u64 << 32) as f64;

    AudioBudgetSnapshot {
      total_samples: napi::bindgen_prelude::BigInt::from(total_samples),
      total_time_ns: napi::bindgen_prelude::BigInt::from(total_time_ns),

      avg_ns_per_sample,
      avg_usage: avg_ns_per_sample / budget_ns_per_sample,

      peak_ns_per_sample,
      peak_usage: peak_ns_per_sample / budget_ns_per_sample,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use modular_core::types::{ModuleIdRemap, ModuleState};
  use parking_lot::Mutex;
  use serde_json::json;

  // #[test]
  // fn test_audio_subscription() {
  //   let patch = Arc::new(Mutex::new(Patch::new(HashMap::new(), HashMap::new())));
  //   let state = AudioState::new(patch, 48000.0, 2);
  //   let sub = ScopeItem::ModuleOutput {
  //     module_id: "sine-1".to_string(),
  //     port_name: "output".to_string(),
  //     speed: 0,
  //   };

  //   state.add_scope(sub.clone());

  //   assert!(
  //     state
  //       .scope_collection
  //       .try_lock()
  //       .unwrap()
  //       .contains_key(&sub)
  //   );
  //   state.remove_scope(&sub);
  //   assert!(
  //     !state
  //       .scope_collection
  //       .try_lock()
  //       .unwrap()
  //       .contains_key(&sub)
  //   );
  // }

  #[test]
  fn test_stopped_state() {
    let patch = Arc::new(Mutex::new(Patch::new()));
    let state = AudioState::new(patch, 48000.0, 2);

    // Initially stopped
    assert!(state.is_stopped());
    state.set_stopped(false);
    assert!(!state.is_stopped());
    state.set_stopped(true);
    assert!(state.is_stopped());
  }

  #[test]
  fn test_apply_patch_module_id_remaps_reuse_instance() {
    let patch = Arc::new(Mutex::new(Patch::new()));
    let state = AudioState::new(patch.clone(), 48000.0, 2);

    state
      .apply_patch(
        PatchGraph {
          modules: vec![ModuleState {
            id: "sine-1".to_string(),
            module_type: "sine".to_string(),
            id_is_explicit: None,
            params: json!({}),
          }],
          module_id_remaps: None,

          scopes: vec![],
        },
        48000.0,
      )
      .unwrap();

    let ptr_before = {
      let patch_lock = patch.lock();
      let module = patch_lock.sampleables.get("sine-1").unwrap();
      Arc::as_ptr(module) as usize
    };

    state
      .apply_patch(
        PatchGraph {
          modules: vec![ModuleState {
            id: "sine-2".to_string(),
            module_type: "sine".to_string(),
            id_is_explicit: None,
            params: json!({}),
          }],
          module_id_remaps: Some(vec![ModuleIdRemap {
            from: "sine-1".to_string(),
            to: "sine-2".to_string(),
          }]),

          scopes: vec![],
        },
        48000.0,
      )
      .unwrap();

    let (ptr_after, has_old, has_new) = {
      let patch_lock = patch.lock();
      let has_old = patch_lock.sampleables.contains_key("sine-1");
      let has_new = patch_lock.sampleables.contains_key("sine-2");
      let module = patch_lock.sampleables.get("sine-2").unwrap();
      (Arc::as_ptr(module) as usize, has_old, has_new)
    };

    assert!(!has_old);
    assert!(has_new);
    assert_eq!(ptr_before, ptr_after);
  }

  #[test]
  fn test_apply_patch_module_id_remaps_chain_reuse_instances() {
    let patch = Arc::new(Mutex::new(Patch::new()));
    let state = AudioState::new(patch.clone(), 48000.0, 2);

    state
      .apply_patch(
        PatchGraph {
          modules: vec![
            ModuleState {
              id: "sine-1".to_string(),
              module_type: "sine".to_string(),
              id_is_explicit: None,
              params: json!({}),
            },
            ModuleState {
              id: "sine-2".to_string(),
              module_type: "sine".to_string(),
              id_is_explicit: None,
              params: json!({}),
            },
          ],
          module_id_remaps: None,

          scopes: vec![],
        },
        48000.0,
      )
      .unwrap();

    let (ptr_1_before, ptr_2_before) = {
      let patch_lock = patch.lock();
      let m1 = patch_lock.sampleables.get("sine-1").unwrap();
      let m2 = patch_lock.sampleables.get("sine-2").unwrap();
      (Arc::as_ptr(m1) as usize, Arc::as_ptr(m2) as usize)
    };

    // Desired ids shift up: sine-2 should reuse old sine-1, sine-3 should reuse old sine-2.
    state
      .apply_patch(
        PatchGraph {
          modules: vec![
            ModuleState {
              id: "sine-2".to_string(),
              module_type: "sine".to_string(),
              id_is_explicit: None,
              params: json!({}),
            },
            ModuleState {
              id: "sine-3".to_string(),
              module_type: "sine".to_string(),
              id_is_explicit: None,
              params: json!({}),
            },
          ],
          module_id_remaps: Some(vec![
            ModuleIdRemap {
              from: "sine-1".to_string(),
              to: "sine-2".to_string(),
            },
            ModuleIdRemap {
              from: "sine-2".to_string(),
              to: "sine-3".to_string(),
            },
          ]),

          scopes: vec![],
        },
        48000.0,
      )
      .unwrap();

    let (ptr_2_after, ptr_3_after, has_1, has_2, has_3) = {
      let patch_lock = patch.lock();
      let has_1 = patch_lock.sampleables.contains_key("sine-1");
      let has_2 = patch_lock.sampleables.contains_key("sine-2");
      let has_3 = patch_lock.sampleables.contains_key("sine-3");
      let m2 = patch_lock.sampleables.get("sine-2").unwrap();
      let m3 = patch_lock.sampleables.get("sine-3").unwrap();
      (
        Arc::as_ptr(m2) as usize,
        Arc::as_ptr(m3) as usize,
        has_1,
        has_2,
        has_3,
      )
    };

    assert!(!has_1);
    assert!(has_2);
    assert!(has_3);
    assert_eq!(ptr_1_before, ptr_2_after);
    assert_eq!(ptr_2_before, ptr_3_after);
  }

  #[test]
  fn test_apply_patch_module_id_remaps_shift_down_drops_destination_instance() {
    let patch = Arc::new(Mutex::new(Patch::new()));
    let state = AudioState::new(patch.clone(), 48000.0, 2);

    state
      .apply_patch(
        PatchGraph {
          modules: vec![
            ModuleState {
              id: "sine-1".to_string(),
              module_type: "sine".to_string(),
              id_is_explicit: None,
              params: json!({}),
            },
            ModuleState {
              id: "sine-2".to_string(),
              module_type: "sine".to_string(),
              id_is_explicit: None,
              params: json!({}),
            },
            ModuleState {
              id: "sine-3".to_string(),
              module_type: "sine".to_string(),
              id_is_explicit: None,
              params: json!({}),
            },
          ],
          module_id_remaps: None,

          scopes: vec![],
        },
        48000.0,
      )
      .unwrap();

    let (ptr_1_before, ptr_2_before, ptr_3_before) = {
      let patch_lock = patch.lock();
      let m1 = patch_lock.sampleables.get("sine-1").unwrap();
      let m2 = patch_lock.sampleables.get("sine-2").unwrap();
      let m3 = patch_lock.sampleables.get("sine-3").unwrap();
      (
        Arc::as_ptr(m1) as usize,
        Arc::as_ptr(m2) as usize,
        Arc::as_ptr(m3) as usize,
      )
    };

    // Desired ids shift down: sine-1 should reuse old sine-2, sine-2 should reuse old sine-3.
    // Old sine-1 is intentionally dropped.
    state
      .apply_patch(
        PatchGraph {
          modules: vec![
            ModuleState {
              id: "sine-1".to_string(),
              module_type: "sine".to_string(),
              id_is_explicit: None,
              params: json!({}),
            },
            ModuleState {
              id: "sine-2".to_string(),
              module_type: "sine".to_string(),
              id_is_explicit: None,
              params: json!({}),
            },
          ],
          module_id_remaps: Some(vec![
            ModuleIdRemap {
              from: "sine-2".to_string(),
              to: "sine-1".to_string(),
            },
            ModuleIdRemap {
              from: "sine-3".to_string(),
              to: "sine-2".to_string(),
            },
          ]),

          scopes: vec![],
        },
        48000.0,
      )
      .unwrap();

    let (ptr_1_after, ptr_2_after, has_1, has_2, has_3) = {
      let patch_lock = patch.lock();
      let has_1 = patch_lock.sampleables.contains_key("sine-1");
      let has_2 = patch_lock.sampleables.contains_key("sine-2");
      let has_3 = patch_lock.sampleables.contains_key("sine-3");
      let m1 = patch_lock.sampleables.get("sine-1").unwrap();
      let m2 = patch_lock.sampleables.get("sine-2").unwrap();
      (
        Arc::as_ptr(m1) as usize,
        Arc::as_ptr(m2) as usize,
        has_1,
        has_2,
        has_3,
      )
    };

    assert!(has_1);
    assert!(has_2);
    assert!(!has_3);
    assert_eq!(ptr_2_before, ptr_1_after);
    assert_eq!(ptr_3_before, ptr_2_after);
    // And importantly, old sine-1 instance did NOT survive at sine-1.
    assert_ne!(ptr_1_before, ptr_1_after);
  }
}
