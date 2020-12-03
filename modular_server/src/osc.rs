use std::{sync::mpsc::Sender, vec};

use modular_core::{
    message::{InputMessage, OutputMessage},
    types::ModuleState,
};
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

fn make_module_state_bndl(state: &ModuleState) -> OscPacket {
    let base = format!("/module/{}", state.id);
    let module_type = state.module_type.clone();
    bndl(
        [
            vec![msg(&base, vec![OscType::String(module_type)])],
            state
                .params
                .iter()
                .map(|(key, maybe_param)| {
                    msg(
                        &format!("{}/param/{}", &base, key),
                        match maybe_param {
                            Some(ref param) => match param {
                                modular_core::types::Param::Value { value } => {
                                    [OscType::String("value".into()), OscType::Float(*value)].into()
                                }
                                modular_core::types::Param::Note { value } => {
                                    [OscType::String("note".into()), OscType::Int(*value as i32)]
                                        .into()
                                }
                                modular_core::types::Param::Cable { module, port } => [
                                    OscType::String("cable".into()),
                                    OscType::String(module.clone()),
                                    OscType::String(port.clone()),
                                ]
                                .into(),
                            },
                            None => [OscType::Nil].into(),
                        },
                    )
                })
                .collect(),
        ]
        .concat(),
    )
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
        OutputMessage::ModuleState(id, state) => {
            if let Some(ref state) = state {
                vec![make_module_state_bndl(state)]
            } else {
                vec![msg(&format!("/module/{}", id), vec![OscType::Nil])]
            }
        }
        OutputMessage::PatchState(state) => state
            .iter()
            .map(|module| make_module_state_bndl(module))
            .collect(),
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
            "/modules" => send(InputMessage::GetModules, tx),
            addr => {
                let s: Vec<&str> = addr.split("/").filter(|s| *s != "").collect();
                println!("{:?}", s);
                if let (Some(&"module"), Some(id), None) = (s.get(0), s.get(1), s.get(2)) {
                    send(InputMessage::GetModule(String::from(*id)), tx);
                }
            }
        },
        OscPacket::Bundle(bundle) => {
            for p in bundle.content {
                osc_to_message(p, tx);
            }
        }
    }
}
