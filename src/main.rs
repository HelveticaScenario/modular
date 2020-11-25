#[macro_use]
extern crate lazy_static;

extern crate anyhow;
extern crate cpal;
extern crate hound;
extern crate serde;
extern crate serde_json;

mod dsp;
mod types;

use std::collections::HashMap;

use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use types::{Config, Patch};

use std::fs::File;
use std::io::BufWriter;
use std::sync::{Arc, Mutex};

lazy_static! {
    static ref ROOT_ID: String = "ROOT".into();
    static ref ROOT_OUTPUT_PORT: String = "output".into();
}
const DATA: &str = include_str!("data.json");

fn main() -> anyhow::Result<()> {
    // let host = cpal::host_from_id(cpal::HostId::Asio).expect("failed to initialize ASIO host");
    let host = cpal::default_host();

    let device = host.default_output_device().unwrap();

    let config = device.default_output_config().unwrap();

    let patch: Patch = create_patch(serde_json::from_str(DATA)?)?;

    match config.sample_format() {
        cpal::SampleFormat::I16 => run::<i16>(&device, config, patch),
        cpal::SampleFormat::U16 => run::<u16>(&device, config, patch),
        cpal::SampleFormat::F32 => run::<f32>(&device, config, patch),
    }
}

fn create_patch(configs: HashMap<String, Config>) -> Result<Patch> {
    let mut patch = HashMap::new();
    let constructors = dsp::get_constructors();
    for (id, config) in configs {
        if let Some(constructor) = constructors.get(&config.module_type) {
            let module = constructor(&id, config.params)?;
            patch.insert(id, module);
        } else {
            return Err(anyhow!(
                "module with id {}: module type {} does not exist.",
                id,
                config.module_type
            ));
        }
    }
    return Ok(patch);
}

fn run<T>(
    device: &cpal::Device,
    config: cpal::SupportedStreamConfig,
    patch: Patch,
) -> Result<(), anyhow::Error>
where
    T: cpal::Sample,
{
    let sample_rate = config.sample_rate().0 as f32;
    let channels = config.channels() as usize;
    println!("{} {}", sample_rate, channels);
    let args: Vec<String> = std::env::args().collect();
    const MANIFEST_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/");
    let mut path: String =  MANIFEST_DIR.to_owned();
    path.push_str(args.get(1).unwrap_or(&"recorded".to_owned()));
    path.push_str(".wav");
    let spec = wav_spec_from_config(&config);
    let writer = hound::WavWriter::create(path.clone(), spec)?;
    let writer = Arc::new(Mutex::new(Some(writer)));

    // A flag to indicate that recording is in progress.
    println!("Begin recording...");

    // Run the input stream on a separate thread.
    let writer_2 = writer.clone();

    let err_fn = |err| eprintln!("error: {}", err);
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_output_stream(
            &config.into(),
            move |data, _: &_| {
                write_data::<f32, f32>(data, channels, &patch, sample_rate, &writer_2)
            },
            err_fn,
        )?,
        cpal::SampleFormat::I16 => device.build_output_stream(
            &config.into(),
            move |data, _: &_| {
                write_data::<i16, i16>(data, channels, &patch, sample_rate, &writer_2)
            },
            err_fn,
        )?,
        cpal::SampleFormat::U16 => device.build_output_stream(
            &config.into(),
            move |data, _: &_| {
                write_data::<u16, i16>(data, channels, &patch, sample_rate, &writer_2)
            },
            err_fn,
        )?,
    };

    stream.play()?;

    std::thread::sleep(std::time::Duration::from_millis(1000));
    drop(stream);
    writer.lock().unwrap().take().unwrap().finalize()?;
    println!("Recording {} complete!", path);
    Ok(())
}

fn wav_spec_from_config(config: &cpal::SupportedStreamConfig) -> hound::WavSpec {
    hound::WavSpec {
        channels: config.channels() as _,
        sample_rate: config.sample_rate().0 as _,
        bits_per_sample: (config.sample_format().sample_size() * 8) as _,
        sample_format: sample_format(config.sample_format()),
    }
}

fn sample_format(format: cpal::SampleFormat) -> hound::SampleFormat {
    match format {
        cpal::SampleFormat::U16 => hound::SampleFormat::Int,
        cpal::SampleFormat::I16 => hound::SampleFormat::Int,
        cpal::SampleFormat::F32 => hound::SampleFormat::Float,
    }
}

fn update_patch(patch: &Patch, sample_rate: f32) {
    for (_, module) in patch {
        module.update(patch, sample_rate);
    }
}

fn tick_patch(patch: &Patch) {
    for (_, module) in patch {
        module.tick();
    }
}

fn get_patch_output(patch: &Patch) -> f32 {
    if let Some(root) = patch.get(&*ROOT_ID) {
        return root.get_sample(&*ROOT_OUTPUT_PORT).unwrap_or_default();
    } else {
        return 0.0;
    }
}

fn process_frame(patch: &Patch, sample_rate: f32) -> f32 {
    update_patch(patch, sample_rate);
    tick_patch(patch);
    get_patch_output(patch) / 5.0
}

type WavWriterHandle = Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>>>;

fn write_data<T, U>(
    output: &mut [T],
    channels: usize,
    patch: &Patch,
    sample_rate: f32,
    writer: &WavWriterHandle,
) where
    T: cpal::Sample,
    U: cpal::Sample + hound::Sample,
{
    if let Ok(mut guard) = writer.try_lock() {
        if let Some(writer) = guard.as_mut() {
            for frame in output.chunks_mut(channels) {
                let s = &process_frame(patch, sample_rate);
                {
                    let value = cpal::Sample::from::<f32>(s);
                    for sample in frame.iter_mut() {
                        *sample = value;
                        {
                            let value: U = cpal::Sample::from::<f32>(s);
                            writer.write_sample(value).ok();
                        }
                    }
                }
            }
        }
    }
}
