use std::{collections::HashMap, sync::Arc, time::Duration};

use crate::{
    dsp::get_constructors,
    // message::OutputMessage,
    types::{
        InternalTrack, ModuleState, Param, SampleableMap, Track, TrackMap, ROOT_ID,
        ROOT_OUTPUT_PORT,
    },
};
use anyhow::anyhow;
use cpal::SampleRate;
use uuid::Uuid;
pub struct Patch {
    pub sampleables: SampleableMap,
    pub tracks: TrackMap,
    sample_rate: f32,
}

impl Patch {
    pub fn new(sample_rate: SampleRate) -> Self {
        let mut sampleables = HashMap::new();
        let tracks = HashMap::new();
        let sample_rate = sample_rate.0 as f32;
        sampleables.insert(
            Uuid::nil(),
            get_constructors().get(&"signal".to_owned()).unwrap()(&Uuid::nil(), sample_rate)
                .unwrap(),
        );
        Patch {
            sampleables,
            tracks,
            sample_rate,
        }
    }

    // pub fn run<T>(
    //     device: &cpal::Device,
    //     config: cpal::SupportedStreamConfig,
    //     receiver: Receiver<InputMessage>,
    //     sender: Sender<OutputMessage>,
    // ) -> Result<(), anyhow::Error>
    // where
    //     T: cpal::Sample,
    // {
    //     let sample_rate = config.sample_rate().0 as f32;
    //     let patch = Arc::new(Mutex::new(Patch::new(HashMap::new(), HashMap::new())));
    //     let channels = config.channels() as usize;
    //     println!("{} {}", sample_rate, channels);

    //     let err_fn = |err| eprintln!("error: {}", err);
    //     patch.clone().lock().sampleables.insert(
    //         Uuid::nil(),
    //         get_constructors().get(&"signal".to_owned()).unwrap()(&Uuid::nil(), sample_rate)
    //             .unwrap(),
    //     );
    //     let patch_clone = patch.clone();

    //     let mut last_instant: Option<StreamInstant> = None;
    //     // let stream = match config.sample_format() {
    //     //     cpal::SampleFormat::F32 => device.build_output_stream(
    //     //         &config.into(),
    //     //         move |data, info: &_| {
    //     //             let new_instant = info.timestamp().callback;

    //     //             let delta = match last_instant {
    //     //                 Some(last_instant) => new_instant.duration_since(&last_instant),
    //     //                 None => None,
    //     //             }
    //     //             .unwrap_or(Duration::from_nanos(0));
    //     //             last_instant = Some(new_instant);
    //     //             let mut patch = patch_clone.lock();
    //     //             write_data::<f32>(data, channels, &mut patch, &delta)
    //     //         },
    //     //         err_fn,
    //     //     )?,
    //     //     cpal::SampleFormat::I16 => device.build_output_stream(
    //     //         &config.into(),
    //     //         move |data, info: &_| {
    //     //             let new_instant = info.timestamp().callback;

    //     //             let delta = match last_instant {
    //     //                 Some(last_instant) => new_instant.duration_since(&last_instant),
    //     //                 None => None,
    //     //             }
    //     //             .unwrap_or(Duration::from_nanos(0));
    //     //             last_instant = Some(new_instant);
    //     //             let mut patch = patch_clone.lock();
    //     //             write_data::<i16>(data, channels, &mut patch, &delta)
    //     //         },
    //     //         err_fn,
    //     //     )?,
    //     //     cpal::SampleFormat::U16 => device.build_output_stream(
    //     //         &config.into(),
    //     //         move |data, info: &_| {
    //     //             let new_instant = info.timestamp().callback;

    //     //             let delta = match last_instant {
    //     //                 Some(last_instant) => new_instant.duration_since(&last_instant),
    //     //                 None => None,
    //     //             }
    //     //             .unwrap_or(Duration::from_nanos(0));
    //     //             last_instant = Some(new_instant);
    //     //             let mut patch = patch_clone.lock();
    //     //             write_data::<u16>(data, channels, &mut patch, &delta)
    //     //         },
    //     //         err_fn,
    //     //     )?,
    //     // };

    //     stream.play()?;

    //     // for message in receiver {
    //     //     handle_message(message, &patch, &sender, sample_rate)?;
    //     // }
    //     Ok(())
    // }

