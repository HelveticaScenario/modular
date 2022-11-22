#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate modular_derive;

extern crate anyhow;
extern crate atomic_float;
extern crate cpal;
extern crate hound;
extern crate parking_lot;
extern crate serde;
extern crate serde_json;

pub mod dsp;
pub mod message;
pub mod patch;
mod sequence;
pub mod types;

use std::thread;

use cpal::traits::{DeviceTrait, HostTrait};
pub use crossbeam_channel;
use crossbeam_channel::{Receiver, Sender};
use message::{InputMessage, OutputMessage};
use patch::Patch;
use thread::JoinHandle;
pub use uuid;

pub struct Modular;

impl Modular {
    pub fn spawn(
        incoming_rx: Receiver<InputMessage>,
        outgoing_tx: Sender<OutputMessage>,
    ) -> JoinHandle<anyhow::Result<()>> {
        // let host = cpal::host_from_id(cpal::HostId::Asio).expect("failed to initialize ASIO host");
        let host = cpal::default_host();

        let device = host.default_output_device().unwrap();

        let config = device.default_output_config().unwrap();

        thread::spawn(move || match config.sample_format() {
            cpal::SampleFormat::I16 => Patch::run::<i16>(&device, config, incoming_rx, outgoing_tx),
            cpal::SampleFormat::U16 => Patch::run::<u16>(&device, config, incoming_rx, outgoing_tx),
            cpal::SampleFormat::F32 => Patch::run::<f32>(&device, config, incoming_rx, outgoing_tx),
        })
    }
}

// fn create_patch(mut configs: HashMap<Uuid, Config>) -> Result<Patch> {
//     if !configs.contains_key(&ROOT_ID) {
//         configs.insert(
//             ROOT_ID.clone(),
//             Config {
//                 module_type: "signal".into(),
//                 params: Value::Object(Map::new()),
//             },
//         );
//     }
//     let mut map = HashMap::new();
//     let constructors = dsp::get_constructors();
//     for (id, config) in configs {
//         if let Some(constructor) = constructors.get(&config.module_type) {
//             let module = constructor(&id)?;
//             map.insert(id, module);
//         } else {
//             return Err(anyhow!(
//                 "module with id {}: module type {} does not exist.",
//                 id,
//                 config.module_type
//             ));
//         }
//     }
//     return Ok(Patch::new(map));
// }

// fn wav_spec_from_config(config: &cpal::SupportedStreamConfig) -> hound::WavSpec {
//     hound::WavSpec {
//         channels: config.channels() as _,
//         sample_rate: config.sample_rate().0 as _,
//         bits_per_sample: (config.sample_format().sample_size() * 8) as _,
//         sample_format: sample_format(config.sample_format()),
//     }
// }

// fn sample_format(format: cpal::SampleFormat) -> hound::SampleFormat {
//     match format {
//         cpal::SampleFormat::U16 => hound::SampleFormat::Int,
//         cpal::SampleFormat::I16 => hound::SampleFormat::Int,
//         cpal::SampleFormat::F32 => hound::SampleFormat::Float,
//     }
// }

// type WavWriterHandle = Arc<Mutex<Option<hound::WavWriter<BufWriter<File>>>>>;
