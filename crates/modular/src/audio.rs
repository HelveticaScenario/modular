use cpal::FromSample;
use cpal::Host;
use cpal::HostId;
use cpal::Sample;
use cpal::SizedSample;
use cpal::traits::{DeviceTrait, HostTrait};

use hound::{WavSpec, WavWriter};
use modular_core::PORT_MAX_CHANNELS;
use modular_core::PatchGraph;
use modular_core::dsp::get_constructors;
use modular_core::dsp::schema;
use modular_core::dsp::utils::SchmittTrigger;
use modular_core::types::ClockMessages;
use modular_core::types::Message;
use modular_core::types::Scope;
use modular_core::types::ScopeMode;
use modular_core::types::WellKnownModule;
use napi::Result;
use napi::bindgen_prelude::Float32Array;
use napi_derive::napi;
use parking_lot::Mutex;
use profiling;
use ringbuf::{
  HeapRb,
  traits::{Consumer, Producer, Split},
};
use rtrb::{Consumer as RtrbConsumer, Producer as RtrbProducer, RingBuffer};
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

use crate::commands::{
  AudioError, COMMAND_QUEUE_CAPACITY, ERROR_QUEUE_CAPACITY, GraphCommand, PatchUpdate,
};
use crate::midi::MidiInputManager;

// ============================================================================
// Audio Host Information
// ============================================================================

/// Information about an audio host
#[derive(Debug, Clone)]
#[napi(object)]
pub struct HostInfo {
  /// Host identifier (e.g., "CoreAudio", "WASAPI", "ALSA")
  pub id: String,
  /// Human-readable host name
  pub name: String,
}

// ============================================================================
// Audio Device Information
// ============================================================================

/// Buffer size range for an audio device
#[derive(Debug, Clone)]
#[napi(object)]
pub struct BufferSizeRange {
  pub min: u32,
  pub max: u32,
}

/// Information about an audio device
#[derive(Debug, Clone)]
#[napi(object)]
pub struct AudioDeviceInfo {
  /// Stable Device ID
  pub id: String,
  /// Host ID this device belongs to
  pub host_id: String,
  /// Device name
  pub name: String,
  /// Number of input channels (0 if output-only)
  pub input_channels: u16,
  /// Number of output channels (0 if input-only)
  pub output_channels: u16,
  /// Whether this is the default device for this host
  pub is_default: bool,
  /// Default sample rate in Hz
  pub sample_rate: u32,
  /// Supported sample rates (common rates that the device supports)
  pub supported_sample_rates: Vec<u32>,
  /// Buffer size range (min/max), or None if unknown
  pub buffer_size_range: Option<BufferSizeRange>,
}

/// Common sample rates to check for support
const COMMON_SAMPLE_RATES: &[u32] = &[44100, 48000, 88200, 96000, 176400, 192000];

/// Maximum sample rate to use as a default for new users / missing config.
/// If the OS/device reports a default above this, we pick the highest
/// supported rate at or below this cap instead.
const PREFERRED_MAX_DEFAULT_SAMPLE_RATE: u32 = 48_000;

/// Choose a sensible default sample rate for the given device.
///
/// Uses the device's cpal-reported default (`device_default`) when it is
/// at or below `PREFERRED_MAX_DEFAULT_SAMPLE_RATE`.  When the device default
/// is higher (common on macOS when Audio MIDI Setup is set to 96 kHz+),
/// we pick the highest rate from `supported_rates` that is still ≤ the cap.
/// If no supported rate is ≤ the cap (very unlikely), we fall back to the
/// device default so audio still works.
pub fn preferred_default_sample_rate(device_default: u32, supported_rates: &[u32]) -> u32 {
  if device_default <= PREFERRED_MAX_DEFAULT_SAMPLE_RATE {
    return device_default;
  }

  // Device default is too high — pick the best rate at or below the cap.
  supported_rates
    .iter()
    .copied()
    .filter(|&r| r <= PREFERRED_MAX_DEFAULT_SAMPLE_RATE)
    .max()
    .unwrap_or(device_default)
}

// ============================================================================
// Device Cache
// ============================================================================

/// Cached information about a device (includes cpal Device handle)
#[derive(Clone)]
pub struct CachedDevice {
  pub info: AudioDeviceInfo,
  // Note: cpal::Device doesn't implement Clone, so we store just the info
  // and look up the device by ID when needed
}

/// Cache of all available audio hosts and devices
#[derive(Default)]
pub struct AudioDeviceCache {
  /// All available hosts
  pub hosts: Vec<HostInfo>,
  /// Output devices keyed by host_id
  pub output_devices: HashMap<String, Vec<AudioDeviceInfo>>,
  /// Input devices keyed by host_id
  pub input_devices: HashMap<String, Vec<AudioDeviceInfo>>,
}

impl AudioDeviceCache {
  pub fn new() -> Self {
    let mut cache = Self::default();
    cache.refresh();
    cache
  }

  /// Refresh the cache by enumerating all hosts and their devices
  pub fn refresh(&mut self) {
    self.hosts.clear();
    self.output_devices.clear();
    self.input_devices.clear();

    for host_id in cpal::available_hosts() {
      let host_id_str = format!("{:?}", host_id);

      self.hosts.push(HostInfo {
        id: host_id_str.clone(),
        name: host_id_str.clone(),
      });

      if let Ok(host) = cpal::host_from_id(host_id) {
        // Get output devices for this host
        let output_devices = enumerate_output_devices(&host, &host_id_str);
        self
          .output_devices
          .insert(host_id_str.clone(), output_devices);

        // Get input devices for this host
        let input_devices = enumerate_input_devices(&host, &host_id_str);
        self.input_devices.insert(host_id_str, input_devices);
      }
    }
  }

  /// Get all output devices across all hosts
  pub fn all_output_devices(&self) -> Vec<AudioDeviceInfo> {
    self.output_devices.values().flatten().cloned().collect()
  }

  /// Get all input devices across all hosts
  pub fn all_input_devices(&self) -> Vec<AudioDeviceInfo> {
    self.input_devices.values().flatten().cloned().collect()
  }

