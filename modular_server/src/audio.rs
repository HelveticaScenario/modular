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
use std::sync::atomic::{AtomicBool, Ordering};

use crate::protocol::OutputMessage;
use modular_core::patch::Patch;
use modular_core::types::{ROOT_OUTPUT_PORT, ScopeItem};

/// Attenuation factor applied to audio output to prevent clipping.
/// DSP modules output signals in the range [-5, 5] volts (modular synth convention).
/// This factor brings the output into a reasonable range for audio output.
const AUDIO_OUTPUT_ATTENUATION: f32 = 5.0;

/// Audio subscription for streaming samples to clients
#[derive(Clone)]
pub struct CyclicBuffer<const N: usize> {
    buf: [f32; N],
    idx: usize, // next write position (0..N-1)
    filled: usize, // number of new samples written since last drain (0..=N)
}

impl<const N: usize> CyclicBuffer<N> {
    pub fn new() -> Self {
        Self {
            buf: [0.0; N],
            idx: 0,
            filled: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }
    pub fn index(&self) -> usize {
        self.idx
    }
    pub fn as_slice(&self) -> &[f32; N] {
        &self.buf
    }

    pub fn push(&mut self, value: f32) {
        self.push_vec(&[value]);
    }

    pub fn to_vec_ordered(&self) -> Vec<f32> {
        let n = self.buf.len();
        if n == 0 {
            return Vec::new();
        }
        let mut out = Vec::with_capacity(n);
        out.extend_from_slice(&self.buf[self.idx..]);
        out.extend_from_slice(&self.buf[..self.idx]);
        out
    }

    pub fn take_vec_if_full(&mut self) -> Option<Vec<f32>> {
        let n = self.buf.len();
        if n == 0 || self.filled < n {
            return None;
        }
        self.filled = 0;
        Some(self.to_vec_ordered())
    }

    pub fn push_vec(&mut self, data: &[f32]) {
        let n = self.buf.len();
        if n == 0 || data.is_empty() {
            return;
        }

        let orig_len = data.len();

        // If data is bigger than N, only the last N values affect final buffer contents,
        // but the *start position* must account for all the skipped writes.
        let (to_write, start) = if orig_len >= n {
            let skipped = orig_len - n;
            (&data[skipped..], (self.idx + skipped) % n)
        } else {
            (data, self.idx)
        };

        // Copy in at most two chunks (end then wrap to start).
        let first = (n - start).min(to_write.len());
        self.buf[start..start + first].copy_from_slice(&to_write[..first]);

        let remaining = to_write.len() - first;
        if remaining > 0 {
            self.buf[..remaining].copy_from_slice(&to_write[first..]);
        }

        // Advance index as-if we wrote the entire input.
        self.idx = (self.idx + orig_len) % n;

        // Track how much fresh data is available for draining.
        self.filled = (self.filled + orig_len).min(n);
    }
}

/// Shared audio state between audio thread and server
pub struct AudioState {
    pub patch: Arc<tokio::sync::Mutex<Patch>>,
    pub patch_code: String,
    pub muted: Arc<AtomicBool>,
    pub subscription_collection: Arc<tokio::sync::Mutex<AudioSubscriptionCollection>>,
    pub recording_writer: Arc<tokio::sync::Mutex<Option<WavWriter<BufWriter<File>>>>>,
    pub recording_path: Arc<tokio::sync::Mutex<Option<PathBuf>>>,
    pub sample_rate: f32,
}

pub struct AudioSubscriptionCollection {
    pub subscriptions:
        HashMap<ScopeItem, AudioSubscriptionBuffer<{ 512 * modular_core::types::NUM_CHANNELS }>>,
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

pub struct AudioSubscriptionBuffer<const N: usize> {
    pub buffer: CyclicBuffer<N>,
    pub txs: Vec<tokio::sync::mpsc::Sender<OutputMessage>>,
}

impl<const N: usize> AudioSubscriptionBuffer<N> {
    pub fn new() -> Self {
        Self {
            buffer: CyclicBuffer::new(),
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
            muted: Arc::new(AtomicBool::new(false)),
            subscription_collection: Arc::new(tokio::sync::Mutex::new(
                AudioSubscriptionCollection::new(),
            )),
            recording_writer: Arc::new(tokio::sync::Mutex::new(None)),
            recording_path: Arc::new(tokio::sync::Mutex::new(None)),
            sample_rate,
        }
    }

    pub fn set_muted(&self, muted: bool) {
        self.muted.store(muted, Ordering::SeqCst);
    }

    pub fn is_muted(&self) -> bool {
        self.muted.load(Ordering::SeqCst)
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
            .or_insert(AudioSubscriptionBuffer::new());

        subscription_buffer.txs.push(tx);
    }

