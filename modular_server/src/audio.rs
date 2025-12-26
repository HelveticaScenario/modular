use anyhow::Result;
use cpal::FromSample;
use cpal::SizedSample;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use hound::{WavSpec, WavWriter};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::protocol::OutputMessage;
use modular_core::patch::Patch;
use modular_core::types::{ROOT_OUTPUT_PORT, ScopeItem};
use std::time::Instant;

/// Attenuation factor applied to audio output to prevent clipping.
/// DSP modules output signals in the range [-5, 5] volts (modular synth convention).
/// This factor brings the output into a reasonable range for audio output.
const AUDIO_OUTPUT_ATTENUATION: f32 = 0.2;

/// Audio subscription for streaming samples to clients
#[derive(Clone)]
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

/// Shared audio state between audio thread and server
pub struct AudioState {
    pub patch: Arc<tokio::sync::Mutex<Patch>>,
    pub patch_code: String,
    pub stopped: Arc<AtomicBool>,
    pub subscription_collection: Arc<tokio::sync::Mutex<AudioSubscriptionCollection>>,
    pub recording_writer: Arc<tokio::sync::Mutex<Option<WavWriter<BufWriter<File>>>>>,
    pub recording_path: Arc<tokio::sync::Mutex<Option<PathBuf>>>,
    pub sample_rate: f32,
    audio_thread_health: AudioThreadHealth,
}

#[derive(Default)]
struct AudioThreadHealth {
    /// Number of audio frames skipped because the real-time callback could not acquire
    /// the patch lock via `try_lock()`.
    patch_lock_misses: AtomicU64,

    /// Number of output callbacks whose execution time exceeded the duration of the
    /// buffer they were asked to fill (a strong signal of underrun risk).
    output_callback_overruns: AtomicU64,
    /// Max observed overrun (elapsed - expected) in nanoseconds.
    output_callback_overrun_max_ns: AtomicU64,
    /// Max observed total callback execution time in nanoseconds.
    output_callback_duration_max_ns: AtomicU64,
}

#[derive(Debug, Clone, Copy)]
pub struct AudioThreadHealthSnapshot {
    pub patch_lock_misses: u64,
    pub output_callback_overruns: u64,
    pub output_callback_overrun_max_ns: u64,
    pub output_callback_duration_max_ns: u64,
}

pub struct AudioSubscriptionCollection {
    pub subscriptions: HashMap<ScopeItem, AudioSubscriptionBuffer>,
}

impl AudioSubscriptionCollection {
    pub fn new() -> Self {
        Self {
            subscriptions: HashMap::new(),
        }
    }

    pub fn clean(&mut self) {
        self.subscriptions.retain(|_, buf| {
            buf.clean_txs();
            !buf.txs.is_empty()
        });
    }
}

pub struct AudioSubscriptionBuffer {
    pub buffer: RingBuffer,
    pub txs: Vec<tokio::sync::mpsc::Sender<OutputMessage>>,
}

impl AudioSubscriptionBuffer {
    pub fn new(buffer_size: usize) -> Self {
        Self {
            buffer: RingBuffer::new(buffer_size),
            txs: Vec::new(),
        }
    }

    pub fn clean_txs(&mut self) {
        self.txs.retain(|tx| !tx.is_closed());
    }
}