  /// Find an output device by ID
  pub fn find_output_device(&self, device_id: &str) -> Option<&AudioDeviceInfo> {
    self
      .output_devices
      .values()
      .flatten()
      .find(|d| d.id == device_id)
  }

  /// Find an input device by ID
  pub fn find_input_device(&self, device_id: &str) -> Option<&AudioDeviceInfo> {
    self
      .input_devices
      .values()
      .flatten()
      .find(|d| d.id == device_id)
  }

  /// Get output devices for a specific host
  pub fn output_devices_for_host(&self, host_id: &str) -> Vec<AudioDeviceInfo> {
    self
      .output_devices
      .get(host_id)
      .cloned()
      .unwrap_or_default()
  }

  /// Get input devices for a specific host
  pub fn input_devices_for_host(&self, host_id: &str) -> Vec<AudioDeviceInfo> {
    self.input_devices.get(host_id).cloned().unwrap_or_default()
  }

  /// Get all host IDs
  pub fn host_ids(&self) -> Vec<String> {
    self.hosts.iter().map(|h| h.id.clone()).collect()
  }
}

/// Per-host device info for the cache snapshot
#[derive(Debug, Clone)]
#[napi(object)]
pub struct HostDeviceInfo {
  pub host_id: String,
  pub host_name: String,
  pub output_devices: Vec<AudioDeviceInfo>,
  pub input_devices: Vec<AudioDeviceInfo>,
}

/// N-API compatible structure for the full device cache
#[derive(Debug, Clone)]
#[napi(object)]
pub struct DeviceCacheSnapshot {
  /// All hosts with their devices grouped together
  pub hosts: Vec<HostDeviceInfo>,
}

/// Current audio state information
#[derive(Debug, Clone)]
#[napi(object)]
pub struct CurrentAudioState {
  pub host_id: String,
  pub output_device_id: Option<String>,
  pub output_device_name: Option<String>,
  pub input_device_id: Option<String>,
  pub input_device_name: Option<String>,
  pub sample_rate: u32,
  pub buffer_size: Option<u32>,
  pub output_channels: u16,
  pub input_channels: u16,
  pub fallback_warning: Option<String>,
}

/// Extract supported sample rates and buffer size range from device configs
fn get_device_capabilities(
  configs: impl Iterator<Item = cpal::SupportedStreamConfigRange>,
) -> (Vec<u32>, Option<BufferSizeRange>) {
  let mut supported_rates = std::collections::HashSet::new();
  let mut min_buffer = u32::MAX;
  let mut max_buffer = 0u32;

  for config in configs {
    // Check which common sample rates are supported
    let min_rate = config.min_sample_rate();
    let max_rate = config.max_sample_rate();
    for &rate in COMMON_SAMPLE_RATES {
      if rate >= min_rate && rate <= max_rate {
        supported_rates.insert(rate);
      }
    }

    // Extract buffer size range
    match config.buffer_size() {
      cpal::SupportedBufferSize::Range { min, max } => {
        min_buffer = min_buffer.min(*min);
        max_buffer = max_buffer.max(*max);
      }
      cpal::SupportedBufferSize::Unknown => {}
    }
  }

  let mut rates: Vec<u32> = supported_rates.into_iter().collect();
  rates.sort();

  let buffer_range = if min_buffer <= max_buffer && max_buffer > 0 {
    Some(BufferSizeRange {
      min: min_buffer,
      max: max_buffer,
    })
  } else {
    None
  };

  (rates, buffer_range)
}

/// Enumerate output devices for a specific host
fn enumerate_output_devices(host: &Host, host_id: &str) -> Vec<AudioDeviceInfo> {
  let default_device_id = host.default_output_device().and_then(|d| d.id().ok());

  host
    .devices()
    .map(|devices| {
      devices
        .filter_map(|device| {
          let id = device.id().ok()?;
          let config = device.default_output_config().ok()?;

          // Get supported configurations
          let (supported_sample_rates, buffer_size_range) = device
            .supported_output_configs()
            .map(get_device_capabilities)
            .unwrap_or_default();

          Some(AudioDeviceInfo {
            is_default: default_device_id.as_ref() == Some(&id),
            id: id.to_string(),
            host_id: host_id.to_string(),
            name: device.description().ok()?.name().to_owned(),
            input_channels: 0,
            output_channels: config.channels(),
            sample_rate: config.sample_rate(),
            supported_sample_rates,
            buffer_size_range,
          })
        })
        .collect()
    })
    .unwrap_or_default()
}

/// Enumerate input devices for a specific host
fn enumerate_input_devices(host: &Host, host_id: &str) -> Vec<AudioDeviceInfo> {
  let default_device_id = host.default_input_device().and_then(|d| d.id().ok());

  host
    .input_devices()
    .map(|devices| {
      devices
        .filter_map(|device| {
          let id = device.id().ok()?;
          let config = device.default_input_config().ok()?;

          // Get supported configurations
          let (supported_sample_rates, buffer_size_range) = device
            .supported_input_configs()
            .map(get_device_capabilities)
            .unwrap_or_default();

          Some(AudioDeviceInfo {
            is_default: default_device_id.as_ref() == Some(&id),
            id: id.to_string(),
            host_id: host_id.to_string(),
            name: device.description().ok()?.name().to_owned(),
            input_channels: config.channels(),
            output_channels: 0,
            sample_rate: config.sample_rate(),
            supported_sample_rates,
            buffer_size_range,
          })
        })
        .collect()
    })
    .unwrap_or_default()
}

// Legacy functions for backward compatibility (now use cache internally)

/// List all available audio hosts
pub fn list_available_hosts() -> Vec<HostInfo> {
  cpal::available_hosts()
    .into_iter()
    .map(|host_id| {
      let name = format!("{:?}", host_id);
      HostInfo {
        id: format!("{:?}", host_id),
        name,
      }
    })
    .collect()
}

/// List all available audio output devices (legacy - enumerates fresh)
pub fn list_output_devices() -> Vec<AudioDeviceInfo> {
  let host = get_host_by_preference();
  let host_id = format!("{:?}", host.id());
  enumerate_output_devices(&host, &host_id)
}

