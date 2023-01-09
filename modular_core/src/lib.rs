#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate modular_derive;

extern crate anyhow;
extern crate cpal;
extern crate hound;
extern crate parking_lot;
extern crate serde;
extern crate serde_json;

pub mod dsp;
pub mod patch;
mod procedure;
mod sequence;
pub mod types;

use std::{rc::Rc, sync::Arc, thread, time::Duration};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    PauseStreamError, PlayStreamError, Sample, Stream, StreamInstant,
};
pub use crossbeam_channel;
use crossbeam_channel::select;
use dsp::schema;

use parking_lot::Mutex;
use patch::Patch;
use procedure::new_procedure;
use types::ModuleSchema;
pub use uuid;

pub struct Modular {
    pub patch: Arc<Mutex<Patch>>,
    pub schema: Vec<ModuleSchema>,
    _stream_handle: thread::JoinHandle<()>,
    play_procedure: procedure::Procedure<Rc<Stream>, Result<(), PlayStreamError>>,
    pause_procedure: procedure::Procedure<Rc<Stream>, Result<(), PauseStreamError>>,
}

impl Modular {
    pub fn new() -> Modular {
        // let host = cpal::host_from_id(cpal::HostId::Asio).expect("failed to initialize ASIO host");
        let host = cpal::default_host();

        let device = host.default_output_device().unwrap();

        let config = device.default_output_config().unwrap();
        let patch = Arc::new(Mutex::new(Patch::new(config.sample_rate())));
        let (play_procedure, pause_procedure, _stream_handle) = match config.sample_format() {
            cpal::SampleFormat::I16 => Modular::stream_make::<i16>(device, config, patch.clone()),
            cpal::SampleFormat::U16 => Modular::stream_make::<u16>(device, config, patch.clone()),
            cpal::SampleFormat::F32 => Modular::stream_make::<f32>(device, config, patch.clone()),
        };
        Modular {
            patch,
            schema: schema(),
            _stream_handle,
            play_procedure,
            pause_procedure,
        }
    }

    fn stream_make<T>(
        device: cpal::Device,
        config: cpal::SupportedStreamConfig,
        patch: Arc<Mutex<Patch>>,
    ) -> (
        procedure::Procedure<Rc<Stream>, Result<(), PlayStreamError>>,
        procedure::Procedure<Rc<Stream>, Result<(), PauseStreamError>>,
        thread::JoinHandle<()>,
    )
    where
        T: Sample,
    {
        let (play_procedure, play_procedure_handler) =
            new_procedure::<Rc<Stream>, Result<(), PlayStreamError>>();
        let (pause_procedure, pause_procedure_handler) =
            new_procedure::<Rc<Stream>, Result<(), PauseStreamError>>();

        let channels = config.channels() as usize;
        let stream_handle = thread::spawn(move || {
            let mut last_instant: Option<StreamInstant> = None;
            let stream = Rc::new(
                device
                    .build_output_stream(
                        &config.into(),
                        move |data: &mut [T], info: &_| {
                            let new_instant = info.timestamp().callback;

                            let delta = match last_instant {
                                Some(last_instant) => new_instant.duration_since(&last_instant),
                                None => None,
                            }
                            .unwrap_or(Duration::from_nanos(0));
                            last_instant = Some(new_instant);
                            let mut patch = patch.lock();
                            patch.write_data(data, channels, &delta)
                        },
                        |err| eprintln!("error: {}", err),
                    )
                    .unwrap(),
            );
            loop {
                select! {
                    recv(play_procedure_handler.rx) -> cb => play_procedure_handler.handle(stream.clone(), cb.unwrap()),
                    recv(pause_procedure_handler.rx) -> cb => pause_procedure_handler.handle(stream.clone(), cb.unwrap())
                }
            }
        });
        (play_procedure, pause_procedure, stream_handle)
    }

    pub fn play(&self) -> Result<(), PlayStreamError> {
        self.play_procedure.call(Box::new(|stream| stream.play()))
    }

    pub fn pause(&self) -> Result<(), PauseStreamError> {
        self.pause_procedure.call(Box::new(|stream| stream.pause()))
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
