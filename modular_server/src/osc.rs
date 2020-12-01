use std::{sync::mpsc::Sender, vec};

use modular_core::message::{InputMessage, OutputMessage};
use rosc::{OscBundle, OscMessage, OscPacket, OscType};

pub fn message_to_osc(message: OutputMessage) -> OscPacket {
    match message {
        OutputMessage::Echo(s) => OscPacket::Message(OscMessage {
            addr: "/echo".to_owned(),
            args: vec![OscType::String(s)],
        }),
        OutputMessage::Schema(schema) => {
            OscPacket::Bundle(OscBundle {
                timetag: (0, 1), // immediately,
                content: vec![],
            })
        }
    }
    // return OscPacket::Message(OscMessage {
    //     addr: "/a".to_owned(),
    //     args: vec![],
    // });
}

fn send(message: InputMessage, tx: &Sender<InputMessage>) {
    if let Err(e) = tx.send(message) {
        println!("Error receiving from socket: {}", e);
    }
}

pub fn osc_to_message(packet: OscPacket, tx: &Sender<InputMessage>) {
    match packet {
        OscPacket::Message(message) => match message.addr.as_str() {
            "/echo" => {
                if let Some(OscType::String(s)) = message.args.get(0) {
                    send(InputMessage::Echo(s.clone()), tx);
                }
            }
            "/schema" => send(InputMessage::Schema, tx),
            _ => {}
        },
        OscPacket::Bundle(bundle) => {
            for p in bundle.content {
                osc_to_message(p, tx);
            }
        }
    }
}