/// List all available audio input devices (legacy - enumerates fresh)
pub fn list_input_devices() -> Vec<AudioDeviceInfo> {
  let host = get_host_by_preference();
  let host_id = format!("{:?}", host.id());
  enumerate_input_devices(&host, &host_id)
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

/// Find an output device by id in a specific host
pub fn find_output_device_in_host(host: &Host, id: &str) -> Option<cpal::Device> {
  host
    .output_devices()
    .ok()?
    .find(|d| d.id().ok() == cpal::DeviceId::from_str(id).ok())
}

/// Find an input device by id in a specific host
pub fn find_input_device_in_host(host: &Host, id: &str) -> Option<cpal::Device> {
  host
    .input_devices()
    .ok()?
    .find(|d| d.id().ok() == cpal::DeviceId::from_str(id).ok())
}

// ============================================================================
// Audio Input Ring Buffer (using ringbuf crate)
// ============================================================================

/// Ring buffer size for audio input (in frames, where each frame has PORT_MAX_CHANNELS samples)
const INPUT_RING_BUFFER_FRAMES: usize = 4096;

/// Total size of the ring buffer in samples
const INPUT_RING_BUFFER_SIZE: usize = INPUT_RING_BUFFER_FRAMES * PORT_MAX_CHANNELS;

/// Producer half of the input ring buffer (used by input stream callback)
pub type InputBufferProducer = ringbuf::HeapProd<f32>;

/// Consumer half of the input ring buffer (used by output stream callback)
pub type InputBufferConsumer = ringbuf::HeapCons<f32>;

/// Writer for input audio - owns the producer, moved into input stream closure
pub struct InputBufferWriter {
  producer: InputBufferProducer,
}

impl InputBufferWriter {
  /// Write interleaved samples to the ring buffer
  pub fn write(&mut self, data: &[f32]) {
    for &sample in data {
      // Drop samples if buffer is full (better than blocking)
      let _ = self.producer.try_push(sample);
    }
  }
}

/// Reader for input audio - owns the consumer + channel count, moved into output stream closure
pub struct InputBufferReader {
  consumer: InputBufferConsumer,
  channels: usize,
}

impl InputBufferReader {
  /// Read one frame of input audio (up to PORT_MAX_CHANNELS samples)
  pub fn read_frame(&mut self) -> [f32; PORT_MAX_CHANNELS] {
    let mut result = [0.0f32; PORT_MAX_CHANNELS];

    if self.channels == 0 {
      return result;
    }

    let samples_to_read = self.channels.min(PORT_MAX_CHANNELS);

    for i in 0..samples_to_read {
      if let Some(sample) = self.consumer.try_pop() {
        result[i] = sample;
      }
    }

    // Skip extra channels if input has more than PORT_MAX_CHANNELS
    for _ in samples_to_read..self.channels {
      let _ = self.consumer.try_pop();
    }

    result
  }
}

/// Create input ring buffer writer and reader
/// Pass writer to input stream, reader to output stream
pub fn create_input_ring_buffer(channels: usize) -> (InputBufferWriter, InputBufferReader) {
  let rb = HeapRb::<f32>::new(INPUT_RING_BUFFER_SIZE);
  let (producer, consumer) = rb.split();
  (
    InputBufferWriter { producer },
    InputBufferReader { consumer, channels },
  )
}

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
    shown.join(", ").to_string()
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

/// Gain factor applied to audio input.
/// Audio input from cpal is in the range [-1, 1]. This factor brings it into
/// the [-5, 5] volt range used by DSP modules (inverse of AUDIO_OUTPUT_ATTENUATION).
const AUDIO_INPUT_GAIN: f32 = 1.0 / AUDIO_OUTPUT_ATTENUATION;

const SCOPE_CAPACITY: u32 = 1024;

use modular_core::types::ScopeStats;

// Adapted from https://github.com/VCVRack/Fundamental/blob/e819498fd388755efcb876b37d1e33fddf4a29ac/src/Scope.cpp
pub struct ScopeBuffer {
  sample_counter: [u32; PORT_MAX_CHANNELS],
  skip_rate: u32,
  trigger_threshold: Option<(f32, ScopeMode)>,
  trigger: [SchmittTrigger; PORT_MAX_CHANNELS],
  /// Multi-channel buffers: 2 buffers per channel, used in ping-pong fashion to allow reading one buffer while filling the other
  buffers: Vec<[[f32; SCOPE_CAPACITY as usize]; 2]>,
  buffer_select: [bool; PORT_MAX_CHANNELS], // false = buffer 0 active, true = buffer 1 active
  recording: [bool; PORT_MAX_CHANNELS],     // whether currently recording for each channel
  buffer_idx: [usize; PORT_MAX_CHANNELS],
  read_idx: [usize; PORT_MAX_CHANNELS],
  /// Current number of channels being captured
  num_channels: usize,
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
      buffers: vec![[[0.0; SCOPE_CAPACITY as usize]; 2]; 1], // Start with 1 channel, 2 buffers per channel
      sample_counter: [0; PORT_MAX_CHANNELS],
      skip_rate: 0,
      trigger_threshold: None,
      trigger: [SchmittTrigger::new(0.0, 0.0); PORT_MAX_CHANNELS],
      buffer_idx: [0; PORT_MAX_CHANNELS],
      buffer_select: [false; PORT_MAX_CHANNELS],
      recording: [false; PORT_MAX_CHANNELS],
      read_idx: [0; PORT_MAX_CHANNELS],
      num_channels: 1,
    };

    sb.update(scope, sample_rate);
    for ch in 0..PORT_MAX_CHANNELS {
      sb.trigger[ch] = SchmittTrigger::new(
        sb.trigger_threshold
          .map(|(thresh, _)| thresh)
          .unwrap_or(0.0),
        sb.trigger_threshold
          .map(|(thresh, _)| thresh)
          .unwrap_or(0.0)
          + 0.001,
      );
    }

    sb
  }

  fn update_trigger_threshold(&mut self, threshold: Option<(i32, ScopeMode)>) {
    let threshold = threshold.map(|(t, mode)| ((t as f32) / 1000.0, mode));
    self.trigger_threshold = threshold;
    if let Some((thresh, _)) = threshold {
      for ch in 0..PORT_MAX_CHANNELS {
        self.trigger[ch].set_thresholds(thresh, thresh);
        self.trigger[ch].reset();
      }
    }
  }

  fn update_skip_rate(&mut self, ms_per_frame: u32, sample_rate: f32) {
    self.skip_rate = calculate_skip_rate(ms_to_samples(ms_per_frame, sample_rate));
    println!(
      "Scope skip rate updated: {} (ms/frame: {}, sample_rate: {})",
      self.skip_rate, ms_per_frame, sample_rate
    );
  }

  fn write_buffer_idx(&self, ch: usize) -> usize {
    if self.buffer_select[ch] { 1 } else { 0 }
  }

  fn read_buffer_idx(&self, ch: usize) -> usize {
    let write_buffer = self.write_buffer_idx(ch);
    let other_buffer = if write_buffer == 0 { 1 } else { 0 };
    match self.trigger_threshold {
      Some((_, ScopeMode::Wait)) => other_buffer, // Read from the buffer that is not currently being written to
      Some((_, ScopeMode::Roll)) => write_buffer, // Read from the active buffer, which is continuously recording and rolling over
      None => write_buffer,                       // No trigger mode, just return active buffer
    }
  }

  /// Push samples for all channels at once
  pub fn push_poly(&mut self, values: &[f32], num_channels: usize) {
    // Dynamically resize buffers if channel count changes
    if num_channels != self.num_channels {
      let new_count = num_channels as usize;
      if new_count > self.buffers.len() {
        // Add new channel buffers
        for _ in self.buffers.len()..new_count {
          self.buffers.push([[0.0; SCOPE_CAPACITY as usize]; 2]);
        }
      }
      self.num_channels = num_channels;
    }
    for ch in 0..num_channels {
      let current_value = values.get(ch).copied().unwrap_or(0.0);

      if self.trigger_threshold.is_none() {
        self.trigger[ch].reset();
        self.recording[ch] = true;
        self.read_idx[ch] = self.buffer_idx[ch]; // Start reading from current write position
      } else if self.trigger[ch].process(current_value) && !self.recording[ch] {
        self.trigger[ch].reset();
        self.recording[ch] = true;
        self.buffer_idx[ch] = 0;
        self.read_idx[ch] = 0; // Start reading from beginning of buffer on new trigger
        self.sample_counter[ch] = 0;
      }

      self.buffer_idx[ch] %= SCOPE_CAPACITY as usize; // Wrap buffer index
      self.read_idx[ch] %= SCOPE_CAPACITY as usize; // Wrap read index

      let write_buffer_idx = self.write_buffer_idx(ch);
      if self.recording[ch] {
        if self.sample_counter[ch] == 0 {
          // Store all channel values
          if ch < self.buffers.len() {
            self.buffers[ch][write_buffer_idx][self.buffer_idx[ch]] = current_value;
          }
          self.buffer_idx[ch] += 1;
          if self.buffer_idx[ch] >= SCOPE_CAPACITY as usize {
            match self.trigger_threshold {
              Some((_, ScopeMode::Wait)) => {
                self.recording[ch] = false; // Stop recording until next trigger
                self.buffer_select[ch] = !self.buffer_select[ch]; // Switch buffers
              }
              Some((_, ScopeMode::Roll)) => {
                self.recording[ch] = false; // Stop recording until next trigger
              }
              None => {
                // No trigger mode, keep recording continuously
              }
            }
          }
        }
        self.sample_counter[ch] += 1;
        if self.sample_counter[ch] > self.skip_rate {
          self.sample_counter[ch] = 0;
        }
      }
    }
  }

  pub fn update(&mut self, scope: &Scope, sample_rate: f32) {
    self.update_trigger_threshold(scope.trigger_threshold);
    self.update_skip_rate(scope.ms_per_frame, sample_rate);
  }

  /// Get buffers for all active channels
  pub fn get_channel_buffers(&self) -> Vec<Float32Array> {
    self
      .buffers
      .iter()
      .take(self.num_channels as usize)
      .enumerate()
      .map(|(ch, buf)| Float32Array::new(buf[self.read_buffer_idx(ch)].to_vec()))
      .collect()
  }

  /// Compute statistics across all channels
  pub fn compute_stats(&self) -> ScopeStats {
    let mut min = f32::MAX;
    let mut max = f32::MIN;

    for ch in 0..self.num_channels as usize {
      if ch < self.buffers.len() {
        for &val in self.buffers[ch][self.read_buffer_idx(ch)].iter() {
          if val < min {
            min = val;
          }
          if val > max {
            max = val;
          }
        }
      }
    }

    // Handle case where no data
    if min == f32::MAX {
      min = 0.0;
    }
    if max == f32::MIN {
      max = 0.0;
    }

    ScopeStats {
      min: min as f64,
      max: max as f64,
      peak_to_peak: (max - min) as f64,
      read_offset: self.read_idx.iter().map(|&idx| idx as u32).collect(),
    }
  }
}

