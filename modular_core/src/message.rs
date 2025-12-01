use crossbeam_channel::Sender;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

use std::collections::{HashMap, HashSet};

use crate::{
    dsp::get_constructors,
    dsp::schema,
    patch::Patch,
    types::ModuleSchema,
    types::{InternalTrack, Keyframe, ModuleState, Param, PatchGraph, Track, TrackUpdate},
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
    
    // New declarative API - send complete desired state
    SetPatch { graph: PatchGraph },

    GetTracks,
    GetTrack { id: String },
    CreateTrack { id: String },
    UpdateTrack { id: String, update: TrackUpdate },
    DeleteTrack { id: String },
    UpsertKeyframe { keyframe: Keyframe },
    DeleteKeyframe { track_id: String, keyframe_id: String },
    
    // Audio streaming
    SubscribeAudio { module_id: String, port: String, buffer_size: usize },
    UnsubscribeAudio { subscription_id: String },
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
    
    // Audio streaming
    AudioSubscribed { subscription_id: String },
    AudioBuffer { subscription_id: String, samples: Vec<f32> },
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
        InputMessage::SetPatch { graph } => {
            apply_patch_diff(patch, graph, sender, sample_rate)?;
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
        InputMessage::SubscribeAudio { module_id, port, buffer_size } => {
            let subscription_id = uuid::Uuid::new_v4().to_string();
            let subscription = crate::patch::AudioSubscription {
                id: subscription_id.clone(),
                module_id,
                port,
                buffer_size,
            };
            
            patch
                .try_lock_for(Duration::from_millis(10))
                .unwrap()
                .audio_subscriptions
                .insert(subscription_id.clone(), subscription);
            
            sender.send(OutputMessage::AudioSubscribed { subscription_id })?;
        }
        InputMessage::UnsubscribeAudio { subscription_id } => {
            patch
                .try_lock_for(Duration::from_millis(10))
                .unwrap()
                .audio_subscriptions
                .remove(&subscription_id);
        }
    };
    Ok(())
}

fn apply_patch_diff(
    patch: &Arc<Mutex<Patch>>,
    desired_graph: PatchGraph,
    sender: &Sender<OutputMessage>,
    sample_rate: f32,
) -> anyhow::Result<()> {
    let mut patch_lock = patch.try_lock_for(Duration::from_millis(10)).unwrap();
    
    // Build maps for efficient lookup
    let desired_modules: HashMap<String, ModuleState> = desired_graph
        .modules
        .into_iter()
        .map(|m| (m.id.clone(), m))
        .collect();
    
    let current_ids: HashSet<String> = patch_lock.sampleables.keys().cloned().collect();
    let desired_ids: HashSet<String> = desired_modules.keys().cloned().collect();
    
    // Find modules to delete (in current but not in desired)
    let to_delete: Vec<String> = current_ids.difference(&desired_ids).cloned().collect();
    
    // Find modules to create (in desired but not in current)
    let to_create: Vec<String> = desired_ids.difference(&current_ids).cloned().collect();
    
    // Delete modules
    for id in to_delete {
        patch_lock.sampleables.remove(&id);
    }
    
    // Create new modules
    let constructors = get_constructors();
    for id in to_create {
        if let Some(desired_module) = desired_modules.get(&id) {
            if let Some(constructor) = constructors.get(&desired_module.module_type) {
                match constructor(&id, sample_rate) {
                    Ok(module) => {
                        patch_lock.sampleables.insert(id.clone(), module);
                    }
                    Err(err) => {
                        sender.send(OutputMessage::Error {
                            message: format!("Failed to create module {}: {}", id, err),
                        })?;
                    }
                }
            } else {
                sender.send(OutputMessage::Error {
                    message: format!("{} is not a valid module type", desired_module.module_type),
                })?;
            }
        }
    }
    
    // Update parameters for all desired modules (both new and existing)
    // We need to do this in two passes to handle cable connections properly:
    // Pass 1: Update all non-cable parameters
    // Pass 2: Update cable parameters (after all modules exist)
    
    for id in desired_ids.iter() {
        if let Some(desired_module) = desired_modules.get(id) {
            if let Some(module) = patch_lock.sampleables.get(id) {
                // Pass 1: Non-cable parameters
                for (param_name, param) in &desired_module.params {
                    if !matches!(param, Param::Cable { .. }) {
                        let internal_param = param.to_internal_param(&patch_lock);
                        if let Err(err) = module.update_param(param_name, &internal_param) {
                            sender.send(OutputMessage::Error {
                                message: format!("Failed to update param {}.{}: {}", id, param_name, err),
                            })?;
                        }
                    }
                }
            }
        }
    }
    
    // Pass 2: Cable parameters
    for id in desired_ids.iter() {
        if let Some(desired_module) = desired_modules.get(id) {
            if let Some(module) = patch_lock.sampleables.get(id) {
                for (param_name, param) in &desired_module.params {
                    if matches!(param, Param::Cable { .. }) {
                        let internal_param = param.to_internal_param(&patch_lock);
                        if let Err(err) = module.update_param(param_name, &internal_param) {
                            sender.send(OutputMessage::Error {
                                message: format!("Failed to update param {}.{}: {}", id, param_name, err),
                            })?;
                        }
                    }
                }
            }
        }
    }
    
    // Send success response with current state
    sender.send(OutputMessage::PatchState {
        modules: patch_lock
            .sampleables
            .iter()
            .map(|(_, module)| module.get_state())
            .collect(),
    })?;
    
    Ok(())
}
