use cpal::FromSample;
use cpal::SizedSample;
use cpal::traits::{DeviceTrait, HostTrait};

use hound::{WavSpec, WavWriter};
use modular_core::PatchGraph;
use modular_core::dsp::get_constructors;
use modular_core::dsp::schema;
use modular_core::dsp::utils::SchmittState;
use modular_core::dsp::utils::SchmittTrigger;
use modular_core::types::ClockMessages;
use modular_core::types::Message;
use modular_core::types::Scope;
use napi::Result;
use napi::bindgen_prelude::Float32Array;
use napi_derive::napi;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicU32;
use std::sync::atomic::{AtomicBool, Ordering};

use modular_core::patch::Patch;
use modular_core::types::{ROOT_OUTPUT_PORT, ScopeItem};
use std::time::Instant;

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

/// Audio subscription for streaming samples to clients
#[derive(Clone, Debug)]
pub struct RingBuffer {
  pub buffer: Vec<f32>,
  capacity: usize,
  index: usize,
}

impl RingBuffer {
  pub fn new(capacity: usize) -> Self {
    Self {
      buffer: Vec::with_capacity(capacity),
      capacity,
      index: 0,
    }
  }

  pub fn push(&mut self, value: f32) {
    if self.buffer.len() < self.capacity {
      self.buffer.push(value);
    } else {
      self.buffer[self.index] = value;
    }
    self.index = (self.index + 1) % self.capacity;
  }

  pub fn to_vec(&self) -> Vec<f32> {
    if self.buffer.is_empty() {
      return Vec::new();
    }

    let len = self.buffer.len();
    let mut vec = Vec::with_capacity(len);

    // Optimize by splitting into two slices to avoid modulo on every iteration
    if len == self.capacity {
      // Buffer is full and has wrapped - copy from index to end, then start to index
      vec.extend_from_slice(&self.buffer[self.index..]);
      vec.extend_from_slice(&self.buffer[..self.index]);
    } else {
      // Buffer not yet full - copy everything in order
      vec.extend_from_slice(&self.buffer);
    }

    vec
  }
}

/// Wrapper for a scope's ring buffer with sample rate control
pub struct ScopeBuffer {
  pub buffer: RingBuffer,
  sample_counter: u32,
  skip_rate: u32,
  trigger_threshold: Option<f32>,
  trigger: SchmittTrigger,
  holding: bool,
}

const SCOPE_CAPACITY: u32 = 256;

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
      buffer: RingBuffer::new(SCOPE_CAPACITY as usize),
      sample_counter: 0,
      skip_rate: 0,
      trigger_threshold: None,
      trigger: SchmittTrigger::new(0.0, 0.0),
      holding: false,
    };

    sb.update(scope, sample_rate);
    sb.trigger = SchmittTrigger::new(
      sb.trigger_threshold.unwrap_or(0.0),
      sb.trigger_threshold.unwrap_or(0.0) + 0.01,
    );

    sb
  }

  fn update_trigger_threshold(&mut self, threshold: Option<i32>) {
    let threshold = threshold.map(|t| (t as f32) / 1000.0);
    self.trigger_threshold = threshold;
    if let Some(thresh) = threshold {
      self.trigger.set_thresholds(thresh, thresh + 0.01);
      self.trigger.reset();
      self.holding = false;
    }
  }

  fn update_skip_rate(&mut self, ms_per_frame: u32, sample_rate: f32) {
    self.skip_rate = calculate_skip_rate(ms_to_samples(ms_per_frame, sample_rate));
  }

  pub fn push(&mut self, value: f32) {
    if self.holding {
      return;
    }
    if let Some(t) = self.trigger_threshold {
      let state = self.trigger.process(value);
      if state == SchmittState::High {
        self.holding = true;
        return;
      }
    }

    self.buffer.push(value);
  }

  pub fn update(&mut self, scope: &Scope, sample_rate: f32) {
    self.update_trigger_threshold(scope.trigger_threshold);
    self.update_skip_rate(scope.ms_per_frame, sample_rate);
  }
}

impl From<ScopeBuffer> for Float32Array {
  fn from(scope_buffer: ScopeBuffer) -> Self {
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
  sample_rate: f32,
  _channels: u16,
  audio_thread_health: AudioThreadHealth,
}

#[derive(Default)]
struct AudioThreadHealth {
  /// Number of audio frames skipped because the real-time callback could not acquire
  /// the patch lock via `try_lock()`.
  patch_lock_misses: AtomicU32,

