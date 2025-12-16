use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use modular_core::Patch;
use modular_core::dsp::get_constructors;
use zeroconf::prelude::*;
use zeroconf::{MdnsService, ServiceType, TxtRecord};
use zeroconf_tokio::MdnsServiceAsync;

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
        if let Ok(module) = constructor(&"root".to_string(), sample_rate) {
            sampleables.insert("root".to_string(), module);
        }
    }
    if let Some(constructor) = constructors.get("clock") {
        if let Ok(module) = constructor(&"root_clock".to_string(), sample_rate) {
            sampleables.insert("root_clock".to_string(), module);
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

    // Advertise the HTTP/WebSocket service over mDNS/Bonjour
    let mut mdns_service = start_mdns_service(config.port).await?;

    // Start server
    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(
        "HTTP server listening on http://localhost:{} (also via http://modular.local:{})",
        config.port,
        config.port
    );

    axum::serve(listener, app).await?;

    // Best-effort shutdown of the advertised service on exit
    let _ = mdns_service.shutdown().await;

    Ok(())
}

async fn start_mdns_service(port: u16) -> anyhow::Result<MdnsServiceAsync> {
    let service_type = ServiceType::new("http", "tcp")?;
    let mut service = MdnsService::new(service_type, port);

    service.set_name("modular");
    service.set_host("modular.local");
    service.set_domain("local.");

    let mut txt_record = TxtRecord::new();
    txt_record.insert("path", "/")?;
    txt_record.insert("ws_path", "/ws")?;
    txt_record.insert("proto", "http+ws")?;
    service.set_txt_record(txt_record);

    let mut service = MdnsServiceAsync::new(service)?;
    let registration = service.start().await?;

    tracing::info!(
        "Published Bonjour service '{}' at {:?} on {}",
        registration.name(),
        registration.service_type(),
        registration.domain()
    );

    Ok(service)
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
        DataParamType::export_all().expect("Failed to export DataParamType");
        DataParamValue::export_all().expect("Failed to export DataParamValue");
        DataParamSchema::export_all().expect("Failed to export DataParamSchema");
        Keyframe::export_all().expect("Failed to export Keyframe");
        InterpolationType::export_all().expect("Failed to export InterpolationType");
        Track::export_all().expect("Failed to export Track");
        SignalParamSchema::export_all().expect("Failed to export SignalParamSchema");
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