// ============================================================================
// Command Queue Types
// ============================================================================

/// Producer end of the command queue (main thread → audio thread)
pub type CommandProducer = RtrbProducer<GraphCommand>;
/// Consumer end of the command queue (audio thread ← main thread)
pub type CommandConsumer = RtrbConsumer<GraphCommand>;

/// Producer end of the error queue (audio thread → main thread)
pub type ErrorProducer = RtrbProducer<AudioError>;
/// Consumer end of the error queue (main thread ← audio thread)
pub type ErrorConsumer = RtrbConsumer<AudioError>;

/// Create the command and error queues for audio thread communication
pub fn create_audio_channels() -> (
  CommandProducer,
  CommandConsumer,
  ErrorProducer,
  ErrorConsumer,
) {
  let (cmd_prod, cmd_cons) = RingBuffer::new(COMMAND_QUEUE_CAPACITY);
  let (err_prod, err_cons) = RingBuffer::new(ERROR_QUEUE_CAPACITY);
  (cmd_prod, cmd_cons, err_prod, err_cons)
}

// ============================================================================
// AudioStateHandle - Main thread side
// ============================================================================

/// Main thread handle for audio state. Sends commands to audio thread.
pub struct AudioState {
  /// Command queue producer (main thread → audio thread)
  command_tx: Mutex<CommandProducer>,
  /// Error queue consumer (main thread ← audio thread)
  error_rx: Mutex<ErrorConsumer>,
  /// Stopped flag - shared with audio thread for quick reads
  stopped: Arc<AtomicBool>,
  /// Scope collection - shared with audio thread for UI reads
  scope_collection: Arc<Mutex<HashMap<ScopeItem, ScopeBuffer>>>,
  /// Recording writer - shared with audio thread
  recording_writer: Arc<Mutex<Option<WavWriter<BufWriter<File>>>>>,
  /// Recording path
  recording_path: Arc<Mutex<Option<PathBuf>>>,
  /// Sample rate
  sample_rate: f32,
  /// Output channels
  channels: u16,
  /// Audio budget meter - written by audio thread, read by main thread
  audio_budget_meter: Arc<AudioBudgetMeter>,
  /// Module states (e.g., seq current step) - written by audio thread, read by main thread
  module_states: Arc<Mutex<HashMap<String, serde_json::Value>>>,
  /// MIDI input manager - shared with audio thread for polling
  midi_manager: Arc<MidiInputManager>,
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
  /// Create a new AudioState with command queue channels
  pub fn new_with_channels(
    command_tx: CommandProducer,
    error_rx: ErrorConsumer,
    sample_rate: f32,
    channels: u16,
    midi_manager: Arc<MidiInputManager>,
  ) -> Self {
    Self {
      command_tx: Mutex::new(command_tx),
      error_rx: Mutex::new(error_rx),
      stopped: Arc::new(AtomicBool::new(true)),
      scope_collection: Arc::new(Mutex::new(HashMap::new())),
      recording_writer: Arc::new(Mutex::new(None)),
      recording_path: Arc::new(Mutex::new(None)),
      sample_rate,
      channels,
      audio_budget_meter: Arc::new(AudioBudgetMeter::default()),
      module_states: Arc::new(Mutex::new(HashMap::new())),
      midi_manager,
    }
  }