  /// Number of output callbacks whose execution time exceeded the duration of the
  /// buffer they were asked to fill (a strong signal of underrun risk).
  output_callback_overruns: AtomicU32,
  /// Max observed overrun (elapsed - expected) in nanoseconds.
  output_callback_overrun_max_ns: AtomicU32,
  /// Max observed total callback execution time in nanoseconds.
  output_callback_duration_max_ns: AtomicU32,
}

#[derive(Debug, Clone, Copy)]
#[napi(object)]
pub struct AudioThreadHealthSnapshot {
  pub patch_lock_misses: u32,
  pub output_callback_overruns: u32,
  pub output_callback_overrun_max_ns: u32,
  pub output_callback_duration_max_ns: u32,
}

impl AudioState {
  pub fn new(patch: Arc<Mutex<Patch>>, sample_rate: f32, channels: u16) -> Self {
    Self {
      patch,
      stopped: Arc::new(AtomicBool::new(true)),
      scope_collection: Arc::new(Mutex::new(HashMap::new())),
      recording_writer: Arc::new(Mutex::new(None)),
      recording_path: Arc::new(Mutex::new(None)),
      sample_rate,
      _channels: channels,
      audio_thread_health: AudioThreadHealth::default(),
    }
  }

  pub fn take_audio_thread_health_snapshot_and_reset(&self) -> AudioThreadHealthSnapshot {
    AudioThreadHealthSnapshot {
      patch_lock_misses: self
        .audio_thread_health
        .patch_lock_misses
        .swap(0, Ordering::Relaxed),
      output_callback_overruns: self
        .audio_thread_health
        .output_callback_overruns
        .swap(0, Ordering::Relaxed),
      output_callback_overrun_max_ns: self
        .audio_thread_health
        .output_callback_overrun_max_ns
        .swap(0, Ordering::Relaxed),
      output_callback_duration_max_ns: self
        .audio_thread_health
        .output_callback_duration_max_ns
        .swap(0, Ordering::Relaxed),
    }
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
      .filter(|(_, scope_buffer)| scope_buffer.buffer.buffer.len() >= scope_buffer.buffer.capacity)
      .map(|(scope_item, scope_buffer)| {
        (
          scope_item.clone(),
          Float32Array::new(scope_buffer.buffer.to_vec()),
        )
      })
      .collect()
  }

