use crossbeam_channel::unbounded;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::broadcast;

use modular_core::dsp::get_constructors;
use modular_core::Patch;

pub mod audio;
mod http_server;
pub mod persistence;
pub mod protocol;
pub mod validation;

pub use audio::{AudioState, AudioSubscription};
pub use http_server::{AppState, create_router, forward_output_messages};
pub use protocol::{InputMessage, OutputMessage, ValidationError};

/// Server configuration
pub struct ServerConfig {
    pub port: u16,
    pub patch_file: Option<PathBuf>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 7812,
            patch_file: None,
        }
    }
}

/// Create shared state for the server
pub fn create_server_state(sample_rate: f32) -> (Arc<AudioState>, crossbeam_channel::Sender<OutputMessage>, crossbeam_channel::Receiver<OutputMessage>) {
    let (output_tx, output_rx) = unbounded();
    
    // Create patch with root module
    let mut sampleables = HashMap::new();
    let constructors = get_constructors();
    if let Some(constructor) = constructors.get("signal") {
        if let Ok(module) = constructor(&"root".to_string(), sample_rate) {
            sampleables.insert("root".to_string(), module);
        }
    }
    
    let patch = Arc::new(Mutex::new(Patch::new(sampleables, HashMap::new())));
    let audio_state = Arc::new(AudioState::new(patch, sample_rate));
    
    (audio_state, output_tx, output_rx)
}

pub async fn run_server(config: ServerConfig) -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Get sample rate from audio device
    let sample_rate = audio::get_sample_rate()?;
    tracing::info!("Audio sample rate: {} Hz", sample_rate);
    
    // Create server state
    let (audio_state, output_tx, output_rx) = create_server_state(sample_rate);
    
    // Start audio thread
    let audio_state_clone = Arc::clone(&audio_state);
    let output_tx_clone = output_tx.clone();
    let _stream = audio::run_audio_thread(audio_state_clone, output_tx_clone)?;
    
    // Create broadcast channel for WebSocket clients
    let (broadcast_tx, _) = broadcast::channel(100);
    
    // Create app state
    let state = AppState {
        audio_state,
        broadcast_tx: broadcast_tx.clone(),
        sample_rate,
    };
    
    // Spawn task to forward output messages to broadcast
    let output_rx_clone = output_rx;
    tokio::spawn(async move {
        forward_output_messages(output_rx_clone, broadcast_tx).await;
    });

    // Create router
    let app = create_router(state);

    // Start server
    let addr = format!("127.0.0.1:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("HTTP server listening on {}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
