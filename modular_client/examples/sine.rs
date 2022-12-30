use std::{sync::mpsc, thread, time::Duration};

use anyhow::anyhow;
use modular_client::{client::spawn_client, osc::Message::Server};
use modular_core::{
    message::{InputMessage, OutputMessage},
    types::Param,
    uuid::Uuid,
};
use modular_server::spawn;

fn main() -> anyhow::Result<()> {
    // let matches = get_matches();

    let (_modular_handle, _receiving_server_handle, _sending_server_handle) =
        spawn("127.0.0.1:7813".to_owned(), "7812".to_owned());

    let (incoming_tx, incoming_rx) = mpsc::channel();
    let (outgoing_tx, outgoing_rx) = mpsc::channel();

    let (_receiving_client_handle, _sending_client_handle) = spawn_client(
        "127.0.0.1:7812".to_owned(),
        "7813".to_owned(),
        incoming_tx,
        outgoing_rx,
    );
    let osc_id = create_mod("sine-oscillator", &outgoing_tx, &incoming_rx)?;
    let atten_id = create_mod("scale-and-shift", &outgoing_tx, &incoming_rx)?;
    let atten_id_2 = create_mod("scale-and-shift", &outgoing_tx, &incoming_rx)?;
    let sum_id_2 = create_mod("sum", &outgoing_tx, &incoming_rx)?;

    set_cable(atten_id_2, "input", osc_id, "output", &outgoing_tx)?;

    set_value(atten_id_2, "scale", 1.0, &outgoing_tx)?;

    set_cable(sum_id_2, "input-1", atten_id_2, "output", &outgoing_tx)?;

    set_note(sum_id_2, "input-2", 69, &outgoing_tx)?;

    set_cable(osc_id, "freq", sum_id_2, "output", &outgoing_tx)?;

    set_cable(atten_id, "input", osc_id, "output", &outgoing_tx)?;

    set_value(atten_id, "scale", 5.0, &outgoing_tx)?;

    set_cable(Uuid::nil(), "source", atten_id, "output", &outgoing_tx)?;

    const A: u8 = 69;
    const B: u8 = 67;
    const C: u8 = 65;
    let part1 = [A, B, C];
    for _ in 0..2 {
        for i in part1.iter() {
            set_note(sum_id_2, "input-2", *i, &outgoing_tx)?;
            thread::sleep(Duration::from_millis(500));
        }
        thread::sleep(Duration::from_millis(500));
    }
    let part2 = [C, C, C, C, B, B, B, B];
    for i in part2.iter() {
        set_note(sum_id_2, "input-2", *i, &outgoing_tx)?;

        thread::sleep(Duration::from_millis(100));
        set_value(atten_id, "scale", 4.0, &outgoing_tx)?;
        thread::sleep(Duration::from_millis(100));
        set_value(atten_id, "scale", 5.0, &outgoing_tx)?;
    }
    for i in part1.iter() {
        set_note(sum_id_2, "input-2", *i, &outgoing_tx)?;
        thread::sleep(Duration::from_millis(500));
    }
    thread::sleep(Duration::from_millis(500));
    // for _ in 0..10 {
    //     for i in 0..12 {
    //         outgoing_tx.send(InputMessage::UpdateParam(
    //             id.clone(),
    //             "freq".into(),
    //             Param::Note { value: 69+i },
    //         ))?;
    //         thread::sleep(dur);
    //     }
    // }
    // let r = running.clone();
    // ctrlc::set_handler(move || {
    //     r.store(false, Ordering::SeqCst);
    // })
    // .expect("Error setting Ctrl-C handler");

    // while running.load(Ordering::SeqCst) {}
    Ok(())
}

fn set_value(
    dest_mod: Uuid,
    dest_port: &str,
    value: f32,
    outgoing_tx: &mpsc::Sender<InputMessage>,
) -> Result<(), anyhow::Error> {
    outgoing_tx.send(InputMessage::UpdateParam(
        dest_mod.clone(),
        dest_port.into(),
        Param::Value { value },
    ))?;
    Ok(())
}

fn set_note(
    dest_mod: Uuid,
    dest_port: &str,
    value: u8,
    outgoing_tx: &mpsc::Sender<InputMessage>,
) -> Result<(), anyhow::Error> {
    outgoing_tx.send(InputMessage::UpdateParam(
        dest_mod.clone(),
        dest_port.into(),
        Param::Note { value },
    ))?;
    Ok(())
}

fn set_cable(
    dest_mod: Uuid,
    dest_port: &str,
    source_mod: Uuid,
    source_port: &str,
    outgoing_tx: &mpsc::Sender<InputMessage>,
) -> Result<(), anyhow::Error> {
    Ok(outgoing_tx.send(InputMessage::UpdateParam(
        dest_mod.clone(),
        dest_port.into(),
        Param::Cable {
            module: source_mod.clone(),
            port: source_port.into(),
        },
    ))?)
}

fn create_mod(
    mod_type: &str,
    outgoing_tx: &mpsc::Sender<InputMessage>,
    incoming_rx: &mpsc::Receiver<modular_client::osc::Message>,
) -> Result<Uuid, anyhow::Error> {
    let id = Uuid::new_v4();
    outgoing_tx.send(InputMessage::CreateModule(mod_type.into(), id))?;
    let abc = incoming_rx.recv();
    println!("{:?}", abc);
    let id = match abc? {
        Server(OutputMessage::CreateModule(module_type, id)) => {
            if module_type == mod_type && id == id {
                Ok(id)
            } else {
                Err(anyhow!("something happened"))
            }
        }
        _ => Err(anyhow!("something happened")),
    }?;
    Ok(id)
}
