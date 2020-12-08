use std::{
    io::Read,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
};

use modular_client::client::spawn_client;

fn main() -> anyhow::Result<()> {
    // let matches = get_matches();

    // let (incoming_tx, incoming_rx) = mpsc::channel();
    // let (outgoing_tx, outgoing_rx) = mpsc::channel();

    // let running = Arc::new(AtomicBool::new(true));
    // let client_address = matches.value_of(CLIENT_ARG).unwrap();
    // let port = matches.value_of(PORT_ARG).unwrap();

    // let (_receiving_client_handle, _sending_client_handle) = spawn_client(
    //     client_address.to_owned(),
    //     port.to_owned(),
    //     incoming_tx,
    //     outgoing_rx,
    // );
    // let r = running.clone();
    // ctrlc::set_handler(move || {
    //     r.store(false, Ordering::SeqCst);
    // })
    // .expect("Error setting Ctrl-C handler");

    // while running.load(Ordering::SeqCst) {}

    Ok(())
}
