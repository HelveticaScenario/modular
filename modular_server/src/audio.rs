use anyhow::Result;
use crossbeam_channel::Sender;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::StreamInstant;
use hound::{WavSpec, WavWriter};
use parking_lot::Mutex;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use modular_core::patch::Patch;
use crate::protocol::OutputMessage;

/// Audio subscription for streaming samples to clients
#[derive(Clone)]
pub struct AudioSubscription {
    pub id: String,
    pub module_id: String,
    pub port: String,
    pub buffer_size: usize,
}

/// Shared audio state between audio thread and server
pub struct AudioState {
    pub patch: Arc<Mutex<Patch>>,
    pub muted: Arc<AtomicBool>,
    pub subscriptions: Arc<Mutex<HashMap<String, AudioSubscription>>>,
    pub audio_buffers: Arc<Mutex<HashMap<String, Vec<f32>>>>,
    pub recording_writer: Arc<Mutex<Option<WavWriter<BufWriter<File>>>>>,
    pub recording_path: Arc<Mutex<Option<PathBuf>>>,
    pub sample_rate: f32,
}

impl AudioState {
    pub fn new(patch: Arc<Mutex<Patch>>, sample_rate: f32) -> Self {
        Self {
            patch,
            muted: Arc::new(AtomicBool::new(false)),
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
            audio_buffers: Arc::new(Mutex::new(HashMap::new())),
            recording_writer: Arc::new(Mutex::new(None)),
            recording_path: Arc::new(Mutex::new(None)),
            sample_rate,
        }
    }
    
    pub fn set_muted(&self, muted: bool) {
        self.muted.store(muted, Ordering::SeqCst);
    }
    
    pub fn is_muted(&self) -> bool {
        self.muted.load(Ordering::SeqCst)
    }
    
    pub fn add_subscription(&self, subscription: AudioSubscription) -> String {
        let id = subscription.id.clone();
        self.subscriptions.lock().insert(id.clone(), subscription);
        id
    }
    
    pub fn remove_subscription(&self, id: &str) {
        self.subscriptions.lock().remove(id);
        self.audio_buffers.lock().remove(id);
    }
    
