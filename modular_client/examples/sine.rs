// use std::{sync::mpsc, thread, time::Duration};

// use anyhow::anyhow;
// use modular_client::{client::spawn_client, osc::Message::Server};
// use modular_core::{
//     message::{InputMessage, OutputMessage},
//     types::Param,
//     uuid::Uuid,
// };
// use modular_server::spawn;

// fn main() -> anyhow::Result<()> {
//     // let matches = get_matches();

//     let (_modular_handle, _receiving_server_handle, _sending_server_handle) =
//         spawn("127.0.0.1:7813".to_owned(), "7812".to_owned());

//     let (incoming_tx, incoming_rx) = mpsc::channel();
//     let (outgoing_tx, outgoing_rx) = mpsc::channel();

//     let (_receiving_client_handle, _sending_client_handle) = spawn_client(
//         "127.0.0.1:7812".to_owned(),
//         "7813".to_owned(),
//         incoming_tx,
//         outgoing_rx,
//     );
//     outgoing_tx.send(InputMessage::CreateModule("sine-oscillator".into(), None))?;
//     let osc_id = match incoming_rx.recv()? {
//         Server(OutputMessage::CreateModule(module_type, id)) => {
//             if module_type == "sine-oscillator" {
//                 Ok(id)
//             } else {
//                 Err(anyhow!("something happened"))
//             }
//         }
//         _ => Err(anyhow!("something happened")),
//     }?;
//     outgoing_tx.send(InputMessage::CreateModule("scale-and-shift".into(), None))?;
//     let atten_id = match incoming_rx.recv()? {
//         Server(OutputMessage::CreateModule(module_type, id)) => {
//             if module_type == "scale-and-shift" {
//                 Ok(id)
//             } else {
//                 Err(anyhow!("something happened"))
//             }
//         }
//         _ => Err(anyhow!("something happened")),
//     }?;
//     outgoing_tx.send(InputMessage::UpdateParam(
//         osc_id.clone(),
//         "freq".into(),
//         Param::Note { value: 69 },
//     ))?;

//     outgoing_tx.send(InputMessage::UpdateParam(
//         atten_id.clone(),
//         "input".into(),
//         Param::Cable {
//             module: osc_id.clone(),
//             port: "output".into(),
//         },
//     ))?;

//     outgoing_tx.send(InputMessage::UpdateParam(
//         atten_id.clone(),
//         "scale".into(),
//         Param::Value { value: 5.0 },
//     ))?;

//     outgoing_tx.send(InputMessage::UpdateParam(
//         Uuid::nil(),
//         "source".into(),
//         Param::Cable {
//             module: atten_id.clone(),
//             port: "output".into(),
//         },
//     ))?;
//     // let dur = Duration::from_millis(1000);
//     const A: u8 = 69;
//     const B: u8 = 67;
//     const C: u8 = 65;
//     let part1 = [A, B, C];
//     for _ in 0..2 {
//         for i in part1.iter() {
//             outgoing_tx.send(InputMessage::UpdateParam(
//                 osc_id.clone(),
//                 "freq".into(),
//                 Param::Note { value: *i },
//             ))?;
//             thread::sleep(Duration::from_millis(500));
//         }
//         thread::sleep(Duration::from_millis(500));
//     }
//     let part2 = [C, C, C, C, B, B, B, B];
//     for i in part2.iter() {
//         outgoing_tx.send(InputMessage::UpdateParam(
//             osc_id.clone(),
//             "freq".into(),
//             Param::Note { value: *i },
//         ))?;

//         thread::sleep(Duration::from_millis(100));
//         outgoing_tx.send(InputMessage::UpdateParam(
//             atten_id.clone(),
//             "scale".into(),
//             Param::Value { value: 0.0 },
//         ))?;
//         thread::sleep(Duration::from_millis(100));
//         outgoing_tx.send(InputMessage::UpdateParam(
//             atten_id.clone(),
//             "scale".into(),
//             Param::Value { value: 5.0 },
//         ))?;
//     }
//     for i in part1.iter() {
//         outgoing_tx.send(InputMessage::UpdateParam(
//             osc_id.clone(),
//             "freq".into(),
//             Param::Note { value: *i },
//         ))?;
//         thread::sleep(Duration::from_millis(500));
//     }
//     thread::sleep(Duration::from_millis(500));
//     // for _ in 0..10 {
//     //     for i in 0..12 {
//     //         outgoing_tx.send(InputMessage::UpdateParam(
//     //             id.clone(),
//     //             "freq".into(),
//     //             Param::Note { value: 69+i },
//     //         ))?;
//     //         thread::sleep(dur);
//     //     }
//     // }
//     // let r = running.clone();
//     // ctrlc::set_handler(move || {
//     //     r.store(false, Ordering::SeqCst);
//     // })
//     // .expect("Error setting Ctrl-C handler");

//     // while running.load(Ordering::SeqCst) {}
//     Ok(())
// }
