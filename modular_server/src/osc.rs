use std::{sync::mpsc::Sender, vec};

use modular_core::message::{InputMessage, OutputMessage};
use rosc::{OscBundle, OscMessage, OscPacket, OscType};

fn bndl(content: Vec<OscPacket>) -> OscPacket {
    OscPacket::Bundle(OscBundle {
        content,
        timetag: (0, 1),
    })
}

fn msg(addr: &str, args: Vec<OscType>) -> OscPacket {
    OscPacket::Message(OscMessage {
        addr: addr.to_owned(),
        args,
    })
}

pub fn message_to_osc(message: OutputMessage) -> Vec<OscPacket> {
    match message {
        OutputMessage::Echo(s) => vec![msg("/echo", vec![OscType::String(s)])],
        OutputMessage::Schema(schemas) => schemas
            .iter()
            .map(|schema| {
                let route = format!("/schema/{}", schema.name);
                let description = vec![msg(
                    &route,
                    vec![OscType::String(schema.description.to_owned())],
                )];
                let params = schema
                    .params
                    .iter()
                    .map(|param| {
                        msg(
                            &format!("{}/param/{}", route, param.name),
                            vec![
                                OscType::String(param.description.to_owned()),
                                OscType::Bool(param.required),
                            ],
                        )
                    })
                    .collect();
                let outputs = schema
                    .outputs
                    .iter()
                    .map(|output| {
                        msg(
                            &format!("{}/output/{}", route, output.name),
                            vec![OscType::String(output.description.to_owned())],
                        )
                    })
                    .collect();
                bndl(vec![description, params, outputs].concat())
            })
            .collect(),
        OutputMessage::ModuleState() => {
            vec![]
        }
    }
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
