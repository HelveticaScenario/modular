use std::{sync::mpsc::Sender, vec};

use modular_core::{
    message::{InputMessage, OutputMessage},
    types::{ModuleState, Param},
};
use rosc::OscType::{Float as OscFloat, Int as OscInt, Nil as OscNil, String as OscStr};
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

// fn make_module_state_bndl(state: &ModuleState) -> OscPacket {
//   let base = format!("/module/{}", state.id);
//   let module_type = state.module_type.clone();
//   bndl(
//       [
//           vec![msg(&base, vec![OscStr(module_type)])],
//           state
//               .params
//               .iter()
//               .map(|(key, param)| {
//                   msg(
//                       &format!("{}/param/{}", &base, key),
//                       match param {
//                           modular_core::types::Param::Value { value } => {
//                               [OscStr("value".into()), OscFloat(*value)].into()
//                           }
//                           modular_core::types::Param::Note { value } => {
//                               [OscStr("note".into()), OscInt(*value as i32)].into()
//                           }
//                           modular_core::types::Param::Cable { module, port } => [
//                               OscStr("cable".into()),
//                               OscStr(module.clone()),
//                               OscStr(port.clone()),
//                           ]
//                           .into(),
//                           modular_core::types::Param::Disconnected => [OscNil].into(),
//                       },
//                   )
//               })
//               .collect(),
//       ]
//       .concat(),
//   )
// }

pub fn message_to_osc(message: InputMessage) -> Vec<OscPacket> {
    match message {
        // OutputMessage::Echo(s) => vec![msg("/echo", vec![OscStr(s)])],
        // OutputMessage::Schema(schemas) => schemas
        //     .iter()
        //     .map(|schema| {
        //         let route = format!("/schema/{}", schema.name);
        //         let description = vec![msg(&route, vec![OscStr(schema.description.to_owned())])];
        //         let params = schema
        //             .params
        //             .iter()
        //             .map(|param| {
        //                 msg(
        //                     &format!("{}/param/{}", route, param.name),
        //                     vec![OscStr(param.description.to_owned())],
        //                 )
        //             })
        //             .collect();
        //         let outputs = schema
        //             .outputs
        //             .iter()
        //             .map(|output| {
        //                 msg(
        //                     &format!("{}/output/{}", route, output.name),
        //                     vec![OscStr(output.description.to_owned())],
        //                 )
        //             })
        //             .collect();
        //         bndl(vec![description, params, outputs].concat())
        //     })
        //     .collect(),
        // OutputMessage::ModuleState(id, state) => {
        //     if let Some(ref state) = state {
        //         vec![make_module_state_bndl(state)]
        //     } else {
        //         vec![msg(&format!("/module/{}", id), vec![OscNil])]
        //     }
        // }
        // OutputMessage::PatchState(state) => state
        //     .iter()
        //     .map(|module| make_module_state_bndl(module))
        //     .collect(),
        // OutputMessage::CreateModule(module_type, id) => {
        //     vec![msg("/create-module", vec![OscStr(module_type), OscStr(id)])]
        // }
        // OutputMessage::Error(err) => {
        //     vec![msg("/error", vec![OscStr(err)])]
        // }
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
        InputMessage::CreateModule(module_type) => {
            vec![msg("/create-module", vec![OscStr(module_type)])]
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
                    vec![OscStr("cable".to_owned()), OscStr(module), OscStr(port)]
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
    }
}

fn send(message: Message, tx: &Sender<Message>) {
    if let Err(e) = tx.send(message) {
        println!("Error receiving from socket: {}", e);
    }
}

pub enum SchemaMessage {
    Description(String, String),
    Param(String, String, String),
    Output(String, String, String),
}

pub enum ModuleMessage {
    Type(String, String),
    Param(String, String, Param),
}

pub enum PartialMessage {
    Schema(SchemaMessage),
    Module(ModuleMessage),
}

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
                        Message::Server(OutputMessage::CreateModule(module_type.clone(), id.clone())),
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
                println!("{:?}", s);
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
                            match (param_type.as_str(), args.1, args.2) {
                                ("value", Some(OscFloat(value)), None) => send(
                                    Message::Client(PartialMessage::Module(ModuleMessage::Param(
                                        (*id).to_owned(),
                                        (*param_name).to_owned(),
                                        Param::Value { value: *value },
                                    ))),
                                    tx,
                                ),
                                ("note", Some(OscInt(value)), None) => send(
                                    Message::Client(PartialMessage::Module(ModuleMessage::Param(
                                        (*id).to_owned(),
                                        (*param_name).to_owned(),
                                        Param::Note {
                                            value: *value as u8,
                                        },
                                    ))),
                                    tx,
                                ),
                                ("cable", Some(OscStr(module)), Some(OscStr(port))) => send(
                                    Message::Client(PartialMessage::Module(ModuleMessage::Param(
                                        (*id).to_owned(),
                                        (*param_name).to_owned(),
                                        Param::Cable { module: module.clone(), port: port.clone() },
                                    ))),
                                    tx,
                                ),
                                ("disconnected", None, None) => send(
                                    Message::Client(PartialMessage::Module(ModuleMessage::Param(
                                        (*id).to_owned(),
                                        (*param_name).to_owned(),
                                        Param::Disconnected,
                                    ))),
                                    tx,
                                ),
                                _ => {}
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
