use std::{thread::JoinHandle, sync::mpsc};

use modular_core::{Modular};
use server::spawn_server;

mod osc;
mod server;

pub fn spawn(
    client_address: String,
    port: String,
) -> (JoinHandle<anyhow::Result<()>>, JoinHandle<()>, JoinHandle<()>) {
    let (incoming_tx, incoming_rx) = mpsc::channel();
    let (outgoing_tx, outgoing_rx) = mpsc::channel();

    let modular = Modular::new();
    let _modular_handle = modular.spawn(incoming_rx, outgoing_tx);

    let (_receiving_server_handle, _sending_server_handle) = spawn_server(
        client_address.to_owned(),
        port.to_owned(),
        incoming_tx,
        outgoing_rx,
    );
    (_modular_handle, _receiving_server_handle, _sending_server_handle)
}
