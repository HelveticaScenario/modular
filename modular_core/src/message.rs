use std::sync::mpsc::Sender;

use serde_json::{Map, Value};
use uuid::Uuid;

use crate::{
    dsp::get_constructors,
    dsp::schema,
    patch::Patch,
    types::ModuleSchema,
    types::{ModuleState, Param},
};

#[derive(Debug, Clone)]
pub enum InputMessage {
    Echo(String),
    Schema,
    GetModules,
    GetModule(String),
    CreateModule(String),
    UpdateParam(String, String, Param),
    DeleteModule(String)
}

#[derive(Debug, Clone)]
pub enum OutputMessage {
    Echo(String),
    Schema(Vec<&'static ModuleSchema>),
    PatchState(Vec<ModuleState>),
    ModuleState(String, Option<ModuleState>),
    CreateModule(String, String),
    Error(String),
}

pub fn handle_message(
    message: InputMessage,
    patch: &mut Patch,
    sender: &Sender<OutputMessage>,
) -> anyhow::Result<()> {
    let patch_map = patch.map.clone();
    match message {
        InputMessage::Echo(s) => sender.send(OutputMessage::Echo(format!("{}!", s)))?,
        InputMessage::Schema => sender.send(OutputMessage::Schema(schema()))?,
        InputMessage::GetModules => {
            sender.send(OutputMessage::PatchState(
                patch_map
                    .lock()
                    .unwrap()
                    .iter()
                    .map(|(_key, val)| val.get_state())
                    .collect(),
            ))?;
        }
        InputMessage::GetModule(id) => {
            let state = patch_map
                .lock()
                .unwrap()
                .get(&id)
                .map(|module| module.get_state());
            sender.send(OutputMessage::ModuleState(id, state))?;
        }
        InputMessage::CreateModule(module_type) => {
            let constructors = get_constructors();
            if let Some(constructor) = constructors.get(&module_type) {
                let uuid = Uuid::new_v4().to_string();
                match constructor(&uuid, Value::Object(Map::new())) {
                    Ok(module) => {
                        patch_map.lock().unwrap().insert(uuid.clone(), module);
                        sender.send(OutputMessage::CreateModule(module_type, uuid))?
                    }
                    Err(err) => {
                        sender.send(OutputMessage::Error(format!("an error occured: {}", err)))?;
                    }
                }
            } else {
                sender.send(OutputMessage::Error(format!(
                    "{} is not a valid module type",
                    module_type
                )))?;
            }
        }
        InputMessage::UpdateParam(id, param_name, new_param) => {
            match patch_map.lock().unwrap().get(&id) {
                Some(module) => module.update_param(&param_name, new_param)?,
                None => sender.send(OutputMessage::Error(format!("{} not found", id)))?,
            }
        }
        InputMessage::DeleteModule(id) => {
            patch_map.lock().unwrap().remove(&id);
        }
    };
    Ok(())
}
