use crossbeam_channel::Sender;
use parking_lot::{Mutex, RwLock};
use std::{sync::Arc, time::Duration};
use uuid::Uuid;

use crate::{
    dsp::get_constructors,
    dsp::schema,
    patch::Patch,
    types::ModuleSchema,
    types::{InternalTrack, Keyframe, ModuleState, Param, Track, TrackUpdate},
};

#[derive(Debug, Clone)]
pub enum InputMessage {
    Echo(String),
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
}

#[derive(Debug, Clone)]
pub enum OutputMessage {
    Echo(String),
    Schema(Vec<ModuleSchema>),
    PatchState(Vec<ModuleState>),
    ModuleState(Uuid, Option<ModuleState>),
    Track(Track),
    CreateModule(String, Uuid),
    CreateTrack(Uuid),
    Error(String),
}

pub fn handle_message(
    message: InputMessage,
    patch: &Arc<Mutex<Patch>>,
    sender: &Sender<OutputMessage>,
    sample_rate: f32,
) -> anyhow::Result<()> {
    println!("{:?}", message);
    match message {
        InputMessage::Echo(s) => sender.send(OutputMessage::Echo(format!("{}!", s)))?,
        InputMessage::Schema => sender.send(OutputMessage::Schema(schema()))?,
        InputMessage::GetModules => {
            sender.send(OutputMessage::PatchState(
                patch
                    .try_lock_for(Duration::from_millis(10))
                    .unwrap()
                    .sampleables
                    .iter()
                    .map(|(_key, val)| val.get_state())
                    .collect(),
            ))?;
        }
        InputMessage::GetModule(id) => {
            let state = patch
                .try_lock_for(Duration::from_millis(10))
                .unwrap()
                .sampleables
                .get(&id)
                .map(|module| module.get_state());
            sender.send(OutputMessage::ModuleState(id, state))?;
        }
        InputMessage::CreateModule(module_type, id) => {
            let constructors = get_constructors();
            println!("sample rate {}", sample_rate);
            if let Some(constructor) = constructors.get(&module_type) {
                match constructor(&id, sample_rate) {
                    Ok(module) => {
                        println!("attempt write");
                        patch
                            .try_lock_for(Duration::from_millis(10))
                            .unwrap()
                            .sampleables
                            .insert(id.clone(), module);
                        println!("written");
                        sender.send(OutputMessage::CreateModule(module_type, id))?
                    }
                    Err(err) => {
                        println!("{}", err);
                        sender.send(OutputMessage::Error(format!("an error occured: {}", err)))?;
                    }
                };
            } else {
                sender.send(OutputMessage::Error(format!(
                    "{} is not a valid module type",
                    module_type
                )))?;
            }
        }
        InputMessage::UpdateParam(id, param_name, new_param) => {
            let patch = patch.try_lock_for(Duration::from_millis(10)).unwrap();
            match patch.sampleables.get(&id) {
                Some(module) => {
                    module.update_param(&param_name, &new_param.to_internal_param(&patch))?
                }
                None => sender.send(OutputMessage::Error(format!("{} not found", id)))?,
            }
        }
        InputMessage::DeleteModule(id) => {
            patch
                .try_lock_for(Duration::from_millis(10))
                .unwrap()
                .sampleables
                .remove(&id);
        }
        InputMessage::GetTracks => {
            for (_, internal_track) in patch
                .try_lock_for(Duration::from_millis(10))
                .unwrap()
                .tracks
                .iter()
            {
                sender.send(OutputMessage::Track(internal_track.to_track()))?;
            }
        }
        InputMessage::GetTrack(id) => {
            if let Some(ref internal_track) = patch
                .try_lock_for(Duration::from_millis(10))
                .unwrap()
                .tracks
                .get(&id)
            {
                sender.send(OutputMessage::Track(internal_track.to_track()))?;
            }
        }
        InputMessage::CreateTrack(id) => {
            patch
                .try_lock_for(Duration::from_millis(10))
                .unwrap()
                .tracks
                .insert(id.clone(), Arc::new(InternalTrack::new(id.clone())));
            sender.send(OutputMessage::CreateTrack(id))?
        }
        InputMessage::UpdateTrack(id, track_update) => {
            if let Some(ref internal_track) = patch.lock().tracks.get(&id) {
                internal_track.update(&track_update)
            }
        }
        InputMessage::DeleteTrack(id) => {
            patch
                .try_lock_for(Duration::from_millis(10))
                .unwrap()
                .tracks
                .remove(&id);
        }
        InputMessage::UpsertKeyframe(keyframe) => {
            let internal_keyframe = keyframe
                .to_internal_keyframe(&patch.try_lock_for(Duration::from_millis(10)).unwrap());

            if let Some(ref track) = patch
                .try_lock_for(Duration::from_millis(10))
                .unwrap()
                .tracks
                .get(&keyframe.track_id)
            {
                track.add_keyframe(internal_keyframe);
            }
        }
        InputMessage::DeleteKeyframe(id, track_id) => {
            if let Some(ref track) = patch
                .try_lock_for(Duration::from_millis(10))
                .unwrap()
                .tracks
                .get(&track_id)
            {
                track.remove_keyframe(id);
            }
        }
    };
    Ok(())
}
