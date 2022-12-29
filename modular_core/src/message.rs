// use parking_lot::Mutex;
// use std::{sync::Arc, time::Duration};
// use uuid::Uuid;

// use crate::{
//     dsp::get_constructors,
//     dsp::schema,
//     patch::Patch,
//     types::ModuleSchema,
//     types::{InternalTrack, Keyframe, ModuleState, Param, Track, TrackUpdate},
//     Modular,
// };

// #[derive(Debug, Clone)]
// pub enum InputMessage {
//     Schema,
//     GetModules,
//     GetModule(Uuid),
//     CreateModule(String, Uuid),
//     UpdateParam(Uuid, String, Param),
//     DeleteModule(Uuid),

//     GetTracks,
//     GetTrack(Uuid),
//     CreateTrack(Uuid),
//     UpdateTrack(Uuid, TrackUpdate),
//     DeleteTrack(Uuid),
//     UpsertKeyframe(Keyframe),
//     DeleteKeyframe(Uuid, Uuid),
// }

// #[derive(Debug, Clone)]
// pub enum OutputMessage {
//     Schema(Vec<ModuleSchema>),
//     PatchState(Vec<ModuleState>),
//     ModuleState(Uuid, Option<ModuleState>),
//     Track(Track),
//     Tracks(Vec<Track>),
//     CreateModule(String, Uuid),
//     CreateTrack(Uuid),
//     Error(String),
//     Ok,
// }

// impl Modular {
//     pub fn handle_message(
//         message: InputMessage,
//         patch: &Arc<Mutex<Patch>>,
//         sample_rate: f32,
//     ) -> Result<OutputMessage, anyhow::Error> {
//         println!("{:?}", message);
//         let patch = patch.try_lock_for(Duration::from_millis(10)).unwrap();
//         match message {
//             InputMessage::Schema => Ok(OutputMessage::Schema(schema())),
//             InputMessage::GetModules => Ok(OutputMessage::PatchState(
//                 patch
//                     .sampleables
//                     .iter()
//                     .map(|(_key, val)| val.get_state())
//                     .collect(),
//             )),
//             InputMessage::GetModule(id) => {
//                 let state = patch.sampleables.get(&id).map(|module| module.get_state());
//                 Ok(OutputMessage::ModuleState(id, state))
//             }
//             InputMessage::CreateModule(module_type, id) => {
//                 let constructors = get_constructors();
//                 println!("sample rate {}", sample_rate);
//                 if let Some(constructor) = constructors.get(&module_type) {
//                     match constructor(&id, sample_rate) {
//                         Ok(module) => {
//                             println!("attempt write");
//                             patch
//                                 .try_lock_for(Duration::from_millis(10))
//                                 .unwrap()
//                                 .sampleables
//                                 .insert(id.clone(), module);
//                             println!("written");
//                             Ok(OutputMessage::CreateModule(module_type, id))
//                         }
//                         Err(err) => {
//                             println!("{}", err);
//                             Ok(OutputMessage::Error(format!("an error occured: {}", err)))
//                         }
//                     }
//                 } else {
//                     Ok(OutputMessage::Error(format!(
//                         "{} is not a valid module type",
//                         module_type
//                     )))
//                 }
//             }
//             InputMessage::UpdateParam(id, param_name, new_param) => {
//                 let patch = patch.try_lock_for(Duration::from_millis(10)).unwrap();
//                 match patch.sampleables.get(&id) {
//                     Some(module) => module
//                         .update_param(&param_name, &new_param.to_internal_param(&patch))
//                         .map(|_| OutputMessage::Ok),
//                     None => Ok(OutputMessage::Error(format!("{} not found", id))),
//                 }
//             }
//             InputMessage::DeleteModule(id) => {
//                 patch.sampleables.remove(&id);
//                 Ok(OutputMessage::Ok)
//             }
//             InputMessage::GetTracks => Ok(OutputMessage::Tracks(
//                 patch
//                     .tracks
//                     .iter()
//                     .map(|(_, internal_track)| track.to_track())
//                     .collect(),
//             )),
//             InputMessage::GetTrack(id) => {
//                 if let Some(ref internal_track) = patch.tracks.get(&id) {
//                     sender.send(OutputMessage::Track(internal_track.to_track()))?;
//                 }
//             }
//             InputMessage::CreateTrack(id) => {
//                 patch
//                     .tracks
//                     .insert(id.clone(), Arc::new(InternalTrack::new(id.clone())));
//                 sender.send(OutputMessage::CreateTrack(id))?
//             }
//             InputMessage::UpdateTrack(id, track_update) => {
//                 if let Some(ref internal_track) = patch.lock().tracks.get(&id) {
//                     internal_track.update(&track_update)
//                 }
//             }
//             InputMessage::DeleteTrack(id) => {
//                 patch.tracks.remove(&id);
//             }
//             InputMessage::UpsertKeyframe(keyframe) => {
//                 let internal_keyframe = keyframe
//                     .to_internal_keyframe(&patch.try_lock_for(Duration::from_millis(10)).unwrap());

//                 if let Some(ref track) = patch.tracks.get(&keyframe.track_id) {
//                     track.add_keyframe(internal_keyframe);
//                 }
//             }
//             InputMessage::DeleteKeyframe(id, track_id) => {
//                 if let Some(ref track) = patch.tracks.get(&track_id) {
//                     track.remove_keyframe(id);
//                 }
//             }
//         }
//     }
// }