  pub fn apply_patch(&self, desired_graph: PatchGraph, sample_rate: f32) -> Result<()> {
    let PatchGraph {
      modules,
      scopes,
      tracks,
    } = desired_graph;

    let mut patch_lock = self.patch.lock();
    // Build maps for efficient lookup
    let desired_modules: HashMap<String, _> = modules.iter().map(|m| (m.id.clone(), m)).collect();

    let current_ids: HashSet<String> = patch_lock.sampleables.keys().cloned().collect();
    let desired_ids: HashSet<String> = desired_modules.keys().cloned().collect();

    // Find modules to delete (in current but not in desired), excluding root
    let mut to_delete: Vec<String> = current_ids
      .difference(&desired_ids)
      .filter(|id| *id != "root" && *id != "root_clock")
      .cloned()
      .collect();

    // Find modules where type changed (same ID but different module_type)
    // These need to be deleted and recreated
    let mut to_recreate: Vec<String> = Vec::new();
    for id in current_ids.intersection(&desired_ids) {
      if id == "root" || id == "root_clock" {
        continue; // Never recreate root or root_clock
      }
      if let (Some(current_module), Some(desired_module)) =
        (patch_lock.sampleables.get(id), desired_modules.get(id))
      {
        if current_module.get_module_type() != desired_module.module_type {
          to_recreate.push(id.clone());
          to_delete.push(id.clone());
        }
      }
    }

    // Find modules to create (in desired but not in current, plus recreated modules)
    let mut to_create: Vec<String> = desired_ids.difference(&current_ids).cloned().collect();
    
    // Create a set of modules being recreated for efficient lookup later
    let to_recreate_set: HashSet<String> = to_recreate.iter().cloned().collect();
    to_create.extend(to_recreate);

    // === SIMILARITY MATCHING ===
    // Try to match modules that are being deleted with modules that are being created
    // of the same type, to reuse the existing module instances instead of recreating them.
    // This is important when IDs change but the module type and params are similar.
    // However, we should NOT reuse modules that are being recreated due to type change.
    let mut id_remapping: HashMap<String, String> = HashMap::new(); // old_id -> new_id
    let mut reused_modules: HashSet<String> = HashSet::new(); // old_ids that were reused

    if !to_delete.is_empty() && !to_create.is_empty() {
      // Group modules by type for efficient matching
      // Exclude modules that are being recreated due to type change
      let mut to_delete_by_type: HashMap<String, Vec<String>> = HashMap::new();
      for old_id in &to_delete {
        // Skip modules that are being recreated (same ID, different type)
        if to_recreate_set.contains(old_id) {
          continue;
        }
        if let Some(module) = patch_lock.sampleables.get(old_id) {
          to_delete_by_type
            .entry(module.get_module_type())
            .or_default()
            .push(old_id.clone());
        }
      }

      let mut to_create_by_type: HashMap<String, Vec<String>> = HashMap::new();
      for new_id in &to_create {
        // Skip modules that are being recreated (same ID, different type)
        if to_recreate_set.contains(new_id) {
          continue;
        }
        if let Some(desired_module) = desired_modules.get(new_id) {
          to_create_by_type
            .entry(desired_module.module_type.clone())
            .or_default()
            .push(new_id.clone());
        }
      }

      // For each type, try to match old modules to new modules by parameter similarity
      for (module_type, old_ids) in to_delete_by_type {
        if let Some(new_ids) = to_create_by_type.get(&module_type) {
          // Simple greedy matching: match based on parameter equality
          // We could use more sophisticated similarity metrics, but exact match is a good start
          for old_id in &old_ids {
            if reused_modules.contains(old_id) {
              continue;
            }
            
            // Get params of the old module by trying to serialize them
            // Since we can't directly get params from Sampleable, we skip this for now
            // and just match the first available module of the same type
            for new_id in new_ids {
              if id_remapping.values().any(|v| v == new_id) {
                continue; // Already mapped
              }
              
              // Check if params are similar enough
              if let Some(_desired_module) = desired_modules.get(new_id) {
                // For now, we'll reuse modules of the same type without checking params
                // A more sophisticated approach would compare params for similarity
                id_remapping.insert(old_id.clone(), new_id.clone());
                reused_modules.insert(old_id.clone());
                break;
              }
            }
          }
        }
      }
    }

    // Remove reused modules from the to_delete list
    // BUT keep modules that are being recreated (same ID, different type)
    to_delete.retain(|id| !reused_modules.contains(id) || to_recreate_set.contains(id));

    // Remove remapped modules from to_create list (they'll be renamed instead)
    // BUT keep modules that are being recreated (same ID, different type)
    let remapped_new_ids: HashSet<String> = id_remapping.values().cloned().collect();
    to_create.retain(|id| !remapped_new_ids.contains(id) || to_recreate_set.contains(id));

    // Delete modules that are not being reused
    for id in to_delete {
      patch_lock.sampleables.remove(&id);
    }

    // Rename/remap reused modules to their new IDs
    for (old_id, new_id) in &id_remapping {
      if let Some(module) = patch_lock.sampleables.remove(old_id) {
        patch_lock.sampleables.insert(new_id.clone(), module);
      }
    }

    // Create new modules that couldn't be reused
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

    // ===== TRACK LIFECYCLE =====
    // The track system is mid-refactor. Keep the legacy implementation available
    // behind a feature flag while core types stabilize.
    // #[cfg(feature = "legacy-tracks")]
    {
      // Build maps for efficient track lookup
      let desired_tracks: HashMap<String, _> = tracks.iter().map(|t| (t.id.clone(), t)).collect();

      let current_track_ids: HashSet<String> = patch_lock.tracks.keys().cloned().collect();
      let desired_track_ids: HashSet<String> = desired_tracks.keys().cloned().collect();

      // Delete removed tracks (in current but not in desired)
      let tracks_to_delete: Vec<String> = current_track_ids
        .difference(&desired_track_ids)
        .cloned()
        .collect();

      println!("Tracks to delete: {:?}", tracks_to_delete);

      for track_id in tracks_to_delete {
        patch_lock.tracks.remove(&track_id);
      }

      // Two-pass track creation to handle keyframes that reference other tracks

      // PASS 1: Create/update track shells (without configuration or keyframes)
      for track in &tracks {
        match patch_lock.tracks.get(&track.id) {
          Some(existing_track) => {
            // Existing track: clear all keyframes (will re-add in pass 2)
            println!("Updating track: {}", track.id);
            existing_track.clear_keyframes()
          }
          None => {
            // Create new track shell with a disconnected playhead param
            println!("Creating track: {}", track.id);
            let default_playhead_param = modular_core::types::Signal::Disconnected;
            let internal_track = Arc::new(modular_core::types::Track::new(
              track.id.clone(),
              default_playhead_param,
              track.interpolation_type,
            ));
            patch_lock.tracks.insert(track.id.clone(), internal_track);
          }
        }
      }

      // PASS 2: Configure tracks and add keyframes (all tracks now exist for Track param resolution)
      for track in tracks {
        if let Some(internal_track) = patch_lock.tracks.get(&track.id) {
          // Configure playhead parameter and interpolation type
          internal_track.configure(
            serde_json::from_value(track.playhead)?,
            track.interpolation_type,
          );

          // Add keyframes (params may reference other tracks, which now exist)
          for kf in track.keyframes {
            internal_track.add_keyframe(TryFrom::try_from(kf)?);
          }
        }
      }
    }

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

      println!("Scopes to remove: {:?}", scopes_to_remove);

      for scope_item in scopes_to_remove {
        scope_collection.remove(&scope_item);
      }

      // Add scopes that are in desired but not in current
      let scopes_to_add: Vec<Scope> = desired_scope_items
        .difference(&current_scope_items)
        .filter_map(|item| desired_scopes.get(item))
        .cloned()
        .collect();

      println!("Scopes to add: {:?}", scopes_to_add);
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

      println!(
        "Current scopes after update: {:?}",
        scope_collection.keys().collect::<Vec<_>>()
      );
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
    for track in patch_lock.tracks.values() {
      track.connect(&patch_lock);
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
) -> Result<cpal::Stream>
where
  T: SizedSample + FromSample<f32> + hound::Sample,
{
  let num_channels = config.channels as usize;
  let sample_rate_hz = config.sample_rate as f64;

  let err_fn = |err| eprintln!("Error building output sound stream: {err}");

  let time_at_start = std::time::Instant::now();
  println!("Time at start: {time_at_start:?}");
  let audio_state = audio_state.clone();

  let mut final_state_processor = FinalStateProcessor::new();

  let stream = device
    .build_output_stream(
      config,
      move |output: &mut [T], _info: &cpal::OutputCallbackInfo| {
        let callback_start = Instant::now();

        for frame in output.chunks_mut(num_channels) {
          let output_sample = T::from_sample(final_state_processor.process_frame(&audio_state));

          for s in frame.iter_mut() {
            *s = output_sample;
          }

          // Record if enabled (use try_lock to avoid blocking audio)
          if let Some(mut writer_guard) = audio_state.recording_writer.try_lock() {
            if let Some(ref mut writer) = *writer_guard {
              let _ = writer.write_sample(output_sample);
            }
          }
        }

        // Detect when the data callback itself is taking too long.
        // We compute the expected wall-time budget based on the number of frames
        // we were asked to generate and the stream sample rate.
        let elapsed = callback_start.elapsed();
        let elapsed_ns = elapsed.as_nanos() as u64;
        audio_state
          .audio_thread_health
          .output_callback_duration_max_ns
          .fetch_max(elapsed_ns as u32, Ordering::Relaxed);

        // `output.len()` is samples across all channels; convert to frames.
        let frames = (output.len() / num_channels) as f64;
        let expected_ns = ((frames * 1_000_000_000.0) / sample_rate_hz) as u64;

        if elapsed_ns > expected_ns {
          let overrun_ns = elapsed_ns - expected_ns;
          audio_state
            .audio_thread_health
            .output_callback_overruns
            .fetch_add(1, Ordering::Relaxed);
          audio_state
            .audio_thread_health
            .output_callback_overrun_max_ns
            .fetch_max(overrun_ns as u32, Ordering::Relaxed);
        }
      },
      err_fn,
      None,
    )
    .map_err(|e| napi::Error::from_reason(format!("Failed to build output stream: {}", e)))?;

  Ok(stream)
}

fn process_frame(audio_state: &Arc<AudioState>) -> f32 {
  use modular_core::types::ROOT_ID;

  // Try to acquire patch lock - if we can't, skip this frame to avoid blocking audio
  let patch_guard = match audio_state.patch.try_lock() {
    Some(guard) => guard,
    None => {
      audio_state
        .audio_thread_health
        .patch_lock_misses
        .fetch_add(1, Ordering::Relaxed);
      return 0.0;
    }
  };

  // Update tracks
  for (_, track) in patch_guard.tracks.iter() {
    track.tick();
  }

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
    // Get the speed from the scope
    let speed = scope_buffer.skip_rate;

    // Check if we should record this sample based on the counter
    if scope_buffer.sample_counter == 0 {
      match scope {
        ScopeItem::ModuleOutput {
          module_id,
          port_name,
          ..
        } => {
          if let Some(module) = patch_guard.sampleables.get(module_id) {
            if let Ok(sample) = module.get_sample(&port_name) {
              scope_buffer.push(sample);
            }
          }
        }
        ScopeItem::Track { track_id, .. } => {
          if let Some(track) = patch_guard.tracks.get(track_id) {
            if let Some(sample) = track.get_value_optional() {
              scope_buffer.push(sample);
            }
          }
        }
      }
    }

    // Increment counter and wrap based on speed
    scope_buffer.sample_counter += 1;
    if scope_buffer.sample_counter > speed {
      scope_buffer.sample_counter = 0;
    }
  }

  // Get output sample before dropping lock
  let output_sample = if let Some(root) = patch_guard.sampleables.get(&*ROOT_ID) {
    root.get_sample(&ROOT_OUTPUT_PORT).unwrap_or(0.0)
  } else {
    0.0
  };

  output_sample
}

/// Get the sample rate from the default audio device
pub fn get_sample_rate() -> Result<f32> {
  let host = cpal::default_host();
  let device = host
    .default_output_device()
    .ok_or_else(|| napi::Error::from_reason("No audio output device found".to_string()))?;
  let config = device
    .default_output_config()
    .map_err(|e| napi::Error::from_reason(format!("Failed to get default output config: {}", e)))?;
  Ok(config.sample_rate() as f32)
}

enum VolumeChange {
  Increase,
  Decrease,
  None,
}
struct FinalStateProcessor {
  attenuation_factor: f32,
  volume_change: VolumeChange,
  prev_is_stopped: bool,
}

impl FinalStateProcessor {
  fn new() -> Self {
    Self {
      attenuation_factor: 0.0,
      volume_change: VolumeChange::None,
      prev_is_stopped: true,
    }
  }

