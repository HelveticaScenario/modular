use std::sync::{
    mpsc::{Receiver, Sender},
    Arc, Mutex,
};

use crate::{
    message::{handle_message, Message},
    types::{PatchMap, ROOT_ID, ROOT_OUTPUT_PORT},
};
use cpal::traits::{DeviceTrait, StreamTrait};
pub struct Patch {
    pub map: Arc<Mutex<PatchMap>>,
}

impl Patch {
    pub fn new(map: PatchMap) -> Self {
        Patch {
            map: Arc::new(Mutex::new(map)),
        }
    }

    pub fn run<T>(
        &mut self,
        device: &cpal::Device,
        config: cpal::SupportedStreamConfig,
        reciever: Receiver<Message>,
        sender: Sender<Message>,
    ) -> Result<(), anyhow::Error>
    where
        T: cpal::Sample,
    {
        let sample_rate = config.sample_rate().0 as f32;
        let channels = config.channels() as usize;
        println!("{} {}", sample_rate, channels);

        let err_fn = |err| eprintln!("error: {}", err);
        let patch_map = self.map.clone();
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => device.build_output_stream(
                &config.into(),
                move |data, _: &_| {
                    if let Ok(ref patch_map) = patch_map.lock() {
                        write_data::<f32>(data, channels, &patch_map, sample_rate)
                    }
                },
                err_fn,
            )?,
            cpal::SampleFormat::I16 => device.build_output_stream(
                &config.into(),
                move |data, _: &_| {
                    if let Ok(ref patch_map) = patch_map.lock() {
                        write_data::<i16>(data, channels, &patch_map, sample_rate)
                    }
                },
                err_fn,
            )?,
            cpal::SampleFormat::U16 => device.build_output_stream(
                &config.into(),
                move |data, _: &_| {
                    if let Ok(ref patch_map) = patch_map.lock() {
                        write_data::<u16>(data, channels, &patch_map, sample_rate)
                    }
                },
                err_fn,
            )?,
        };

        stream.play()?;

        for message in reciever {
            handle_message(message, self, &sender)?;
        }

        Ok(())
    }
}

fn write_data<T>(output: &mut [T], channels: usize, patch_map: &PatchMap, sample_rate: f32)
where
    T: cpal::Sample,
{
    for frame in output.chunks_mut(channels) {
        let value = cpal::Sample::from::<f32>(&process_frame(patch_map, sample_rate));
        for sample in frame.iter_mut() {
            *sample = value;
        }
    }
}

fn update_patch(patch_map: &PatchMap, sample_rate: f32) {
    for (_, module) in patch_map {
        module.update(patch_map, sample_rate);
    }
}

fn tick_patch(patch_map: &PatchMap) {
    for (_, module) in patch_map {
        module.tick();
    }
}

fn get_patch_output(patch_map: &PatchMap) -> f32 {
    if let Some(root) = patch_map.get(&*ROOT_ID) {
        return root.get_sample(&*ROOT_OUTPUT_PORT).unwrap_or_default();
    } else {
        return 0.0;
    }
}

fn process_frame(patch_map: &PatchMap, sample_rate: f32) -> f32 {
    update_patch(patch_map, sample_rate);
    tick_patch(patch_map);
    get_patch_output(patch_map) / 5.0
}
