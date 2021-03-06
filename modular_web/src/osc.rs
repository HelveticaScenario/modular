use std::vec;
use modular_core::crossbeam_channel::Sender;
use modular_core::{
    message::{InputMessage, OutputMessage},
    types::Param,
    uuid::Uuid,
};
use modular_server::rosc::OscType::{Float as OscFloat, Int as OscInt, String as OscStr};
use modular_server::rosc::{OscMessage, OscPacket, OscType};

fn msg(addr: &str, args: Vec<OscType>) -> OscPacket {
    OscPacket::Message(OscMessage {
        addr: addr.to_owned(),
        args,
    })
}

pub fn message_to_osc(message: InputMessage) -> Vec<OscPacket> {
    match message {
        InputMessage::Echo(s) => {
            vec![msg("/echo", vec![OscStr(s)])]
        }
        InputMessage::Schema => {
            vec![msg("/schema", vec![])]
        }
        InputMessage::GetModules => {
            vec![msg("/modules", vec![])]
        }
        InputMessage::GetModule(id) => {
            vec![msg(&format!("/module/{}", id), vec![])]
        }
        InputMessage::CreateModule(module_type, id) => {
            vec![msg(
                "/create-module",
                vec![OscStr(module_type), OscStr(id.to_string())],
            )]
        }
        InputMessage::UpdateParam(id, param_name, new_param) => {
            let args = match new_param {
                Param::Value { value } => {
                    vec![OscStr("value".to_owned()), OscFloat(value)]
                }
                Param::Note { value } => {
                    vec![OscStr("note".to_owned()), OscInt(value as i32)]
                }
                Param::Cable { module, port } => {
                    vec![
                        OscStr("cable".to_owned()),
                        OscStr(module.to_string()),
                        OscStr(port),
                    ]
                }
                Param::Track { track } => {
                    vec![OscStr("track".to_owned()), OscStr(track.to_string())]
                }
                Param::Disconnected => {
                    vec![OscStr("disconnected".to_owned())]
                }
            };
            vec![msg(
                &format!("/update-module/{}/param/{}", id, param_name),
                args,
            )]
        }
        InputMessage::DeleteModule(id) => {
            vec![msg("/delete-module", vec![OscStr(id.to_string())])]
        }
        InputMessage::GetTracks => {
            todo! {}
        }
        InputMessage::GetTrack(_) => {
            todo! {}
        }
        InputMessage::CreateTrack(_) => {
            todo! {}
        }
        InputMessage::UpdateTrack(_, _) => {
            todo! {}
        }
        InputMessage::DeleteTrack(_) => {
            todo! {}
        }
        InputMessage::UpsertKeyframe(_) => {
            todo! {}
        }
        InputMessage::DeleteKeyframe(_, _) => {
            todo! {}
        }
    }
}

fn send(message: Message, tx: &Sender<Message>) {
    if let Err(e) = tx.send(message) {
        println!("Error receiving from socket: {}", e);
    }
}

#[derive(Debug, Clone)]
pub enum SchemaMessage {
    Description(String, String),
    Param(String, String, String),
    Output(String, String, String),
}

#[derive(Debug, Clone)]
pub enum ModuleMessage {
    Type(String, String),
    Param(String, String, Param),
}

#[derive(Debug, Clone)]
pub enum PartialMessage {
    Schema(SchemaMessage),
    Module(ModuleMessage),
}

#[derive(Debug, Clone)]
pub enum Message {
    Server(OutputMessage),
    Client(PartialMessage),
}