  fn process_frame(&mut self, audio_state: &Arc<AudioState>) -> f32 {
    let is_stopped = audio_state.is_stopped();
    match (self.prev_is_stopped, is_stopped) {
      (true, false) => {
        self.volume_change = VolumeChange::Increase;
        if self.attenuation_factor < f32::EPSILON {
          self.attenuation_factor = 0.01;
        }
      }
      (false, true) => {
        self.volume_change = VolumeChange::Decrease;
      }
      _ => {}
    }
    self.prev_is_stopped = is_stopped;

    match self.volume_change {
      VolumeChange::Decrease => {
        self.attenuation_factor *= 0.9;
        if self.attenuation_factor < 0.01 {
          self.attenuation_factor = 0.0;
          self.volume_change = VolumeChange::None;
        }
      }
      VolumeChange::Increase => {
        self.attenuation_factor *= 1.1;
        if self.attenuation_factor > 1.0 {
          self.attenuation_factor = 1.0;
          self.volume_change = VolumeChange::None;
        }
      }
      VolumeChange::None => {}
    }

    if self.attenuation_factor < f32::EPSILON {
      0.0
    } else {
      let sample =
        (process_frame(audio_state) * AUDIO_OUTPUT_ATTENUATION * self.attenuation_factor).tanh();

      if is_stopped && sample.abs() < f32::EPSILON {
        self.attenuation_factor = 0.0;
        self.volume_change = VolumeChange::None;
        0.0
      } else {
        sample
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use parking_lot::Mutex;

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
    let patch = Arc::new(Mutex::new(Patch::new(HashMap::new(), HashMap::new())));
    let state = AudioState::new(patch, 48000.0, 2);

    // AudioState starts in stopped state by default
    assert!(state.is_stopped());
    state.set_stopped(false);
    assert!(!state.is_stopped());
    state.set_stopped(true);
    assert!(state.is_stopped());
  }

  #[test]
  fn test_module_reuse_when_id_changes() {
    use modular_core::types::ModuleState;
    use serde_json::json;

    let patch = Arc::new(Mutex::new(Patch::new(HashMap::new(), HashMap::new())));
    let state = AudioState::new(patch.clone(), 48000.0, 2);

    // Create initial patch with a noise module
    let initial_patch = PatchGraph {
      modules: vec![ModuleState {
        id: "noise-old-id".to_string(),
        module_type: "noise".to_string(),
        params: json!({
          "color": "White",
          "gain": {"type": "volts", "value": 1.0}
        }),
      }],
      tracks: vec![],
      scopes: vec![],
    };

    // Apply the initial patch
    let result = state.apply_patch(initial_patch, 48000.0);
    assert!(result.is_ok(), "Initial patch should apply successfully");

    // Verify the module was created
    {
      let patch_lock = patch.lock();
      assert!(patch_lock.sampleables.contains_key("noise-old-id"));
      assert_eq!(
        patch_lock.sampleables.get("noise-old-id").unwrap().get_module_type(),
        "noise"
      );
    }

    // Get a reference to the original module to verify it's the same instance
    let original_module_ptr = {
      let patch_lock = patch.lock();
      Arc::as_ptr(patch_lock.sampleables.get("noise-old-id").unwrap()) as usize
    };

    // Create a new patch with the same module but different ID
    let updated_patch = PatchGraph {
      modules: vec![ModuleState {
        id: "noise-new-id".to_string(),
        module_type: "noise".to_string(),
        params: json!({
          "color": "White",
          "gain": {"type": "volts", "value": 1.0}
        }),
      }],
      tracks: vec![],
      scopes: vec![],
    };

    // Apply the updated patch
    let result = state.apply_patch(updated_patch, 48000.0);
    assert!(result.is_ok(), "Updated patch should apply successfully");

    // Verify the module was reused (same instance, new ID)
    {
      let patch_lock = patch.lock();
      assert!(!patch_lock.sampleables.contains_key("noise-old-id"), "Old ID should not exist");
      assert!(patch_lock.sampleables.contains_key("noise-new-id"), "New ID should exist");
      assert_eq!(
        patch_lock.sampleables.get("noise-new-id").unwrap().get_module_type(),
        "noise"
      );

      // Check if it's the same instance (pointer comparison)
      let new_module_ptr = Arc::as_ptr(patch_lock.sampleables.get("noise-new-id").unwrap()) as usize;
      assert_eq!(
        original_module_ptr, new_module_ptr,
        "Module instance should be reused, not recreated"
      );
    }
  }

  #[test]
  fn test_module_recreation_when_type_changes() {
    use modular_core::types::ModuleState;
    use serde_json::json;

    let patch = Arc::new(Mutex::new(Patch::new(HashMap::new(), HashMap::new())));
    let state = AudioState::new(patch.clone(), 48000.0, 2);

    // Create initial patch with a noise module
    let initial_patch = PatchGraph {
      modules: vec![ModuleState {
        id: "module-1".to_string(),
        module_type: "noise".to_string(),
        params: json!({
          "color": "White",
          "gain": {"type": "volts", "value": 1.0}
        }),
      }],
      tracks: vec![],
      scopes: vec![],
    };

    let result = state.apply_patch(initial_patch, 48000.0);
    assert!(result.is_ok());

    // Verify initial module type
    {
      let patch_lock = patch.lock();
      assert_eq!(
        patch_lock.sampleables.get("module-1").unwrap().get_module_type(),
        "noise"
      );
    }

    // Update patch with different module type but same ID
    let updated_patch = PatchGraph {
      modules: vec![ModuleState {
        id: "module-1".to_string(),
        module_type: "sine".to_string(),
        params: json!({
          "freq": {"type": "volts", "value": 1.0},
          "phase": {"type": "volts", "value": 0.0}
        }),
      }],
      tracks: vec![],
      scopes: vec![],
    };

    let result = state.apply_patch(updated_patch, 48000.0);
    assert!(result.is_ok());

    // Verify the module was recreated (type changed)
    {
      let patch_lock = patch.lock();
      assert!(patch_lock.sampleables.contains_key("module-1"));
      // The important thing is that the type changed - pointer comparison
      // may fail due to allocator reuse, but type change proves recreation
      assert_eq!(
        patch_lock.sampleables.get("module-1").unwrap().get_module_type(),
        "sine",
        "Module type should be changed from noise to sine, proving recreation"
      );
    }
  }
}
