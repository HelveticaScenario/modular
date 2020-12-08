use std::{time::Duration, collections::HashMap, sync::mpsc, thread};

use anyhow::anyhow;
use modular_client::{client::spawn_client, osc::Message::Server};
use modular_core::{
    message::{InputMessage, OutputMessage},
    types::Param,
};
use modular_server::spawn;

fn main() -> anyhow::Result<()> {
    // let matches = get_matches();

    let (_modular_handle, _receiving_server_handle, _sending_server_handle) = spawn(
        "127.0.0.1:7813".to_owned(),
        "7812".to_owned(),
        HashMap::new(),
    )?;

    let (incoming_tx, incoming_rx) = mpsc::channel();
    let (outgoing_tx, outgoing_rx) = mpsc::channel();

    let (_receiving_client_handle, _sending_client_handle) = spawn_client(
        "127.0.0.1:7812".to_owned(),
        "7813".to_owned(),
        incoming_tx,
        outgoing_rx,
    );
    outgoing_tx.send(InputMessage::CreateModule("sine-oscillator".into()))?;
    let id = match incoming_rx.recv()? {
        Server(OutputMessage::CreateModule(module_type, id)) => {
            if module_type == "sine-oscillator" {
                Ok(id)
            } else {
                Err(anyhow!("something happened"))
            }
        }
        _ => Err(anyhow!("something happened")),
    }?;
    print!("{:?}", id);
    outgoing_tx.send(InputMessage::UpdateParam(
        id.clone(),
        "freq".into(),
        Param::Note { value: 69 },
    ))?;
    outgoing_tx.send(InputMessage::UpdateParam(
        "ROOT".into(),
        "source".into(),
        Param::Cable {
            module: id.clone(),
            port: "output".into(),
        },
    ))?;
    for _ in 0..3 {
        for i in 0..12 {
            outgoing_tx.send(InputMessage::UpdateParam(
                id.clone(),
                "freq".into(),
                Param::Note { value: 69+i },
            ))?;
            thread::sleep(Duration::from_millis(250));
        }
    }
    thread::sleep(Duration::from_secs(1));
    // let r = running.clone();
    // ctrlc::set_handler(move || {
    //     r.store(false, Ordering::SeqCst);
    // })
    // .expect("Error setting Ctrl-C handler");

    // while running.load(Ordering::SeqCst) {}
    Ok(())
}
