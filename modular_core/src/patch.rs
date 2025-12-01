use crossbeam_channel::{Receiver, Sender};
use parking_lot::Mutex;
use std::{collections::HashMap, sync::Arc, time::Duration};

use crate::{
    dsp::get_constructors,
    message::{handle_message, InputMessage, OutputMessage},
    types::{SampleableMap, TrackMap, ROOT_ID, ROOT_OUTPUT_PORT},
};
use cpal::{
    traits::{DeviceTrait, StreamTrait},
    StreamInstant,
};

#[derive(Clone)]
pub struct AudioSubscription {
    pub id: String,
    pub module_id: String,
    pub port: String,
    pub buffer_size: usize,
}

pub struct Patch {
    pub sampleables: SampleableMap,
    pub tracks: TrackMap,
    pub audio_subscriptions: HashMap<String, AudioSubscription>,
    pub audio_buffers: HashMap<String, Vec<f32>>,
}

impl Patch {
    pub fn new(sampleables: SampleableMap, tracks: TrackMap) -> Self {
        Patch {
            sampleables,
            tracks,
            audio_subscriptions: HashMap::new(),
            audio_buffers: HashMap::new(),
        }
    }

    pub fn run<T>(
        device: &cpal::Device,
        config: cpal::SupportedStreamConfig,
        receiver: Receiver<InputMessage>,
        sender: Sender<OutputMessage>,
    ) -> Result<(), anyhow::Error>
    where
        T: cpal::Sample,
    {
        let sample_rate = config.sample_rate().0 as f32;
        let patch = Arc::new(Mutex::new(Patch::new(HashMap::new(), HashMap::new())));
        let channels = config.channels() as usize;
        println!("{} {}", sample_rate, channels);

        let err_fn = |err| eprintln!("error: {}", err);
        patch.clone().lock().sampleables.insert(
            String::from("root"),
            get_constructors().get(&"signal".to_owned()).unwrap()(&String::from("root"), sample_rate)
                .unwrap(),
        );
        let patch_clone = patch.clone();
        let sender_clone = sender.clone();

        let mut last_instant: Option<StreamInstant> = None;
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_output_stream(
                &config.into(),
                move |data, info: &_| {
                    let new_instant = info.timestamp().callback;

                    let delta = match last_instant {
                        Some(last_instant) => new_instant.duration_since(&last_instant),
                        None => None,
                    }
                    .unwrap_or(Duration::from_nanos(0));
                    last_instant = Some(new_instant);
                    let mut patch = patch_clone.lock();
                    write_data::<f32>(data, channels, &mut patch, &delta, &sender_clone)
                },
                err_fn,
                None,
            )?,
            cpal::SampleFormat::I16 => device.build_output_stream(
                &config.into(),
                move |data, info: &_| {
                    let new_instant = info.timestamp().callback;

                    let delta = match last_instant {
                        Some(last_instant) => new_instant.duration_since(&last_instant),
                        None => None,
                    }
                    .unwrap_or(Duration::from_nanos(0));
                    last_instant = Some(new_instant);
                    let mut patch = patch_clone.lock();
                    write_data::<i16>(data, channels, &mut patch, &delta, &sender_clone)
                },
                err_fn,
                None,
            )?,
            cpal::SampleFormat::U16 => device.build_output_stream(
                &config.into(),
                move |data, info: &_| {
                    let new_instant = info.timestamp().callback;

                    let delta = match last_instant {
                        Some(last_instant) => new_instant.duration_since(&last_instant),
                        None => None,
                    }
                    .unwrap_or(Duration::from_nanos(0));
                    last_instant = Some(new_instant);
                    let mut patch = patch_clone.lock();
                    write_data::<u16>(data, channels, &mut patch, &delta, &sender_clone)
                },
                err_fn,
                None,
            )?,
            _ => panic!("Unsupported sample format"),
        };

        stream.play()?;

        for message in receiver {
            handle_message(message, &patch, &sender, sample_rate)?;
        }
        Ok(())
    }
}

fn write_data<T>(output: &mut [T], channels: usize, patch: &mut Patch, delta: &Duration, sender: &Sender<OutputMessage>)
where
    T: cpal::Sample + cpal::FromSample<f32>,
{
    for frame in output.chunks_mut(channels) {
        let value: T = cpal::Sample::from_sample(process_frame(patch, delta));
        for sample in frame.iter_mut() {
            *sample = value;
        }
    }
    
    // Check if any audio buffers are ready to send
    send_audio_buffers(patch, sender);
}

fn update_tracks(tracks: &mut TrackMap, delta: &Duration) {
    for (_, track) in tracks {
        track.tick(delta);
    }
}

fn update_sampleables(sampleables: &mut SampleableMap) {
    for (_, module) in sampleables {
        module.update();
    }
}

fn tick_sampleables(sampleables: &mut SampleableMap) {
    for (_, module) in sampleables {
        module.tick();
    }
}

fn get_patch_output(sampleables: &SampleableMap) -> f32 {
    if let Some(root) = sampleables.get(&*ROOT_ID) {
        return root.get_sample(&*ROOT_OUTPUT_PORT).unwrap_or_default();
    } else {
        return 0.0;
    }
}

fn send_audio_buffers(patch: &mut Patch, sender: &Sender<OutputMessage>) {
    // Send audio buffers when they reach the target size
    let subscriptions: Vec<(String, usize)> = patch
        .audio_subscriptions
        .iter()
        .map(|(id, sub)| (id.clone(), sub.buffer_size))
        .collect();
    
    for (sub_id, buffer_size) in subscriptions {
        if let Some(buffer) = patch.audio_buffers.get_mut(&sub_id) {
            if buffer.len() >= buffer_size {
                // Extract the buffer samples
                let samples: Vec<f32> = buffer.drain(0..buffer_size).collect();
                
                // Send via channel (ignore errors as websocket may not be connected)
                let _ = sender.try_send(OutputMessage::AudioBuffer {
                    subscription_id: sub_id,
                    samples,
                });
            }
        }
    }
}

fn process_frame(patch: &mut Patch, delta: &Duration) -> f32 {
    let Patch {
        sampleables,
        tracks,
        audio_subscriptions,
        audio_buffers,
    } = patch;
    update_tracks(tracks, delta);
    update_sampleables(sampleables);
    tick_sampleables(sampleables);
    
    // Capture audio for subscriptions
    for (sub_id, subscription) in audio_subscriptions.iter() {
        if let Some(module) = sampleables.get(&subscription.module_id) {
            if let Ok(sample) = module.get_sample(&subscription.port) {
                let buffer = audio_buffers.entry(sub_id.clone()).or_insert_with(Vec::new);
                buffer.push(sample);
                
                // Keep buffer from growing indefinitely (max 10 buffers worth)
                if buffer.len() > subscription.buffer_size * 10 {
                    buffer.drain(0..subscription.buffer_size);
                }
            }
        }
    }
    
    get_patch_output(sampleables) / 5.0
}