  /// Send a command to the audio thread
  pub(crate) fn send_command(&self, cmd: GraphCommand) -> Result<()> {
    let mut tx = self.command_tx.lock();
    tx.push(cmd).map_err(|_| {
      napi::Error::from_reason("Command queue full - audio thread may be overloaded".to_string())
    })
  }

  /// Drain any errors accumulated on the audio thread
  pub fn drain_errors(&self) -> Vec<AudioError> {
    let mut rx = self.error_rx.lock();
    let mut errors = Vec::new();
    while let Ok(err) = rx.pop() {
      errors.push(err);
    }
    errors
  }

  pub fn take_audio_thread_budget_snapshot_and_reset(&self) -> AudioBudgetSnapshot {
    self
      .audio_budget_meter
      .take_snapshot(self.sample_rate as f64, self.channels as f64)
  }

  pub fn set_stopped(&self, stopped: bool) {
    self.stopped.store(stopped, Ordering::SeqCst);
    // Also send command so audio thread sees it immediately
    let cmd = if stopped {
      GraphCommand::Stop
    } else {
      GraphCommand::Start
    };
    let _ = self.send_command(cmd);
  }

  pub fn is_stopped(&self) -> bool {
    self.stopped.load(Ordering::SeqCst)
  }

  /// Get shared references for audio processor creation
  pub fn get_shared_state(&self) -> AudioSharedState {
    AudioSharedState {
      stopped: self.stopped.clone(),
      scope_collection: self.scope_collection.clone(),
      recording_writer: self.recording_writer.clone(),
      audio_budget_meter: self.audio_budget_meter.clone(),
      module_states: self.module_states.clone(),
      midi_manager: self.midi_manager.clone(),
    }
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

  pub fn get_audio_buffers(&self) -> Vec<(ScopeItem, Vec<Float32Array>, ScopeStats)> {
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
      .map(|(scope_item, scope_buffer)| {
        let channels = scope_buffer.get_channel_buffers();
        let stats = scope_buffer.compute_stats();
        (scope_item.clone(), channels, stats)
      })
      .collect()
  }

  pub fn get_module_states(&self) -> HashMap<String, serde_json::Value> {
    // Read module states from shared buffer (written by audio thread)
    match self.module_states.try_lock() {
      Some(states) => states.clone(),
      None => HashMap::new(), // Skip if locked by audio thread
    }
  }

  /// Build a PatchUpdate from desired graph and send to audio thread.
  /// This computes the diff using the shadow state and constructs new modules on the main thread.
  pub fn apply_patch(&self, desired_graph: PatchGraph, sample_rate: f32) -> Result<()> {
    let PatchGraph {
      modules,
      module_id_remaps,
      scopes,
      ..
    } = desired_graph;

    // Build PatchUpdate with all the info needed
    let mut update = PatchUpdate::new(sample_rate);

    // Add remaps
    update.remaps = module_id_remaps.unwrap_or_default();

    // Build maps for efficient lookup
    let desired_modules: HashMap<String, _> = modules.iter().map(|m| (m.id.clone(), m)).collect();

    // Compute scopes to add/remove/update
    {
      let scope_collection = self.scope_collection.lock();
      let current_scope_items: HashSet<ScopeItem> = scope_collection.keys().cloned().collect();
      let desired_scopes: HashMap<ScopeItem, Scope> =
        scopes.into_iter().map(|s| (s.item.clone(), s)).collect();
      let desired_scope_items: HashSet<ScopeItem> = desired_scopes.keys().cloned().collect();

      // Scopes to remove
      update.scope_removes = current_scope_items
        .difference(&desired_scope_items)
        .cloned()
        .collect();

      // Scopes to add
      update.scope_adds = desired_scope_items
        .difference(&current_scope_items)
        .filter_map(|item| desired_scopes.get(item))
        .cloned()
        .collect();

      // Scopes to update
      update.scope_updates = desired_scope_items
        .intersection(&current_scope_items)
        .filter_map(|item| desired_scopes.get(item))
        .cloned()
        .collect();
    }

    // For now, we send all modules as param_updates and inserts
    // The audio thread will figure out what actually needs to be created vs updated
    // This is a temporary simplification - a proper implementation would track shadow state

    // Construct all modules that appear in desired graph on main thread
    let constructors = get_constructors();
    for (id, module_state) in &desired_modules {
      // // Skip well-known modules
      // if id == WellKnownModule::RootOutput.id()
      //   || id == WellKnownModule::RootClock.id()
      //   || id == WellKnownModule::RootInput.id()
      //   || id == WellKnownModule::HiddenAudioIn.id()
      // {
      //   continue;
      // }

      if let Some(constructor) = constructors.get(&module_state.module_type) {
        match constructor(id, sample_rate) {
          Ok(module) => {
            // Extract the inner Box<dyn Sampleable> from the Arc
            // This is safe because we just created it and hold the only reference
            match Arc::try_unwrap(module) {
              Ok(boxed) => {
                update.inserts.push((id.clone(), boxed));
              }
              Err(_) => {
                return Err(napi::Error::from_reason(format!(
                  "Failed to unwrap Arc for module {}",
                  id
                )));
              }
            }
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
          module_state.module_type
        )));
      }

      // Also add param update with precomputed channel count
      let channel_count =
        crate::lookup_or_derive_channel_count(&module_state.module_type, &module_state.params)
          .unwrap_or(1);
      update
        .param_updates
        .push((id.clone(), module_state.params.clone(), channel_count));
    }

    // Send the update to audio thread
    self.send_command(GraphCommand::PatchUpdate(update))
  }

