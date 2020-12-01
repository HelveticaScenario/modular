use std::sync::mpsc::Sender;

use crate::{patch::Patch, types::ModuleSchema, dsp::schema};

pub enum InputMessage {
    Echo(String),
    Schema,
}

pub enum OutputMessage {
    Echo(String),
    Schema(Vec<&'static ModuleSchema>)
}

pub fn handle_message(
    message: InputMessage,
    patch: &mut Patch,
    sender: &Sender<OutputMessage>,
) -> anyhow::Result<()> {
    let patch_map = patch.map.clone();
    println!(
        "{:?}",
        match message {
            InputMessage::Echo(s) => sender.send(OutputMessage::Echo(format!("{}!", s))),
            InputMessage::Schema => sender.send(OutputMessage::Schema(schema()))
        }
    );
    Ok(())
}