    pub async fn start_recording(&self, filename: Option<String>) -> Result<String> {
        let filename =
            filename.unwrap_or_else(|| format!("recording_{}.wav", chrono_simple_timestamp()));
        let path = PathBuf::from(&filename);

        let spec = WavSpec {
            channels: 2,
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

    let err_fn = |err| eprintln!("Error building output sound stream: {err}");

    let time_at_start = std::time::Instant::now();
    println!("Time at start: {time_at_start:?}");
    let audio_state = audio_state.clone();

    let mut buffer = [0.0f32; modular_core::types::NUM_CHANNELS];

    let stream = device.build_output_stream(
        config,
        move |output: &mut [T], _: &cpal::OutputCallbackInfo| {
            for frame in output.chunks_mut(num_channels) {
                let muted = audio_state.is_muted();

                process_frame(&audio_state, &mut buffer);
                let chan_count = num_channels.min(modular_core::types::NUM_CHANNELS);
                if muted {
                    frame.fill(T::from_sample(0.0));
                } else {
                    for frame_idx in 0..chan_count {
                        frame[frame_idx] =
                            T::from_sample(buffer[frame_idx] / AUDIO_OUTPUT_ATTENUATION);
                    }
                }

                // Record if enabled (use try_lock to avoid blocking audio)
                if let Ok(mut writer_guard) = audio_state.recording_writer.try_lock() {
                    if let Some(ref mut writer) = *writer_guard {
                        let _ = writer.write_sample(buffer[0] / AUDIO_OUTPUT_ATTENUATION);
                    }
                }
            }
        },
        err_fn,
        None,
    )?;

    Ok(stream)
}

fn process_frame(
    audio_state: &Arc<AudioState>,
    sample_buffer: &mut [f32; modular_core::types::NUM_CHANNELS],
) {
    use modular_core::types::ROOT_ID;

    // Try to acquire patch lock - if we can't, skip this frame to avoid blocking audio
    let patch_guard = match audio_state.patch.try_lock() {
        Ok(guard) => guard,
        Err(_) => return, // Skip frame if patch is locked by another thread
    };

    // Tick tracks first (advance playheads)
    for (_, track) in patch_guard.tracks.iter() {
        track.tick();
    }

    // Prepare sampleables for this frame
    for (_, module) in patch_guard.sampleables.iter() {
        module.tick();
    }

    // Capture audio for subscriptions (modules will update lazily in get_sample)
    if let Ok(mut sub_collection) = audio_state.subscription_collection.try_lock() {
        for (subscription, subscription_buffer) in sub_collection.subscriptions.iter_mut() {
            match subscription {
                ScopeItem::ModuleOutput {
                    module_id,
                    port_name,
                } => {
                    if let Some(module) = patch_guard.sampleables.get(module_id) {
                        let mut buf = [0.0; modular_core::types::NUM_CHANNELS];
                        if let Ok(()) = module.get_sample(port_name, &mut buf) {
                            subscription_buffer.buffer.push_vec(&buf);
                        }
                    }
                }
                ScopeItem::Track { track_id } => {
                    if let Some(track) = patch_guard.tracks.get(track_id) {
                        let mut buf = [0.0; modular_core::types::NUM_CHANNELS];
                        if track.get_value_optional(&mut buf).is_some() {
                            subscription_buffer.buffer.push_vec(&buf);
                        }
                    }
                }
            }
        }
    }

    // Get output sample before dropping lock
    if let Some(root) = patch_guard.sampleables.get(&*ROOT_ID) {
        let _ = root.get_sample(&ROOT_OUTPUT_PORT, sample_buffer);
    }
}

pub fn send_audio_buffers(audio_state: &Arc<AudioState>) {
    // Skip emitting audio buffers entirely when muted
    if audio_state.is_muted() {
        return;
    }

    let mut subscription_collection = match audio_state.subscription_collection.try_lock() {
        Ok(subscription_collection) => subscription_collection,
        Err(_) => return, // Skip if locked
    };
    subscription_collection.clean();
    for (sub, subscription_buffer) in subscription_collection.subscriptions.iter_mut() {
        if let Some(samples) = subscription_buffer.buffer.take_vec_if_full() {
            for tx in subscription_buffer.txs.iter() {
                if let Err(e) = tx.try_send(OutputMessage::AudioBuffer {
                    subscription: sub.clone(),
                    samples: samples.clone(),
                }) {
                    eprintln!("Failed to send audio buffer: {}", e);
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
    fn test_mute_state() {
        let patch = Arc::new(tokio::sync::Mutex::new(Patch::new(
            HashMap::new(),
            HashMap::new(),
        )));
        let state = AudioState::new(patch, "".into(), 48000.0);

        assert!(!state.is_muted());
        state.set_muted(true);
        assert!(state.is_muted());
        state.set_muted(false);
        assert!(!state.is_muted());
    }

    #[tokio::test]
    async fn test_send_audio_buffers_respects_mute() {
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
            let capacity = buffer.buffer.len();
            for i in 0..capacity {
                buffer.buffer.push(i as f32);
            }
        }

        send_audio_buffers(&state);
        assert!(rx.try_recv().is_ok());

        state.set_muted(true);
        send_audio_buffers(&state);
        assert!(matches!(rx.try_recv(), Err(TryRecvError::Empty)));
    }
}