  pub fn handle_set_patch(
    &self,
    patch_graph: PatchGraph,
    sample_rate: f32,
  ) -> Vec<ApplyPatchError> {
    // Validate patch
    let schemas = schema();
    if let Err(errors) = validate_patch(&patch_graph, &schemas) {
      return vec![ApplyPatchError {
        message: "Validation failed".to_string(),
        errors: Some(errors),
      }];
    }

    // If stopped, send clear command first to reset state
    if self.is_stopped() {
      let _ = self.send_command(GraphCommand::ClearPatch);
      let mut scope_collection = self.scope_collection.lock();
      scope_collection.clear();
    }

    // Apply patch
    if let Err(e) = self.apply_patch(patch_graph, sample_rate) {
      return vec![ApplyPatchError {
        message: format!("Failed to apply patch: {}", e),
        errors: None,
      }];
    }

    let responses: Vec<ApplyPatchError> = vec![];

    // Auto-unmute on SetPatch to match prior imperative flow
    if self.is_stopped() {
      self.set_stopped(false);
    }

    responses
  }
}

/// Shared state that both AudioState (main thread) and AudioProcessor (audio thread) can access
pub struct AudioSharedState {
  pub stopped: Arc<AtomicBool>,
  pub scope_collection: Arc<Mutex<HashMap<ScopeItem, ScopeBuffer>>>,
  pub recording_writer: Arc<Mutex<Option<WavWriter<BufWriter<File>>>>>,
  pub audio_budget_meter: Arc<AudioBudgetMeter>,
  /// Module states (e.g., seq current step) - written by audio thread, read by main thread
  pub module_states: Arc<Mutex<HashMap<String, serde_json::Value>>>,
  /// MIDI input manager for polling MIDI messages
  pub midi_manager: Arc<MidiInputManager>,
}

fn chrono_simple_timestamp() -> String {
  use std::time::{SystemTime, UNIX_EPOCH};
  let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
  format!("{}", duration.as_secs())
}

// ============================================================================
// AudioProcessor - Audio thread side
// ============================================================================

/// Audio processor that runs on the audio thread.
/// Owns the Patch directly and processes commands from the main thread.
struct AudioProcessor {
  /// The DSP patch graph - owned directly, no mutex needed
  patch: Patch,
  /// Command queue consumer
  command_rx: CommandConsumer,
  /// Error queue producer
  error_tx: ErrorProducer,
  /// Shared stopped flag
  stopped: Arc<AtomicBool>,
  /// Shared scope collection
  scope_collection: Arc<Mutex<HashMap<ScopeItem, ScopeBuffer>>>,
  /// Shared module states (e.g., seq current step)
  module_states: Arc<Mutex<HashMap<String, serde_json::Value>>>,
  /// MIDI input manager for polling
  midi_manager: Arc<MidiInputManager>,
}

impl AudioProcessor {
  fn new(command_rx: CommandConsumer, error_tx: ErrorProducer, shared: AudioSharedState) -> Self {
    Self {
      patch: Patch::new(),
      command_rx,
      error_tx,
      stopped: shared.stopped,
      scope_collection: shared.scope_collection,
      module_states: shared.module_states,
      midi_manager: shared.midi_manager,
    }
  }

  /// Process all pending commands from the main thread and poll MIDI.
  /// Called at the start of each audio callback before processing frames.
  fn process_commands(&mut self) {
    // Poll MIDI messages and dispatch directly to the patch
    // This happens in the audio thread for low-latency MIDI response
    for msg in self.midi_manager.take_messages() {
      if let Err(e) = self.patch.dispatch_message(&msg) {
        let _ = self.error_tx.push(AudioError::MessageDispatchFailed {
          message: e.to_string(),
        });
      }
    }

    // Process commands from the main thread
    while let Ok(cmd) = self.command_rx.pop() {
      match cmd {
        GraphCommand::PatchUpdate(update) => {
          self.apply_patch_update(update);
        }
        GraphCommand::SingleParamUpdate {
          module_id,
          params,
          channel_count,
        } => {
          if let Some(module) = self.patch.sampleables.get(&module_id) {
            if let Err(e) = module.try_update_params(params, channel_count) {
              let _ = self.error_tx.push(AudioError::ParamUpdateFailed {
                module_id,
                message: e.to_string(),
              });
            }
          }
        }
        GraphCommand::DispatchMessage(msg) => {
          if let Err(e) = self.patch.dispatch_message(&msg) {
            let _ = self.error_tx.push(AudioError::MessageDispatchFailed {
              message: e.to_string(),
            });
          }
        }
        GraphCommand::Start => {
          let msg = Message::Clock(ClockMessages::Start);
          let _ = self.patch.dispatch_message(&msg);
        }
        GraphCommand::Stop => {
          // Stop is handled via the stopped flag
        }
        GraphCommand::ClearPatch => {
          self.patch.sampleables.clear();
          self.patch.insert_audio_in();
          self.patch.rebuild_message_listeners();
        }
      }
    }
  }

