use std::{
    collections::HashMap,
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
    time::Duration,
};

use crate::{
    message::{handle_message, InputMessage, OutputMessage},
    types::{SampleableMap, TrackMap, ROOT_ID, ROOT_OUTPUT_PORT},
};
use cpal::{
    traits::{DeviceTrait, StreamTrait}, StreamInstant,
};
pub struct Patch {
    pub sampleables: Arc<Mutex<SampleableMap>>,
    pub tracks: Arc<Mutex<TrackMap>>,
}

impl Patch {
    pub fn new(sampleables: SampleableMap, tracks: TrackMap) -> Self {
        Patch {
            sampleables: Arc::new(Mutex::new(sampleables)),
            tracks: Arc::new(Mutex::new(tracks)),
        }
    }

    pub fn run<T>(
        device: &cpal::Device,
        config: cpal::SupportedStreamConfig,
        reciever: Receiver<InputMessage>,
        sender: Sender<OutputMessage>,
    ) -> Result<(), anyhow::Error>
    where
        T: cpal::Sample,
    {
        let mut patch = Patch::new(HashMap::new(), HashMap::new());
        let sample_rate = config.sample_rate().0 as f32;
        let channels = config.channels() as usize;
        println!("{} {}", sample_rate, channels);

        let err_fn = |err| eprintln!("error: {}", err);
        let sampleables = patch.sampleables.clone();
        let tracks = patch.tracks.clone();
        let mut last_instant: Option<StreamInstant> = None;
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_output_stream(
                &config.into(),
                move |data, info: &_| {
                    if let (Ok(ref sampleables), Ok(ref tracks)) =
                        (sampleables.lock(), tracks.lock())
                    {
                        let new_instant = info.timestamp().callback;

                        let delta = match last_instant {
                            Some(last_instant) => new_instant.duration_since(&last_instant),
                            None => None,
                        }
                        .unwrap_or(Duration::from_nanos(0));
                        last_instant = Some(new_instant);
                        write_data::<f32>(
                            data,
                            channels,
                            &sampleables,
                            &tracks,
                            sample_rate,
                            &delta,
                        )
                    }
                },
                err_fn,
            )?,
            cpal::SampleFormat::I16 => device.build_output_stream(
                &config.into(),
                move |data, info: &_| {
                    if let (Ok(ref sampleables), Ok(ref tracks)) =
                        (sampleables.lock(), tracks.lock())
                    {
                        let new_instant = info.timestamp().callback;

                        let delta = match last_instant {
                            Some(last_instant) => new_instant.duration_since(&last_instant),
                            None => None,
                        }
                        .unwrap_or(Duration::from_nanos(0));
                        last_instant = Some(new_instant);
                        write_data::<i16>(
                            data,
                            channels,
                            &sampleables,
                            &tracks,
                            sample_rate,
                            &delta,
                        )
                    }
                },
                err_fn,
            )?,
            cpal::SampleFormat::U16 => device.build_output_stream(
                &config.into(),
                move |data, info: &_| {
                    if let (Ok(ref sampleables), Ok(ref tracks)) =
                        (sampleables.lock(), tracks.lock())
                    {
                        let new_instant = info.timestamp().callback;

                        let delta = match last_instant {
                            Some(last_instant) => new_instant.duration_since(&last_instant),
                            None => None,
                        }
                        .unwrap_or(Duration::from_nanos(0));
                        last_instant = Some(new_instant);
                        write_data::<u16>(
                            data,
                            channels,
                            &sampleables,
                            &tracks,
                            sample_rate,
                            &delta,
                        )
                    }
                },
                err_fn,
            )?,
        };

        stream.play()?;

        for message in reciever {
            handle_message(message, &mut patch, &sender)?;
        }

        Ok(())
    }
}

fn write_data<T>(
    output: &mut [T],
    channels: usize,
    sampleables: &SampleableMap,
    tracks: &TrackMap,
    sample_rate: f32,
    delta: &Duration,
) where
    T: cpal::Sample,
{
    for frame in output.chunks_mut(channels) {
        let value =
            cpal::Sample::from::<f32>(&process_frame(sampleables, tracks, sample_rate, delta));
        for sample in frame.iter_mut() {
            *sample = value;
        }
    }
}

fn update_tracks(tracks: &TrackMap, delta: &Duration) {
    for (_, track) in tracks {
        track.tick(delta);
    }
}

fn update_sampleables(sampleables: &SampleableMap, sample_rate: f32) {
    for (_, module) in sampleables {
        module.update(sample_rate);
    }
}

fn tick_sampleables(sampleables: &SampleableMap) {
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

fn process_frame(
    sampleables: &SampleableMap,
    tracks: &TrackMap,
    sample_rate: f32,
    delta: &Duration,
) -> f32 {
    update_tracks(tracks, delta);
    update_sampleables(sampleables, sample_rate);
    tick_sampleables(sampleables);
    get_patch_output(sampleables) / 5.0
}
