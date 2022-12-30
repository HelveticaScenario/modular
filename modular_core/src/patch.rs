use std::{collections::HashMap, sync::Arc, time::Duration};

use crate::{
    dsp::get_constructors,
    types::{
        InternalTrack, Keyframe, ModuleState, Param, SampleableMap, Track, TrackMap, TrackUpdate,
        ROOT_ID, ROOT_OUTPUT_PORT,
    },
};
use anyhow::anyhow;
use cpal::SampleRate;
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
            ROOT_ID.clone(),
            get_constructors().get(&"signal".to_owned()).unwrap()(&ROOT_ID, sample_rate).unwrap(),
        );
        Patch {
            sampleables,
            tracks,
            sample_rate,
        }
    }

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

    pub fn get_module(&self, id: &String) -> Option<ModuleState> {
        self.sampleables.get(id).map(|module| module.get_state())
    }

    pub fn create_module(&mut self, module_type: String, id: &String) -> Result<(), anyhow::Error> {
        let constructors = get_constructors();
        println!("sample rate {}", self.sample_rate);
        if let Some(constructor) = constructors.get(&module_type) {
            constructor(id, self.sample_rate).map(|module| {
                self.sampleables.insert(id.clone(), module);
            })
        } else {
            Err(anyhow!("{} is not a valid module type", module_type))
        }
    }

    pub fn update_param(
        &self,
        id: &String,
        param_name: &String,
        new_param: &Param,
    ) -> Result<(), anyhow::Error> {
        match self.sampleables.get(id) {
            Some(module) => module.update_param(param_name, &new_param.to_internal_param(self)),
            None => Err(anyhow!("{} not found", id)),
        }
    }

    pub fn delete_module(&mut self, id: &String) {
        self.sampleables.remove(id);
    }

    pub fn get_tracks(&self) -> Vec<Track> {
        self.tracks
            .iter()
            .map(|(_, internal_track)| internal_track.to_track())
            .collect()
    }
    pub fn get_track(&self, id: &String) -> Option<Track> {
        self.tracks
            .get(id)
            .map(|internal_track| internal_track.to_track())
    }

    pub fn create_track(&mut self, id: &String) -> Option<Arc<InternalTrack>> {
        self.tracks
            .insert(id.clone(), Arc::new(InternalTrack::new(id.clone())))
    }

    pub fn update_track(
        &self,
        id: &String,
        track_update: TrackUpdate,
    ) -> Result<(), anyhow::Error> {
        match self.tracks.get(id) {
            Some(ref internal_track) => {
                internal_track.update(&track_update);
                Ok(())
            }
            None => Err(anyhow!("{} not found", id)),
        }
    }

    pub fn delete_track(&mut self, id: &String) {
        self.tracks.remove(id);
    }

    pub fn upsert_keyframe(&self, keyframe: &Keyframe) -> Result<(), anyhow::Error> {
        let internal_keyframe = keyframe.to_internal_keyframe(self);

        match self.tracks.get(&keyframe.track_id) {
            Some(ref track) => {
                track.add_keyframe(internal_keyframe);
                Ok(())
            }
            None => Err(anyhow!("{} not found", keyframe.track_id)),
        }
    }

    pub fn delete_keyframe(&self, id: &String, track_id: &String) {
        if let Some(ref track) = self.tracks.get(track_id) {
            track.remove_keyframe(id.clone());
        }
    }
}