  /// Apply a patch update command
  fn apply_patch_update(&mut self, update: PatchUpdate) {
    // Apply remaps first
    for remap in &update.remaps {
      // Skip reserved module IDs
      if is_reserved_module_id(&remap.from) || is_reserved_module_id(&remap.to) {
        continue;
      }
      if remap.from == remap.to {
        continue;
      }
      if let Some(module) = self.patch.sampleables.remove(&remap.from) {
        // Remove existing target if present
        self.patch.sampleables.remove(&remap.to);
        self.patch.sampleables.insert(remap.to.clone(), module);
      }
    }

    // Collect the set of desired module IDs from inserts before consuming them.
    // apply_patch sends ALL desired modules as inserts, so any module not in this
    // set (and not reserved) is stale and should be removed.
    let desired_ids: std::collections::HashSet<String> =
      update.inserts.iter().map(|(id, _)| id.clone()).collect();

    // Insert new modules - wrap Box in Arc
    for (id, boxed_module) in update.inserts {
      // Check if module already exists - if so, skip (keep existing instance)
      self
        .patch
        .sampleables
        .entry(id)
        .or_insert_with(|| Arc::new(boxed_module));
    }

    // Remove modules that are no longer in the desired graph.
    // This handles the case where a module was removed from the patch — without this,
    // stale modules would keep running on the audio thread and reporting state.
    self
      .patch
      .sampleables
      .retain(|id, _| is_reserved_module_id(id) || desired_ids.contains(id));

    // Update params for all modules
    for (id, params, channel_count) in &update.param_updates {
      if let Some(module) = self.patch.sampleables.get(id)
        && let Err(e) = module.try_update_params(params.clone(), *channel_count)
      {
        let _ = self.error_tx.push(AudioError::ParamUpdateFailed {
          module_id: id.clone(),
          message: e.to_string(),
        });
      }
    }

    // Rebuild message listeners after structural changes
    self.patch.rebuild_message_listeners();

    // Connect all modules
    for module in self.patch.sampleables.values() {
      module.connect(&self.patch);
    }

    // Call on_patch_update for all modules
    for module in self.patch.sampleables.values() {
      module.on_patch_update();
    }

    // Update scopes
    {
      let mut scope_collection = self.scope_collection.lock();

      // Remove scopes
      for scope_item in &update.scope_removes {
        scope_collection.remove(scope_item);
      }

      // Add new scopes
      for scope in &update.scope_adds {
        scope_collection.insert(
          scope.item.clone(),
          ScopeBuffer::new(scope, update.sample_rate),
        );
      }

      // Update existing scopes
      for scope in &update.scope_updates {
        if let Some(buffer) = scope_collection.get_mut(&scope.item) {
          buffer.update(scope, update.sample_rate);
        }
      }
    }
  }

  /// Process a single frame, returning multi-channel output
  fn process_frame(&mut self, num_channels: usize) -> [f32; PORT_MAX_CHANNELS] {
    use modular_core::types::ROOT_ID;
    profiling::scope!("process_frame");

    let mut output = [0.0f32; PORT_MAX_CHANNELS];

    // Update all modules
    {
      profiling::scope!("update_modules");
      for module in self.patch.sampleables.values() {
        module.update();
      }
    }

    // Tick all modules
    {
      profiling::scope!("tick_modules");
      for module in self.patch.sampleables.values() {
        module.tick();
      }
    }

    // Capture audio for scopes
    {
      profiling::scope!("capture_scopes");
      let mut scope_lock = self.scope_collection.lock();
      for (scope, scope_buffer) in scope_lock.iter_mut() {
        match scope {
          ScopeItem::ModuleOutput {
            module_id,
            port_name,
          } => {
            if let Some(module) = self.patch.sampleables.get(module_id)
              && let Ok(poly) = module.get_poly_sample(port_name)
            {
              let num_channels = poly.channels();
              let values: Vec<f32> = (0..num_channels).map(|ch| poly.get(ch)).collect();
              scope_buffer.push_poly(&values, num_channels);
            }
          }
        }
      }
    }

    // Get output from root module
    if let Some(root) = self.patch.sampleables.get(&*ROOT_ID) {
      if let Ok(poly) = root.get_poly_sample(&ROOT_OUTPUT_PORT) {
        for ch in 0..num_channels.min(PORT_MAX_CHANNELS) {
          output[ch] = poly.get(ch) * AUDIO_OUTPUT_ATTENUATION;
        }
      }
    }

    output
  }

  /// Collect states from modules that implement StatefulModule (e.g., Seq).
  /// Uses try_lock to avoid blocking the audio thread if the main thread is reading.
  fn collect_module_states(&self) {
    // Use try_lock to avoid blocking audio if main thread is reading
    if let Some(mut states) = self.module_states.try_lock() {
      states.clear();
      for (id, module) in &self.patch.sampleables {
        if let Some(state) = module.get_state() {
          states.insert(id.clone(), state);
        }
      }
    }
  }

  fn is_stopped(&self) -> bool {
    self.stopped.load(Ordering::SeqCst)
  }
}

/// Check if a module ID is reserved (well-known system module)
fn is_reserved_module_id(id: &str) -> bool {
  id == WellKnownModule::RootOutput.id()
    || id == WellKnownModule::RootClock.id()
    || id == WellKnownModule::RootInput.id()
    || id == WellKnownModule::HiddenAudioIn.id()
}

