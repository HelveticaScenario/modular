pub use modular_core::crossbeam_channel;
use modular_core::crossbeam_channel::unbounded;
use std::sync::Arc;
use tokio::sync::{Mutex as TokioMutex, broadcast};

use modular_core::Modular;

mod http_server;
pub mod persistence;
pub mod protocol;
pub mod validation;

pub use http_server::{AppState, create_router, forward_output_messages};
pub use protocol::{InputMessage, OutputMessage};

pub fn create_app_state() -> (
    AppState,
    modular_core::crossbeam_channel::Receiver<modular_core::message::InputMessage>,
    modular_core::crossbeam_channel::Sender<modular_core::message::OutputMessage>,
    modular_core::crossbeam_channel::Receiver<modular_core::message::OutputMessage>,
) {
    let (incoming_tx, incoming_rx) = unbounded();
    let (outgoing_tx, outgoing_rx) = unbounded();

    let (broadcast_tx, _) = broadcast::channel(100);

    let state = AppState {
        input_tx: Arc::new(TokioMutex::new(incoming_tx)),
        broadcast_tx,
    };

    (state, incoming_rx, outgoing_tx, outgoing_rx)
}

pub async fn run_server(port: u16) -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create app state and channels
    let (state, incoming_rx, outgoing_tx, outgoing_rx) = create_app_state();

    // Spawn modular core thread
    let _modular_handle = Modular::spawn(incoming_rx, outgoing_tx);

    // Spawn task to forward output messages to broadcast
    let output_rx = Arc::new(TokioMutex::new(outgoing_rx));
    let broadcast_tx = state.broadcast_tx.clone();
    tokio::spawn(async move {
        forward_output_messages(output_rx, broadcast_tx).await;
    });

    // Create router
    let app = create_router(state);

    // Start server
    let addr = format!("127.0.0.1:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("HTTP server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