    pub fn write_data<T>(&mut self, output: &mut [T], channels: usize, delta: &Duration)
    where
        T: cpal::Sample,
    {
        for frame in output.chunks_mut(channels) {
            let value = cpal::Sample::from::<f32>(&self.process_frame(delta));
            for sample in frame.iter_mut() {
                *sample = value;
            }
        }
    }

    fn update_tracks(&mut self, delta: &Duration) {
        for (_, track) in self.tracks.iter() {
            track.tick(delta);
        }
    }

    fn update_sampleables(&mut self) {
        for (_, module) in self.sampleables.iter() {
            module.update();
        }
    }

    fn tick_sampleables(&mut self) {
        for (_, module) in self.sampleables.iter() {
            module.tick();
        }
    }

    fn get_patch_output(&self) -> f32 {
        if let Some(root) = self.sampleables.get(&*ROOT_ID) {
            return root.get_sample(&*ROOT_OUTPUT_PORT).unwrap_or_default();
        } else {
            return 0.0;
        }
    }

    fn process_frame(&mut self, delta: &Duration) -> f32 {
        self.update_tracks(delta);
        self.update_sampleables();
        self.tick_sampleables();
        self.get_patch_output() / 5.0
    }

    pub fn get_modules(&self) -> Vec<ModuleState> {
        self.sampleables
            .iter()
            .map(|(_key, val)| val.get_state())
            .collect()
    }

    pub fn get_module(&self, id: Uuid) -> Option<ModuleState> {
        self.sampleables.get(&id).map(|module| module.get_state())
    }

    pub fn create_module(&mut self, module_type: String, id: Uuid) -> Result<(), anyhow::Error> {
        let constructors = get_constructors();
        println!("sample rate {}", self.sample_rate);
        if let Some(constructor) = constructors.get(&module_type) {
            constructor(&id, self.sample_rate).map(|module| {
                self.sampleables.insert(id.clone(), module);
            })
        } else {
            Err(anyhow!("{} is not a valid module type", module_type))
        }
    }

    pub fn update_param(
        &self,
        id: Uuid,
        param_name: String,
        new_param: Param,
    ) -> Result<(), anyhow::Error> {
        match self.sampleables.get(&id) {
            Some(module) => module.update_param(&param_name, &new_param.to_internal_param(self)),
            None => Err(anyhow!("{} not found", id)),
        }
    }

    pub fn delete_module(&mut self, id: Uuid) {
        self.sampleables.remove(&id);
    }

    /*
    Schema,
    GetModules,
    GetModule(Uuid),
    CreateModule(String, Uuid),
    UpdateParam(Uuid, String, Param),
    DeleteModule(Uuid),

    GetTracks,
    GetTrack(Uuid),
    CreateTrack(Uuid),
    UpdateTrack(Uuid, TrackUpdate),
    DeleteTrack(Uuid),
    UpsertKeyframe(Keyframe),
    DeleteKeyframe(Uuid, Uuid),
     */

    pub fn get_tracks(&self) -> Vec<Track> {
        self.tracks
            .iter()
            .map(|(_, internal_track)| internal_track.to_track())
            .collect()
    }
    pub fn get_track(&self, id: Uuid) -> Option<Track> {
        self.tracks
            .get(&id)
            .map(|internal_track| internal_track.to_track())
    }

    pub fn create_track(&mut self, id: Uuid) -> Option<Arc<InternalTrack>> {
        self.tracks
            .insert(id.clone(), Arc::new(InternalTrack::new(id.clone())))
    }

    // pub fn update_track(&self, i)

    //         InputMessage::UpdateTrack(id, track_update) => {
    //             if let Some(ref internal_track) = patch.lock().tracks.get(&id) {
    //                 internal_track.update(&track_update)
    //             }
    //         }
    //         InputMessage::DeleteTrack(id) => {
    //             patch.tracks.remove(&id);
    //         }
    //         InputMessage::UpsertKeyframe(keyframe) => {
    //             let internal_keyframe = keyframe
    //                 .to_internal_keyframe(&patch.try_lock_for(Duration::from_millis(10)).unwrap());

    //             if let Some(ref track) = patch.tracks.get(&keyframe.track_id) {
    //                 track.add_keyframe(internal_keyframe);
    //             }
    //         }
    //         InputMessage::DeleteKeyframe(id, track_id) => {
    //             if let Some(ref track) = patch.tracks.get(&track_id) {
    //                 track.remove_keyframe(id);
    //             }
    //         }
}