pub fn osc_to_message(packet: OscPacket, tx: &Sender<Message>) {
    match packet {
        OscPacket::Message(message) => match message.addr.as_str() {
            "/echo" => {
                if let Some(OscStr(s)) = message.args.get(0) {
                    send(Message::Server(OutputMessage::Echo(s.clone())), tx);
                }
            }
            "/error" => {
                if let Some(OscStr(err)) = message.args.get(0) {
                    send(Message::Server(OutputMessage::Error(err.clone())), tx);
                }
            }
            "/create-module" => {
                if let (Some(OscStr(module_type)), Some(OscStr(id))) =
                    (message.args.get(0), message.args.get(1))
                {
                    send(
                        Message::Server(OutputMessage::CreateModule(
                            module_type.clone(),
                            match Uuid::parse_str(id) {
                                Ok(id) => id,
                                Err(err) => {
                                    println!("{}", err);
                                    return;
                                }
                            },
                        )),
                        tx,
                    );
                }
            }
            // "/schema" => send(InputMessage::Schema, tx),
            // "/modules" => send(InputMessage::GetModules, tx),
            // addr => {
            //     let s: Vec<&str> = addr.split("/").filter(|s| *s != "").collect();
            //     println!("{:?}", s);
            //     let addr = (s.get(0), s.get(1), s.get(2), s.get(3), s.get(4));
            //     let args = (
            //         message.args.get(0),
            //         message.args.get(1),
            //         message.args.get(2),
            //         message.args.get(3),
            //     );
            //     if let (Some(&"module"), Some(id), None) = (addr.0, addr.1, addr.2) {
            //         send(InputMessage::GetModule(String::from(*id)), tx);
            //     } else if let (Some(&"create-module"), None, Some(OscStr(ref module_type)), None) =
            //         (addr.0, addr.1, args.0, args.1)
            //     {
            //         send(InputMessage::CreateModule(module_type.clone()), tx);
            //     } else if let (
            //         Some(&"update-module"),
            //         Some(id),
            //         Some(&"param"),
            //         Some(param),
            //         None,
            //         Some(OscStr(param_type)),
            //     ) = (addr.0, addr.1, addr.2, addr.3, addr.4, args.0)
            //     {
            //         send(
            //             InputMessage::UpdateParam(
            //                 String::from(*id),
            //                 String::from(*param),
            //                 match (param_type.as_str(), args.1, args.2, args.3) {
            //                     ("value", Some(OscFloat(value)), None, None) => {
            //                         Param::Value { value: *value }
            //                     }
            //                     ("cable", Some(OscStr(module)), Some(OscStr(port)), None) => {
            //                         Param::Cable {
            //                             module: module.clone(),
            //                             port: port.clone(),
            //                         }
            //                     }
            //                     ("note", Some(OscInt(note)), None, None) => Param::Note {
            //                         value: note.clone() as u8,
            //                     },
            //                     ("disconnected", None, None, None) => Param::Disconnected,
            //                     (param_type, _, _, _) => {
            //                         println!("param type not value: {}", param_type);
            //                         return;
            //                     }
            //                 },
            //             ),
            //             tx,
            //         );
            //     }
            // }
            addr => {
                let s: Vec<&str> = addr.split("/").filter(|s| *s != "").collect();
                let addr = (s.get(0), s.get(1), s.get(2), s.get(3), s.get(4));
                let args = (
                    message.args.get(0),
                    message.args.get(1),
                    message.args.get(2),
                    message.args.get(3),
                );
                if let (Some(&"schema"), Some(name)) = (addr.0, addr.1) {
                    match (addr.2, addr.3, args.0) {
                        (None, None, Some(OscStr(description))) => send(
                            Message::Client(PartialMessage::Schema(SchemaMessage::Description(
                                (*name).to_owned(),
                                description.clone(),
                            ))),
                            tx,
                        ),
                        (Some(&"param"), Some(param_name), Some(OscStr(description))) => send(
                            Message::Client(PartialMessage::Schema(SchemaMessage::Param(
                                (*name).to_owned(),
                                (*param_name).to_owned(),
                                description.clone(),
                            ))),
                            tx,
                        ),
                        (Some(&"output"), Some(output_name), Some(OscStr(description))) => send(
                            Message::Client(PartialMessage::Schema(SchemaMessage::Output(
                                (*name).to_owned(),
                                (*output_name).to_owned(),
                                description.clone(),
                            ))),
                            tx,
                        ),
                        _ => {}
                    }
                } else if let (Some(&"module"), Some(id)) = (addr.0, addr.1) {
                    match (addr.2, addr.3, args.0) {
                        (None, None, Some(OscStr(module_type))) => send(
                            Message::Client(PartialMessage::Module(ModuleMessage::Type(
                                (*id).to_owned(),
                                module_type.clone(),
                            ))),
                            tx,
                        ),
                        (Some(&"param"), Some(param_name), Some(OscStr(param_type))) => {
                            let param = match (param_type.as_str(), args.1, args.2) {
                                ("value", Some(OscFloat(value)), None) => {
                                    Some(Param::Value { value: *value })
                                }
                                ("note", Some(OscInt(value)), None) => Some(Param::Note {
                                    value: *value as u8,
                                }),
                                ("cable", Some(OscStr(module)), Some(OscStr(port))) => {
                                    Some(Param::Cable {
                                        module: match Uuid::parse_str(module) {
                                            Ok(id) => id,
                                            Err(err) => {
                                                println!("{}", err);
                                                return;
                                            }
                                        },
                                        port: port.clone(),
                                    })
                                }
                                ("disconnected", None, None) => Some(Param::Disconnected),
                                _ => None,
                            };
                            match param {
                                Some(param) => send(
                                    Message::Client(PartialMessage::Module(ModuleMessage::Param(
                                        (*id).to_owned(),
                                        (*param_name).to_owned(),
                                        param,
                                    ))),
                                    tx,
                                ),
                                None => {}
                            }
                        }

                        _ => {}
                    }
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
