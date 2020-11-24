#[macro_use]
extern crate lazy_static;

extern crate anyhow;
extern crate cpal;
extern crate serde;
extern crate serde_json;

mod dsp;
mod types;

use std::collections::HashMap;

use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use types::{Config, Patch, Sampleable};

lazy_static! {
    static ref ROOT_ID: String = "ROOT".into();
    static ref ROOT_OUTPUT_PORT: String = "output".into();
}
// const ROOT_ID: &str = "ROOT";
// const ROOT_OUTPUT_PORT: &str = "output";
const DATA: &str = include_str!("data.json");


fn main() -> anyhow::Result<()> {
    // let host = cpal::host_from_id(cpal::HostId::Asio).expect("failed to initialise ASIO host");
    let host = cpal::default_host();

    let device = host.default_output_device().unwrap();

    let config = device.default_output_config().unwrap();

    let patch: Patch = create_patch(serde_json::from_str(DATA)?)?;

    match config.sample_format() {
        cpal::SampleFormat::I16 => run::<i16>(&device, &config.into(), patch),
        cpal::SampleFormat::U16 => run::<u16>(&device, &config.into(), patch),
        cpal::SampleFormat::F32 => run::<f32>(&device, &config.into(), patch),
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
    config: &cpal::StreamConfig,
    patch: Patch,
) -> Result<(), anyhow::Error>
where
    T: cpal::Sample,
{
    let sample_rate = config.sample_rate.0 as f32;
    let channels = config.channels as usize;
    // let a = signal::rate(sample_rate).const_hz(440.0);

    let err_fn = |err| eprintln!("error: {}", err);
    // for i in 0..10000 {
    //     println!("{}: {}", i, process_frame(&patch));
    // }

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _| write_data(data, channels, &patch, sample_rate),
        err_fn,
    )?;
    stream.play()?;

    std::thread::sleep(std::time::Duration::from_millis(1000));

    Ok(())
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

fn write_data<T>(output: &mut [T], channels: usize, patch: &Patch, sample_rate: f32)
where
    T: cpal::Sample,
{
    for frame in output.chunks_mut(channels) {
        let value = cpal::Sample::from::<f32>(&process_frame(patch, sample_rate));
        for sample in frame.iter_mut() {
            *sample = value
        }
    }
}
