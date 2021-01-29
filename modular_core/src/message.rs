use crossbeam_channel::Sender;
use std::sync::Arc;
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
    patch: &mut Patch,
    sender: &Sender<OutputMessage>,
) -> anyhow::Result<()> {
    println!("{:?}", message);
    let sampleables = patch.sampleables.clone();
    let tracks = patch.tracks.clone();
    match message {
        InputMessage::Echo(s) => sender.send(OutputMessage::Echo(format!("{}!", s)))?,
        InputMessage::Schema => sender.send(OutputMessage::Schema(schema()))?,
        InputMessage::GetModules => {
            sender.send(OutputMessage::PatchState(
                sampleables
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|(_key, val)| val.get_state())
                    .collect(),
            ))?;
        }
        InputMessage::GetModule(id) => {
            let state = sampleables
                .lock()
                .unwrap()
                .get(&id)
                .map(|module| module.get_state());
            sender.send(OutputMessage::ModuleState(id, state))?;
        }
        InputMessage::CreateModule(module_type, id) => {
            let constructors = get_constructors();
            if let Some(constructor) = constructors.get(&module_type) {
                match constructor(&id) {
                    Ok(module) => {
                        sampleables.lock().unwrap().insert(id.clone(), module);
                        sender.send(OutputMessage::CreateModule(module_type, id))?
                    }
                    Err(err) => {
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
            let sampleables = sampleables.lock().unwrap();
            let tracks = tracks.lock().unwrap();
            match sampleables.get(&id) {
                Some(module) => module.update_param(
                    &param_name,
                    &new_param.to_internal_param(&sampleables, &tracks),
                )?,
                None => sender.send(OutputMessage::Error(format!("{} not found", id)))?,
            }
        }
        InputMessage::DeleteModule(id) => {
            sampleables.lock().unwrap().remove(&id);
        }
        InputMessage::GetTracks => {
            for (_, internal_track) in tracks.lock().unwrap().iter() {
                sender.send(OutputMessage::Track(internal_track.to_track()))?;
            }
        }
        InputMessage::GetTrack(id) => {
            if let Some(ref internal_track) = tracks.lock().unwrap().get(&id) {
                sender.send(OutputMessage::Track(internal_track.to_track()))?;
            }
        }
        InputMessage::CreateTrack(id) => {
            tracks
                .lock()
                .unwrap()
                .insert(id.clone(), Arc::new(InternalTrack::new(id.clone())));
            sender.send(OutputMessage::CreateTrack(id))?
        }
        InputMessage::UpdateTrack(id, track_update) => {
            if let Some(ref internal_track) = tracks.lock().unwrap().get(&id) {
                internal_track.update(&track_update)
            }
        }
        InputMessage::DeleteTrack(id) => {
            tracks.lock().unwrap().remove(&id);
        }
        InputMessage::UpsertKeyframe(keyframe) => {
            let ref tracks = tracks.lock().unwrap();
            let internal_keyframe =
                keyframe.to_internal_keyframe(&sampleables.lock().unwrap(), tracks);

            if let Some(ref track) = tracks.get(&keyframe.track_id) {
                track.add_keyframe(internal_keyframe);
            }
        }
        InputMessage::DeleteKeyframe(id, track_id) => {
            if let Some(ref track) = tracks.lock().unwrap().get(&track_id) {
                track.remove_keyframe(id);
            }
        }
    };
    Ok(())
}
