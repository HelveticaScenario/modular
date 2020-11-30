use std::vec;

use modular_core::message::Message;
use rosc::{OscMessage, OscPacket};

pub fn message_to_osc(message: Message) -> OscPacket {
    OscPacket::Message(OscMessage {
        addr: "/a".to_owned(),
        args: vec![],
    })
}

pub fn osc_to_message(packet: OscPacket) -> Option<Message> {
    None
}