    pub fn start_recording(&self, filename: Option<String>) -> Result<String> {
        let filename = filename.unwrap_or_else(|| {
            format!("recording_{}.wav", chrono_simple_timestamp())
        });
        let path = PathBuf::from(&filename);
        
        let spec = WavSpec {
            channels: 1,
            sample_rate: self.sample_rate as u32,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        
        let writer = WavWriter::create(&path, spec)?;
        *self.recording_writer.lock() = Some(writer);
        *self.recording_path.lock() = Some(path);
        
        Ok(filename)
    }
    
    pub fn stop_recording(&self) -> Result<Option<String>> {
        let writer = self.recording_writer.lock().take();
        let path = self.recording_path.lock().take();
        
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
pub fn run_audio_thread(
    audio_state: Arc<AudioState>,
    output_tx: Sender<OutputMessage>,
) -> Result<cpal::Stream> {
    let host = cpal::default_host();
    let device = host.default_output_device()
        .ok_or_else(|| anyhow::anyhow!("No audio output device found"))?;
    let config = device.default_output_config()?;
    let sample_rate = config.sample_rate().0 as f32;
    let channels = config.channels() as usize;
    
    tracing::info!("Audio: {} Hz, {} channels", sample_rate, channels);
    
    let mut last_instant: Option<StreamInstant> = None;
    
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => {
            device.build_output_stream(
                &config.into(),
                move |data: &mut [f32], info: &_| {
                    let new_instant = info.timestamp().callback;
                    let delta = last_instant
                        .and_then(|last| new_instant.duration_since(&last))
                        .unwrap_or(Duration::from_nanos(0));
                    last_instant = Some(new_instant);
                    
                    write_audio_data(data, channels, &audio_state, &delta, &output_tx);
                },
                |err| tracing::error!("Audio error: {}", err),
                None,
            )?
        }
        cpal::SampleFormat::I16 => {
            device.build_output_stream(
                &config.into(),
                move |data: &mut [i16], info: &_| {
                    let new_instant = info.timestamp().callback;
                    let delta = last_instant
                        .and_then(|last| new_instant.duration_since(&last))
                        .unwrap_or(Duration::from_nanos(0));
                    last_instant = Some(new_instant);
                    
                    write_audio_data_i16(data, channels, &audio_state, &delta, &output_tx);
                },
                |err| tracing::error!("Audio error: {}", err),
                None,
            )?
        }
        cpal::SampleFormat::U16 => {
            device.build_output_stream(
                &config.into(),
                move |data: &mut [u16], info: &_| {
                    let new_instant = info.timestamp().callback;
                    let delta = last_instant
                        .and_then(|last| new_instant.duration_since(&last))
                        .unwrap_or(Duration::from_nanos(0));
                    last_instant = Some(new_instant);
                    
                    write_audio_data_u16(data, channels, &audio_state, &delta, &output_tx);
                },
                |err| tracing::error!("Audio error: {}", err),
                None,
            )?
        }
        _ => return Err(anyhow::anyhow!("Unsupported audio sample format")),
    };
    
    stream.play()?;
    Ok(stream)
}

fn write_audio_data(
    output: &mut [f32],
    channels: usize,
    audio_state: &Arc<AudioState>,
    delta: &Duration,
    output_tx: &Sender<OutputMessage>,
) {
    for frame in output.chunks_mut(channels) {
        let sample = process_frame(audio_state, delta, output_tx);
        let muted = audio_state.is_muted();
        let output_sample = if muted { 0.0 } else { sample / 5.0 };
        
        for s in frame.iter_mut() {
            *s = output_sample;
        }
        
        // Record if enabled
        if let Some(ref mut writer) = *audio_state.recording_writer.lock() {
            let _ = writer.write_sample(output_sample);
        }
    }
}

fn write_audio_data_i16(
    output: &mut [i16],
    channels: usize,
    audio_state: &Arc<AudioState>,
    delta: &Duration,
    output_tx: &Sender<OutputMessage>,
) {
    for frame in output.chunks_mut(channels) {
        let sample = process_frame(audio_state, delta, output_tx);
        let muted = audio_state.is_muted();
        let output_sample = if muted { 0.0 } else { sample / 5.0 };
        let sample_i16 = (output_sample * i16::MAX as f32) as i16;
        
        for s in frame.iter_mut() {
            *s = sample_i16;
        }
        
        // Record if enabled
        if let Some(ref mut writer) = *audio_state.recording_writer.lock() {
            let _ = writer.write_sample(output_sample);
        }
    }
}

fn write_audio_data_u16(
    output: &mut [u16],
    channels: usize,
    audio_state: &Arc<AudioState>,
    delta: &Duration,
    output_tx: &Sender<OutputMessage>,
) {
    for frame in output.chunks_mut(channels) {
        let sample = process_frame(audio_state, delta, output_tx);
        let muted = audio_state.is_muted();
        let output_sample = if muted { 0.0 } else { sample / 5.0 };
        let sample_u16 = ((output_sample + 1.0) * 0.5 * u16::MAX as f32) as u16;
        
        for s in frame.iter_mut() {
            *s = sample_u16;
        }
        
        // Record if enabled
        if let Some(ref mut writer) = *audio_state.recording_writer.lock() {
            let _ = writer.write_sample(output_sample);
        }
    }
}

fn process_frame(
    audio_state: &Arc<AudioState>,
    delta: &Duration,
    output_tx: &Sender<OutputMessage>,
) -> f32 {
    use modular_core::types::ROOT_ID;
    
    let patch = audio_state.patch.lock();
    
    // Update tracks
    for (_, track) in patch.tracks.iter() {
        track.tick(delta);
    }
    
    // Update sampleables
    for (_, module) in patch.sampleables.iter() {
        module.update();
    }
    
    // Tick sampleables
    for (_, module) in patch.sampleables.iter() {
        module.tick();
    }
    
    // Capture audio for subscriptions
    let subscriptions = audio_state.subscriptions.lock();
    for (sub_id, subscription) in subscriptions.iter() {
        if let Some(module) = patch.sampleables.get(&subscription.module_id) {
            if let Ok(sample) = module.get_sample(&subscription.port) {
                let mut buffers = audio_state.audio_buffers.lock();
                let buffer = buffers.entry(sub_id.clone()).or_insert_with(Vec::new);
                buffer.push(sample);
                
                // Keep buffer from growing indefinitely
                if buffer.len() > subscription.buffer_size * 10 {
                    buffer.drain(0..subscription.buffer_size);
                }
            }
        }
    }
    drop(subscriptions);
    
    // Send audio buffers when ready
    send_audio_buffers(audio_state, output_tx);
    
    // Get output sample
    if let Some(root) = patch.sampleables.get(&*ROOT_ID) {
        root.get_sample(&"output".to_string()).unwrap_or(0.0)
    } else {
        0.0
    }
}

fn send_audio_buffers(audio_state: &Arc<AudioState>, output_tx: &Sender<OutputMessage>) {
    let subscriptions: Vec<(String, usize)> = audio_state
        .subscriptions
        .lock()
        .iter()
        .map(|(id, sub)| (id.clone(), sub.buffer_size))
        .collect();
    
    let mut buffers = audio_state.audio_buffers.lock();
    for (sub_id, buffer_size) in subscriptions {
        if let Some(buffer) = buffers.get_mut(&sub_id) {
            if buffer.len() >= buffer_size {
                let samples: Vec<f32> = buffer.drain(0..buffer_size).collect();
                let _ = output_tx.try_send(OutputMessage::AudioBuffer {
                    subscription_id: sub_id,
                    samples,
                });
            }
        }
    }
}

/// Get the sample rate from the default audio device
pub fn get_sample_rate() -> Result<f32> {
    let host = cpal::default_host();
    let device = host.default_output_device()
        .ok_or_else(|| anyhow::anyhow!("No audio output device found"))?;
    let config = device.default_output_config()?;
    Ok(config.sample_rate().0 as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_audio_subscription() {
        let patch = Arc::new(Mutex::new(Patch::new(HashMap::new(), HashMap::new())));
        let state = AudioState::new(patch, 48000.0);
        
        let sub = AudioSubscription {
            id: "test-sub".to_string(),
            module_id: "sine-1".to_string(),
            port: "output".to_string(),
            buffer_size: 512,
        };
        
        let id = state.add_subscription(sub);
        assert_eq!(id, "test-sub");
        
        assert!(state.subscriptions.lock().contains_key("test-sub"));
        
        state.remove_subscription("test-sub");
        assert!(!state.subscriptions.lock().contains_key("test-sub"));
    }
    
    #[test]
    fn test_mute_state() {
        let patch = Arc::new(Mutex::new(Patch::new(HashMap::new(), HashMap::new())));
        let state = AudioState::new(patch, 48000.0);
        
        assert!(!state.is_muted());
        state.set_muted(true);
        assert!(state.is_muted());
        state.set_muted(false);
        assert!(!state.is_muted());
    }
}
