use crossbeam_channel::Sender;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

use crate::{
    dsp::get_constructors,
    dsp::schema,
    patch::Patch,
    types::ModuleSchema,
    types::{InternalTrack, Keyframe, ModuleState, Param, Track, TrackUpdate},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum InputMessage {
    Echo { message: String },
    Schema,
    GetModules,
    GetModule { id: String },
    CreateModule { module_type: String, id: String },
    UpdateParam { id: String, param_name: String, param: Param },
    DeleteModule { id: String },

    GetTracks,
    GetTrack { id: String },
    CreateTrack { id: String },
    UpdateTrack { id: String, update: TrackUpdate },
    DeleteTrack { id: String },
    UpsertKeyframe { keyframe: Keyframe },
    DeleteKeyframe { track_id: String, keyframe_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum OutputMessage {
    Echo { message: String },
    Schema { schemas: Vec<ModuleSchema> },
    PatchState { modules: Vec<ModuleState> },
    ModuleState { id: String, state: Option<ModuleState> },
    Track { track: Track },
    CreateModule { module_type: String, id: String },
    CreateTrack { id: String },
    Error { message: String },
}

pub fn handle_message(
    message: InputMessage,
    patch: &Arc<Mutex<Patch>>,
    sender: &Sender<OutputMessage>,
    sample_rate: f32,
) -> anyhow::Result<()> {
    println!("{:?}", message);
    match message {
        InputMessage::Echo { message: s } => sender.send(OutputMessage::Echo { message: format!("{}!", s) })?,
        InputMessage::Schema => sender.send(OutputMessage::Schema { schemas: schema() })?,
        InputMessage::GetModules => {
            sender.send(OutputMessage::PatchState {
                modules: patch
                    .try_lock_for(Duration::from_millis(10))
                    .unwrap()
                    .sampleables
                    .iter()
                    .map(|(_key, val)| val.get_state())
                    .collect(),
            })?;
        }
        InputMessage::GetModule { id } => {
            let state = patch
                .try_lock_for(Duration::from_millis(10))
                .unwrap()
                .sampleables
                .get(&id)
                .map(|module| module.get_state());
            sender.send(OutputMessage::ModuleState { id, state })?;
        }
        InputMessage::CreateModule { module_type, id } => {
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
                        sender.send(OutputMessage::CreateModule { module_type, id })?
                    }
                    Err(err) => {
                        println!("{}", err);
                        sender.send(OutputMessage::Error { message: format!("an error occured: {}", err) })?;
                    }
                };
            } else {
                sender.send(OutputMessage::Error {
                    message: format!("{} is not a valid module type", module_type)
                })?;
            }
        }
        InputMessage::UpdateParam { id, param_name, param: new_param } => {
            let patch = patch.try_lock_for(Duration::from_millis(10)).unwrap();
            match patch.sampleables.get(&id) {
                Some(module) => {
                    module.update_param(&param_name, &new_param.to_internal_param(&patch))?
                }
                None => sender.send(OutputMessage::Error { message: format!("{} not found", id) })?,
            }
        }
        InputMessage::DeleteModule { id } => {
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
                sender.send(OutputMessage::Track { track: internal_track.to_track() })?;
            }
        }
        InputMessage::GetTrack { id } => {
            if let Some(ref internal_track) = patch
                .try_lock_for(Duration::from_millis(10))
                .unwrap()
                .tracks
                .get(&id)
            {
                sender.send(OutputMessage::Track { track: internal_track.to_track() })?;
            }
        }
        InputMessage::CreateTrack { id } => {
            patch
                .try_lock_for(Duration::from_millis(10))
                .unwrap()
                .tracks
                .insert(id.clone(), Arc::new(InternalTrack::new(id.clone())));
            sender.send(OutputMessage::CreateTrack { id })?
        }
        InputMessage::UpdateTrack { id, update: track_update } => {
            if let Some(ref internal_track) = patch.lock().tracks.get(&id) {
                internal_track.update(&track_update)
            }
        }
        InputMessage::DeleteTrack { id } => {
            patch
                .try_lock_for(Duration::from_millis(10))
                .unwrap()
                .tracks
                .remove(&id);
        }
        InputMessage::UpsertKeyframe { keyframe } => {
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
        InputMessage::DeleteKeyframe { keyframe_id, track_id } => {
            if let Some(ref track) = patch
                .try_lock_for(Duration::from_millis(10))
                .unwrap()
                .tracks
                .get(&track_id)
            {
                track.remove_keyframe(keyframe_id);
            }
        }
    };
    Ok(())
}