pub fn make_stream<T>(
  device: &cpal::Device,
  config: &cpal::StreamConfig,
  command_rx: CommandConsumer,
  error_tx: ErrorProducer,
  shared: AudioSharedState,
  mut input_reader: InputBufferReader,
) -> Result<cpal::Stream>
where
  T: SizedSample + FromSample<f32> + hound::Sample,
{
  let num_channels = config.channels as usize;

  let err_fn = |err| eprintln!("Error building output sound stream: {err}");

  let time_at_start = std::time::Instant::now();
  println!("Time at start: {time_at_start:?}");

  // Clone shared state for the closure
  let recording_writer = shared.recording_writer.clone();
  let audio_budget_meter = shared.audio_budget_meter.clone();

  // Create the audio processor that owns the patch
  let mut audio_processor = AudioProcessor::new(command_rx, error_tx, shared);

  let mut final_state_processor = FinalStateProcessor::new(num_channels);

  let stream = device
    .build_output_stream(
      config,
      move |output: &mut [T], _info: &cpal::OutputCallbackInfo| {
        profiling::scope!("audio_callback");

        let callback_start = Instant::now();

        // Process any pending commands from the main thread
        {
          profiling::scope!("process_commands");
          audio_processor.process_commands();
        }

        {
          profiling::scope!("process_frames");
          for frame in output.chunks_mut(num_channels) {
            // Read from the input buffer and update audio_in
            {
              let mut audio_in = audio_processor.patch.audio_in.lock();
              let input_samples = input_reader.read_frame();

              // Set channel count so that get() returns values instead of 0.0
              audio_in.set_channels(PORT_MAX_CHANNELS);
              for i in 0..PORT_MAX_CHANNELS {
                // Apply gain to bring input from [-1, 1] to [-5, 5] volt range
                audio_in.set(i, input_samples[i] * AUDIO_INPUT_GAIN);
              }
            }

            // Process frame and get multi-channel output
            let samples = final_state_processor
              .process_frame_with_processor(&mut audio_processor, num_channels);

            for (ch, s) in frame.iter_mut().enumerate() {
              if ch < samples.len() {
                *s = T::from_sample(samples[ch]);
              } else {
                *s = T::from_sample(0.0);
              }
            }

            // Record if enabled (use try_lock to avoid blocking audio)
            // For multi-channel, record first channel (mono mix could be added later)
            if let Some(mut writer_guard) = recording_writer.try_lock()
              && let Some(ref mut writer) = *writer_guard
            {
              let _ = writer.write_sample(T::from_sample(samples[0]));
            }
          }
        }

        // Collect module states for UI (e.g., seq step highlighting)
        // Done once per buffer, not per frame, to minimize overhead
        {
          profiling::scope!("collect_module_states");
          audio_processor.collect_module_states();
        }

        let elapsed_ns = callback_start.elapsed().as_nanos() as u64;

        audio_budget_meter.record_chunk(output.len() as u64, elapsed_ns);
      },
      err_fn,
      None,
    )
    .map_err(|e| napi::Error::from_reason(format!("Failed to build output stream: {}", e)))?;

  Ok(stream)
}

/// Build an input stream that writes to the input buffer
pub fn make_input_stream<T>(
  device: &cpal::Device,
  config: &cpal::StreamConfig,
  mut input_writer: InputBufferWriter,
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
        input_writer.write(&f32_data);
      },
      err_fn,
      None,
    )
    .map_err(|e| napi::Error::from_reason(format!("Failed to build input stream: {}", e)))?;

  Ok(stream)
}

pub fn get_host_by_preference() -> Host {
  #[cfg(target_os = "windows")]
  {
    // if let Ok(asio_host) = cpal::host_from_id(HostId::Asio) {
    //   println!("Using ASIO");
    //   return asio_host;
    // }

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

  /// Process frame using AudioProcessor and return multi-channel output
  fn process_frame_with_processor(
    &mut self,
    processor: &mut AudioProcessor,
    num_channels: usize,
  ) -> [f32; PORT_MAX_CHANNELS] {
    let is_stopped = processor.is_stopped();
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

    let raw_output = processor.process_frame(num_channels);

    // Apply attenuation and soft clipping to all channels
    let mut any_audible = false;
    for ch in 0..num_channels.min(PORT_MAX_CHANNELS) {
      let sample = raw_output[ch] * self.attenuation_factor;
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

  // ============================================================================
  // Legacy tests - commented out after Phase 2 architecture change
  // ============================================================================
  // These tests used the old AudioState::new() and direct apply_patch() method
  // which have been replaced by the command queue architecture.
  //
  // TODO: Rewrite tests to use the new architecture:
  // - Create AudioProcessor directly for unit tests
  // - Or create integration tests that use the full command queue flow
  //
  // The functionality being tested (module ID remaps, stopped state, etc.)
  // is now handled in AudioProcessor::apply_patch_update() and the command
  // queue dispatch logic.
  // ============================================================================

  #[test]
  fn test_stopped_state_via_shared_state() {
    // Test the shared stopped atomic directly
    let stopped = Arc::new(AtomicBool::new(true));

    // Initially stopped
    assert!(stopped.load(Ordering::Acquire));
    stopped.store(false, Ordering::Release);
    assert!(!stopped.load(Ordering::Acquire));
    stopped.store(true, Ordering::Release);
    assert!(stopped.load(Ordering::Acquire));
  }

  #[test]
  fn test_audio_processor_owns_patch() {
    // Verify AudioProcessor can be created and owns patch exclusively
    let (cmd_producer, cmd_consumer, err_producer, _err_consumer) = create_audio_channels();

    // Drop the command producer since we won't use it in this test
    drop(cmd_producer);

    let shared = AudioSharedState {
      stopped: Arc::new(AtomicBool::new(true)),
      scope_collection: Arc::new(Mutex::new(HashMap::new())),
      recording_writer: Arc::new(Mutex::new(None)),
      audio_budget_meter: Arc::new(AudioBudgetMeter::new()),
      module_states: Arc::new(Mutex::new(HashMap::new())),
      midi_manager: Arc::new(MidiInputManager::new()),
    };

    let processor = AudioProcessor::new(cmd_consumer, err_producer, shared);

    // Processor starts with empty patch (may have hidden audio_in)
    assert!(processor.patch.sampleables.is_empty() || processor.patch.sampleables.len() == 1);
  }
}
