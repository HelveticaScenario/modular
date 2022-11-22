use atomic_float::AtomicF32;
use crossbeam_channel::{Receiver, Sender};
use parking_lot::{Mutex, RwLock};
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
use uuid::Uuid;
pub struct Patch {
    pub sampleables: SampleableMap,
    pub tracks: TrackMap,
}

pub const SAMPLE_RATE: AtomicF32 = AtomicF32::new(0.0);

impl Patch {
    pub fn new(sampleables: SampleableMap, tracks: TrackMap) -> Self {
        Patch {
            sampleables,
            tracks,
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
            Uuid::nil(),
            get_constructors().get(&"signal".to_owned()).unwrap()(&Uuid::nil(), sample_rate)
                .unwrap(),
        );
        let patch_clone = patch.clone();

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
                    write_data::<f32>(data, channels, &mut patch, &delta)
                },
                err_fn,
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
                    write_data::<i16>(data, channels, &mut patch, &delta)
                },
                err_fn,
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
                    write_data::<u16>(data, channels, &mut patch, &delta)
                },
                err_fn,
            )?,
        };

        stream.play()?;

        for message in receiver {
            handle_message(message, &patch, &sender, sample_rate)?;
        }
        Ok(())
    }
}

fn write_data<T>(output: &mut [T], channels: usize, patch: &mut Patch, delta: &Duration)
where
    T: cpal::Sample,
{
    for frame in output.chunks_mut(channels) {
        let value = cpal::Sample::from::<f32>(&process_frame(patch, delta));
        for sample in frame.iter_mut() {
            *sample = value;
        }
    }
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

fn process_frame(patch: &mut Patch, delta: &Duration) -> f32 {
    let Patch {
        ref mut sampleables,
        ref mut tracks,
    } = patch;
    update_tracks(tracks, delta);
    update_sampleables(sampleables);
    tick_sampleables(sampleables);
    get_patch_output(sampleables) / 5.0
}
