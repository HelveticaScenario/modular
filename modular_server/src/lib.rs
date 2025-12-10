use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use modular_core::Patch;
use modular_core::dsp::get_constructors;

pub mod audio;
mod http_server;
pub mod persistence;
pub mod protocol;
pub mod validation;

pub use audio::AudioState;
pub use http_server::{AppState, create_router};
pub use protocol::{InputMessage, OutputMessage, ValidationError};

use crate::audio::send_audio_buffers;

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
pub fn create_server_state(sample_rate: f32) -> Arc<AudioState> {
    // Create patch with root module
    let mut sampleables = HashMap::new();
    let constructors = get_constructors();
    if let Some(constructor) = constructors.get("signal") {
        if let Ok(module) = constructor("root".to_string(), sample_rate) {
            sampleables.insert("root".to_string(), module);
        }
    }

    let patch = Arc::new(tokio::sync::Mutex::new(Patch::new(
        sampleables,
        HashMap::new(),
    )));
    let audio_state = Arc::new(AudioState::new(patch, "".into(), sample_rate));

    audio_state
}

pub async fn run_server(config: ServerConfig) -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Get sample rate from audio device
    let sample_rate = audio::get_sample_rate()?;
    tracing::info!("Audio sample rate: {} Hz", sample_rate);

    // Create server state
    let audio_state = create_server_state(sample_rate);

    // Start audio thread
    let _stream = audio::run_audio_thread(audio_state.clone())?;

    // Create app state
    let state = AppState {
        audio_state: audio_state.clone(),
    };

    // Spawn task to send audio buffers periodically
    let audio_state_clone = audio_state.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_millis(16)).await;
            send_audio_buffers(&audio_state_clone.clone());
        }
    });

    // Create router
    let app = create_router(state);

    // Start server
    let addr = format!("localhost:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("HTTP server listening on http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    /// This test exports TypeScript types from Rust structs.
    /// Run with: cargo test export_types -- --ignored
    #[test]
    #[ignore]
    fn export_types() {
        use crate::protocol::*;
        use modular_core::types::*;
        use ts_rs::TS;

        // Core types
        Param::export_all().expect("Failed to export Param");
        Keyframe::export_all().expect("Failed to export Keyframe");
        InterpolationType::export_all().expect("Failed to export InterpolationType");
        Track::export_all().expect("Failed to export Track");
        ParamSchema::export_all().expect("Failed to export PortSchema");
        ModuleSchema::export_all().expect("Failed to export ModuleSchema");
        ModuleState::export_all().expect("Failed to export ModuleState");
        PatchGraph::export_all().expect("Failed to export PatchGraph");

        // Protocol types
        InputMessage::export_all().expect("Failed to export InputMessage");
        OutputMessage::export_all().expect("Failed to export OutputMessage");
        ValidationError::export_all().expect("Failed to export ValidationError");

        println!("TypeScript types exported successfully!");
    }
}
