use std::sync::mpsc::Sender;

use crate::{dsp::schema, patch::Patch, types::ModuleSchema, types::ModuleState};

pub enum InputMessage {
    Echo(String),
    Schema,
    GetModules,
    GetModule(String),
}

pub enum OutputMessage {
    Echo(String),
    Schema(Vec<&'static ModuleSchema>),
    PatchState(Vec<ModuleState>),
    ModuleState(String, Option<ModuleState>),
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
    };
    Ok(())
}
