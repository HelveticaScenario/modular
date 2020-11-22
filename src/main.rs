extern crate anyhow;
extern crate cpal;

use std::{rc::Rc, cell::RefCell};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

trait Sampleable {
    fn next_sample(&mut self, frame: u64, patch: Vec<Rc<RefCell<Box<dyn Sampleable>>>>) -> f32;
}

type SampleableConstructor = Fn(()) -> Box<dyn Sampleable>;

fn main() {
    let freq = match std::env::args().nth(1) {
        Some(f) => f.parse::<f32>(),
        None => Ok(440f32),
    }
    .unwrap();

    let host = cpal::host_from_id(cpal::HostId::Asio).expect("failed to initialise ASIO host");

    let device = host.default_output_device().unwrap();

    let config = device.default_output_config().unwrap();

    match config.sample_format() {
        cpal::SampleFormat::I16 => run::<i16>(&device, &config.into(), freq).unwrap(),
        cpal::SampleFormat::U16 => run::<u16>(&device, &config.into(), freq).unwrap(),
        cpal::SampleFormat::F32 => run::<f32>(&device, &config.into(), freq).unwrap(),
    }
}

fn clamp<T: std::cmp::PartialOrd>(min: T, max:T, val: T) -> T  {
    if val < min {
        min
    } else if val > max {
        max
    } else {
        val
    }
} 

fn run<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    freq: f32,
) -> Result<(), anyhow::Error>
where
    T: cpal::Sample,
{
    let sample_rate = config.sample_rate.0 as f32;
    let channels = config.channels as usize;

    let mut sample_clock = 0f32;
    let mut next_value = move || {
        sample_clock = (sample_clock + 1.0) % sample_rate;
        clamp(-1.0, 1.0, (sample_clock * freq * 2.0 * std::f32::consts::PI / sample_rate).sin() * 10.0)
    };

    let err_fn = |err| eprintln!("error: {}", err);

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _| write_data(data, channels, &mut next_value),
        err_fn,
    )?;
    stream.play()?;
    
    std::thread::sleep(std::time::Duration::from_millis(1000));

    Ok(())
}

fn write_data<T>(output: &mut [T], channels: usize, next_sample: &mut dyn FnMut() -> f32)
where
    T: cpal::Sample,
{
    for frame in output.chunks_mut(channels) {
        let value = cpal::Sample::from::<f32>(&next_sample());
        for sample in frame.iter_mut() {
            *sample = value
        }
    }
}
