pub use modular_core::crossbeam_channel;
use modular_core::crossbeam_channel::unbounded;
use std::thread::JoinHandle;

use modular_core::Modular;
use server::spawn_server;

mod server;

pub fn spawn(
    client_address: String,
    port: String,
) -> (
    JoinHandle<anyhow::Result<()>>,
    JoinHandle<()>,
    JoinHandle<()>,
) {
    let (incoming_tx, incoming_rx) = unbounded();
    let (outgoing_tx, outgoing_rx) = unbounded();

    let _modular_handle = Modular::spawn(incoming_rx, outgoing_tx);

    let (_receiving_server_handle, _sending_server_handle) = spawn_server(
        client_address.to_owned(),
        port.to_owned(),
        incoming_tx,
        outgoing_rx,
    );
    (
        _modular_handle,
        _receiving_server_handle,
        _sending_server_handle,
    )
}