impl AudioState {
    pub fn new(
        patch: Arc<tokio::sync::Mutex<Patch>>,
        patch_code: String,
        sample_rate: f32,
    ) -> Self {
        Self {
            patch,
            patch_code,
            stopped: Arc::new(AtomicBool::new(true)),
            subscription_collection: Arc::new(tokio::sync::Mutex::new(
                AudioSubscriptionCollection::new(),
            )),
            recording_writer: Arc::new(tokio::sync::Mutex::new(None)),
            recording_path: Arc::new(tokio::sync::Mutex::new(None)),
            sample_rate,
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

    pub async fn add_subscription(
        &self,
        subscription: ScopeItem,
        tx: tokio::sync::mpsc::Sender<OutputMessage>,
    ) {
        let mut subscription_collection = self.subscription_collection.lock().await;
        let subscription_buffer = subscription_collection
            .subscriptions
            .entry(subscription.clone())
            .or_insert(AudioSubscriptionBuffer::new(512));

        subscription_buffer.txs.push(tx);
    }

    pub async fn start_recording(&self, filename: Option<String>) -> Result<String> {
        let filename =
            filename.unwrap_or_else(|| format!("recording_{}.wav", chrono_simple_timestamp()));
        let path = PathBuf::from(&filename);

        let spec = WavSpec {
            channels: 1,
            sample_rate: self.sample_rate as u32,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        let writer = WavWriter::create(&path, spec)?;
        *self.recording_writer.lock().await = Some(writer);
        *self.recording_path.lock().await = Some(path);

        Ok(filename)
    }

    pub async fn stop_recording(&self) -> Result<Option<String>> {
        let writer = self.recording_writer.lock().await.take();
        let path = self.recording_path.lock().await.take();

        if let Some(w) = writer {
            w.finalize()?;
        }

        Ok(path.map(|p| p.to_string_lossy().to_string()))
    }
}

fn chrono_simple_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    format!("{}", duration.as_secs())
}

/// Run the audio thread with cpal
pub fn run_audio_thread(audio_state: Arc<AudioState>) -> Result<cpal::Stream> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| anyhow::anyhow!("No audio output device found"))?;
    let config = device.default_output_config()?;
    let sample_rate = config.sample_rate().0 as f32;
    let channels = config.channels() as usize;

    tracing::info!("Audio: {} Hz, {} channels", sample_rate, channels);

    let stream = match config.sample_format() {
        cpal::SampleFormat::I8 => make_stream::<i8>(&device, &config.into(), &audio_state)?,
        cpal::SampleFormat::I16 => make_stream::<i16>(&device, &config.into(), &audio_state)?,
        cpal::SampleFormat::I32 => make_stream::<i32>(&device, &config.into(), &audio_state)?,
        cpal::SampleFormat::F32 => make_stream::<f32>(&device, &config.into(), &audio_state)?,
        _ => Err(anyhow::anyhow!(
            "Unsupported sample format: {:?}",
            config.sample_format()
        ))?,
    };

    stream.play()?;
    Ok(stream)
}

