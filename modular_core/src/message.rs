use std::sync::mpsc::Sender;

use crate::{dsp::schema, patch::Patch, types::ModuleSchema};

pub enum InputMessage {
    Echo(String),
    Schema,
    GetPatch,
}

pub enum OutputMessage {
    Echo(String),
    Schema(Vec<&'static ModuleSchema>),
    ModuleState(),
}

pub fn handle_message(
    message: InputMessage,
    patch: &mut Patch,
    sender: &Sender<OutputMessage>,
) -> anyhow::Result<()> {
    let patch_map = patch.map.clone();
    let res = match message {
        InputMessage::Echo(s) => sender.send(OutputMessage::Echo(format!("{}!", s))),
        InputMessage::Schema => sender.send(OutputMessage::Schema(schema())),
        InputMessage::GetPatch => Ok(()),
    };
    println!("{:?}", res);
    Ok(())
}
