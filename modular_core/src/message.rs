use std::sync::mpsc::Sender;

use crate::patch::Patch;

pub enum Message {}

pub fn handle_message(
    message: Message,
    patch: &mut Patch,
    sender: &Sender<Message>,
) -> anyhow::Result<()> {
    let patch_map = patch.map.clone();
    match message {};
    Ok(())
}