pub fn make_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    audio_state: &Arc<AudioState>,
) -> Result<cpal::Stream, anyhow::Error>
where
    T: SizedSample + FromSample<f32> + hound::Sample,
{
    let num_channels = config.channels as usize;
    let sample_rate_hz = config.sample_rate.0 as f64;

    let err_fn = |err| eprintln!("Error building output sound stream: {err}");

    let time_at_start = std::time::Instant::now();
    println!("Time at start: {time_at_start:?}");
    let audio_state = audio_state.clone();

    let mut final_state_processor = FinalStateProcessor::new();

    let stream = device.build_output_stream(
        config,
        move |output: &mut [T], _info: &cpal::OutputCallbackInfo| {
            let callback_start = Instant::now();

            for frame in output.chunks_mut(num_channels) {
                let output_sample =
                    T::from_sample(final_state_processor.process_frame(&audio_state));

                for s in frame.iter_mut() {
                    *s = output_sample;
                }

                // Record if enabled (use try_lock to avoid blocking audio)
                if let Ok(mut writer_guard) = audio_state.recording_writer.try_lock() {
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
                .fetch_max(elapsed_ns, Ordering::Relaxed);

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
                    .fetch_max(overrun_ns, Ordering::Relaxed);
            }
        },
        err_fn,
        None,
    )?;

    Ok(stream)
}

fn process_frame(audio_state: &Arc<AudioState>) -> f32 {
    use modular_core::types::ROOT_ID;

    // Try to acquire patch lock - if we can't, skip this frame to avoid blocking audio
    let patch_guard = match audio_state.patch.try_lock() {
        Ok(guard) => guard,
        Err(_) => {
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

    // Capture audio for subscriptions
    if let Ok(mut sub_collection) = audio_state.subscription_collection.try_lock() {
        for (subscription, subscription_buffer) in sub_collection.subscriptions.iter_mut() {
            match subscription {
                ScopeItem::ModuleOutput {
                    module_id,
                    port_name,
                } => {
                    if let Some(module) = patch_guard.sampleables.get(module_id) {
                        if let Ok(sample) = module.get_sample(port_name) {
                            subscription_buffer.buffer.push(sample);
                        }
                    }
                }
                ScopeItem::Track { track_id } => {
                    if let Some(track) = patch_guard.tracks.get(track_id) {
                        if let Some(sample) = track.get_value_optional() {
                            subscription_buffer.buffer.push(sample);
                        }
                    }
                }
            }
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

pub fn send_audio_buffers(audio_state: &Arc<AudioState>) {
    // Skip emitting audio buffers entirely when stopped
    if audio_state.is_stopped() {
        return;
    }

    let mut subscription_collection = match audio_state.subscription_collection.try_lock() {
        Ok(subscription_collection) => subscription_collection,
        Err(_) => return, // Skip if locked
    };
    subscription_collection.clean();
    for (sub, AudioSubscriptionBuffer { buffer, txs }) in
        subscription_collection.subscriptions.iter()
    {
        if buffer.buffer.len() >= buffer.capacity {
            let samples: Vec<f32> = buffer.to_vec();
            for tx in txs.iter() {
                match tx.try_send(OutputMessage::AudioBuffer {
                    subscription: sub.clone(),
                    samples: samples.clone(),
                }) {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Failed to send audio buffer: {}", e);
                    }
                }
            }
        }
    }
}

/// Get the sample rate from the default audio device
pub fn get_sample_rate() -> Result<f32> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or_else(|| anyhow::anyhow!("No audio output device found"))?;
    let config = device.default_output_config()?;
    Ok(config.sample_rate().0 as f32)
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
                (process_frame(audio_state) * AUDIO_OUTPUT_ATTENUATION * self.attenuation_factor)
                    .tanh();

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
    use tokio::sync::mpsc::error::TryRecvError;

    #[tokio::test]
    async fn test_audio_subscription() {
        let patch = Arc::new(tokio::sync::Mutex::new(Patch::new(
            HashMap::new(),
            HashMap::new(),
        )));
        let state = AudioState::new(patch, "".into(), 48000.0);
        let sub = ScopeItem::ModuleOutput {
            module_id: "sine-1".to_string(),
            port_name: "output".to_string(),
        };

        let (tx, rx) = tokio::sync::mpsc::channel(10);
        state.add_subscription(sub.clone(), tx).await;

        assert!(
            state
                .subscription_collection
                .try_lock()
                .unwrap()
                .subscriptions
                .contains_key(&sub)
        );
        drop(rx); // Close the receiver to simulate client disconnect
        state.subscription_collection.try_lock().unwrap().clean();
        assert!(
            !state
                .subscription_collection
                .try_lock()
                .unwrap()
                .subscriptions
                .contains_key(&sub)
        );
    }

    #[test]
    fn test_stopped_state() {
        let patch = Arc::new(tokio::sync::Mutex::new(Patch::new(
            HashMap::new(),
            HashMap::new(),
        )));
        let state = AudioState::new(patch, "".into(), 48000.0);

        assert!(!state.is_stopped());
        state.set_stopped(true);
        assert!(state.is_stopped());
        state.set_stopped(false);
        assert!(!state.is_stopped());
    }

    #[tokio::test]
    async fn test_send_audio_buffers_respects_stopped() {
        let patch = Arc::new(tokio::sync::Mutex::new(Patch::new(
            HashMap::new(),
            HashMap::new(),
        )));
        let state = Arc::new(AudioState::new(patch, "".into(), 48_000.0));

        let sub = ScopeItem::ModuleOutput {
            module_id: "sine-1".to_string(),
            port_name: "output".to_string(),
        };

        let (tx, mut rx) = tokio::sync::mpsc::channel(10);
        state.add_subscription(sub.clone(), tx).await;

        {
            let mut guard = state.subscription_collection.try_lock().unwrap();
            let buffer = guard.subscriptions.get_mut(&sub).unwrap();
            let capacity = buffer.buffer.capacity;
            for i in 0..capacity {
                buffer.buffer.push(i as f32);
            }
        }

        send_audio_buffers(&state);
        assert!(rx.try_recv().is_ok());

        state.set_stopped(true);
        send_audio_buffers(&state);
        assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
    }
}
