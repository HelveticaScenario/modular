use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use crossbeam_channel::Receiver;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::broadcast;
use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tracing::{error, info};

use modular_core::dsp::{get_constructors, schema};
use modular_core::types::{InternalTrack, Param, PatchGraph};

use crate::audio::{AudioState, AudioSubscription};
use crate::protocol::{InputMessage, OutputMessage};
use crate::validation::validate_patch;

// Shared server state
#[derive(Clone)]
pub struct AppState {
    pub audio_state: Arc<AudioState>,
    pub broadcast_tx: broadcast::Sender<OutputMessage>,
    pub sample_rate: f32,
}

// Build the Axum router
pub fn create_router(state: AppState) -> Router {
    // Serve static files from the static directory
    // Falls back to index.html for SPA routing
    let static_dir = std::env::current_dir()
        .unwrap_or_default()
        .join("modular_server")
        .join("static");
    
    let serve_dir = ServeDir::new(&static_dir)
        .not_found_service(ServeFile::new(static_dir.join("index.html")));

    Router::new()
        // WebSocket endpoint
        .route("/ws", get(ws_handler))
        // Health check
        .route("/health", get(health_check))
        // Static files (must be last as it's a fallback)
        .fallback_service(serve_dir)
        .layer(CorsLayer::permissive())
        .with_state(state)
}

// Health check endpoint
async fn health_check() -> &'static str {
    "OK"
}

// WebSocket handler
async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, state: AppState) {
    use futures_util::{SinkExt, StreamExt};
    
    // Subscribe to broadcast channel for server-initiated messages
    let mut broadcast_rx = state.broadcast_tx.subscribe();
    
    // Spawn task to forward broadcast messages to this client
    let (mut sender, mut receiver) = socket.split();
    
    let mut send_task = tokio::spawn(async move {
        while let Ok(msg) = broadcast_rx.recv().await {
            // Check if this is an audio buffer message - send as binary
            let message = if let OutputMessage::AudioBuffer { subscription_id, samples } = &msg {
                // Convert to binary: subscription_id as null-terminated string + f32 samples
                let mut bytes = subscription_id.as_bytes().to_vec();
                bytes.push(0); // null terminator
                for sample in samples {
                    bytes.extend_from_slice(&sample.to_le_bytes());
                }
                Message::Binary(bytes)
            } else {
                // YAML message
                let yaml = match serde_yaml::to_string(&msg) {
                    Ok(y) => y,
                    Err(e) => {
                        error!("Failed to serialize message to YAML: {}", e);
                        continue;
                    }
                };
                Message::Text(yaml)
            };
            
            if sender.send(message).await.is_err() {
                break;
            }
        }
    });
    
    // Handle incoming messages from client
    let state_clone = state.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                handle_client_message(&text, &state_clone).await;
            }
        }
    });
    
    // Wait for either task to complete (disconnect)
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    }
    
    info!("WebSocket connection closed");
}

// Handle a message from a WebSocket client
async fn handle_client_message(text: &str, state: &AppState) {
    // Try to parse as YAML first, then fall back to JSON for backward compatibility
    let message: InputMessage = match serde_yaml::from_str(text) {
        Ok(m) => m,
        Err(_) => match serde_json::from_str(text) {
            Ok(m) => m,
            Err(e) => {
                error!("Failed to parse message: {}", e);
                let _ = state.broadcast_tx.send(OutputMessage::Error {
                    message: format!("Failed to parse message: {}", e),
                    errors: None,
                });
                return;
            }
        }
    };
    
    tracing::debug!("{:?}", message);
    
    match message {
        InputMessage::Echo { message } => {
            let _ = state.broadcast_tx.send(OutputMessage::Echo {
                message: format!("{}!", message),
            });
        }
        
        InputMessage::GetSchemas => {
            let _ = state.broadcast_tx.send(OutputMessage::Schemas {
                schemas: schema(),
            });
        }
        
        InputMessage::GetPatch => {
            let patch = state.audio_state.patch.lock();
            let modules = patch.get_state();
            let _ = state.broadcast_tx.send(OutputMessage::PatchState {
                patch: PatchGraph { modules },
            });
        }
        
        InputMessage::SetPatch { patch } => {
            // Validate patch
            let schemas = schema();
            if let Err(errors) = validate_patch(&patch, &schemas) {
                let _ = state.broadcast_tx.send(OutputMessage::Error {
                    message: "Validation failed".to_string(),
                    errors: Some(errors),
                });
                return;
            }
            
            // Apply patch
            if let Err(e) = apply_patch(&state.audio_state, &patch, state.sample_rate) {
                let _ = state.broadcast_tx.send(OutputMessage::Error {
                    message: format!("Failed to apply patch: {}", e),
                    errors: None,
                });
                return;
            }
            
            // Auto-unmute on SetPatch - convenient for live editing workflows
            // where you typically want to hear changes immediately
            state.audio_state.set_muted(false);
            
            // Send updated state
            let patch_lock = state.audio_state.patch.lock();
            let modules = patch_lock.get_state();
            let _ = state.broadcast_tx.send(OutputMessage::PatchState {
                patch: PatchGraph { modules },
            });
        }
        
        InputMessage::GetTracks => {
            let patch = state.audio_state.patch.lock();
            for (_, internal_track) in patch.tracks.iter() {
                let _ = state.broadcast_tx.send(OutputMessage::Track {
                    track: internal_track.to_track(),
                });
            }
        }
        
        InputMessage::GetTrack { id } => {
            let patch = state.audio_state.patch.lock();
            if let Some(ref internal_track) = patch.tracks.get(&id) {
                let _ = state.broadcast_tx.send(OutputMessage::Track {
                    track: internal_track.to_track(),
                });
            }
        }
        
        InputMessage::CreateTrack { id } => {
            let mut patch = state.audio_state.patch.lock();
            patch.tracks.insert(id.clone(), Arc::new(InternalTrack::new(id.clone())));
            let _ = state.broadcast_tx.send(OutputMessage::CreateTrack { id });
        }
        
        InputMessage::UpdateTrack { id, update } => {
            let patch = state.audio_state.patch.lock();
            if let Some(ref internal_track) = patch.tracks.get(&id) {
                internal_track.update(&update);
            }
        }
        
        InputMessage::DeleteTrack { id } => {
            let mut patch = state.audio_state.patch.lock();
            patch.tracks.remove(&id);
        }
        
        InputMessage::UpsertKeyframe { keyframe } => {
            let patch = state.audio_state.patch.lock();
            let internal_keyframe = keyframe.to_internal_keyframe(&patch);
            if let Some(ref track) = patch.tracks.get(&keyframe.track_id) {
                track.add_keyframe(internal_keyframe);
            }
        }
        
        InputMessage::DeleteKeyframe { track_id, keyframe_id } => {
            let patch = state.audio_state.patch.lock();
            if let Some(ref track) = patch.tracks.get(&track_id) {
                track.remove_keyframe(keyframe_id);
            }
        }
        
        InputMessage::SubscribeAudio { module_id, port, buffer_size } => {
            let subscription_id = uuid::Uuid::new_v4().to_string();
            let subscription = AudioSubscription {
                id: subscription_id.clone(),
                module_id,
                port,
                buffer_size,
            };
            
            state.audio_state.add_subscription(subscription);
            let _ = state.broadcast_tx.send(OutputMessage::AudioSubscribed { subscription_id });
        }
        
        InputMessage::UnsubscribeAudio { subscription_id } => {
            state.audio_state.remove_subscription(&subscription_id);
        }
        
        InputMessage::Mute => {
            state.audio_state.set_muted(true);
            let _ = state.broadcast_tx.send(OutputMessage::Muted);
        }
        
        InputMessage::Unmute => {
            state.audio_state.set_muted(false);
            let _ = state.broadcast_tx.send(OutputMessage::Unmuted);
        }
        
        InputMessage::StartRecording { filename } => {
            match state.audio_state.start_recording(filename) {
                Ok(path) => {
                    let _ = state.broadcast_tx.send(OutputMessage::RecordingStarted { filename: path });
                }
                Err(e) => {
                    let _ = state.broadcast_tx.send(OutputMessage::Error {
                        message: format!("Failed to start recording: {}", e),
                        errors: None,
                    });
                }
            }
        }
        
        InputMessage::StopRecording => {
            match state.audio_state.stop_recording() {
                Ok(Some(path)) => {
                    let _ = state.broadcast_tx.send(OutputMessage::RecordingStopped { filename: path });
                }
                Ok(None) => {
                    let _ = state.broadcast_tx.send(OutputMessage::Error {
                        message: "No recording in progress".to_string(),
                        errors: None,
                    });
                }
                Err(e) => {
                    let _ = state.broadcast_tx.send(OutputMessage::Error {
                        message: format!("Failed to stop recording: {}", e),
                        errors: None,
                    });
                }
            }
        }
    }
}

// Apply a patch graph to the audio state
fn apply_patch(
    audio_state: &Arc<AudioState>,
    desired_graph: &PatchGraph,
    sample_rate: f32,
) -> anyhow::Result<()> {
    let mut patch_lock = audio_state.patch.lock();
    
    // Build maps for efficient lookup
    let desired_modules: HashMap<String, _> = desired_graph
        .modules
        .iter()
        .map(|m| (m.id.clone(), m))
        .collect();
    
    let current_ids: HashSet<String> = patch_lock.sampleables.keys().cloned().collect();
    let desired_ids: HashSet<String> = desired_modules.keys().cloned().collect();
    
    // Find modules to delete (in current but not in desired), excluding root
    let to_delete: Vec<String> = current_ids
        .difference(&desired_ids)
        .filter(|id| *id != "root")
        .cloned()
        .collect();
    
    // Find modules to create (in desired but not in current)
    let to_create: Vec<String> = desired_ids.difference(&current_ids).cloned().collect();
    
    // Delete modules
    for id in to_delete {
        patch_lock.sampleables.remove(&id);
    }
    
    // Create new modules
    let constructors = get_constructors();
    for id in &to_create {
        if let Some(desired_module) = desired_modules.get(id) {
            if let Some(constructor) = constructors.get(&desired_module.module_type) {
                match constructor(id, sample_rate) {
                    Ok(module) => {
                        patch_lock.sampleables.insert(id.clone(), module);
                    }
                    Err(err) => {
                        return Err(anyhow::anyhow!("Failed to create module {}: {}", id, err));
                    }
                }
            } else {
                return Err(anyhow::anyhow!("{} is not a valid module type", desired_module.module_type));
            }
        }
    }
    
    // Update parameters for all desired modules (both new and existing)
    // Pass 1: Non-cable parameters
    for id in desired_ids.iter() {
        if let Some(desired_module) = desired_modules.get(id) {
            if let Some(module) = patch_lock.sampleables.get(id) {
                for (param_name, param) in &desired_module.params {
                    if !matches!(param, Param::Cable { .. }) {
                        let internal_param = param.to_internal_param(&patch_lock);
                        if let Err(err) = module.update_param(param_name, &internal_param) {
                            return Err(anyhow::anyhow!("Failed to update param {}.{}: {}", id, param_name, err));
                        }
                    }
                }
            }
        }
    }
    
    // Pass 2: Cable parameters (after all modules exist)
    for id in desired_ids.iter() {
        if let Some(desired_module) = desired_modules.get(id) {
            if let Some(module) = patch_lock.sampleables.get(id) {
                for (param_name, param) in &desired_module.params {
                    if matches!(param, Param::Cable { .. }) {
                        let internal_param = param.to_internal_param(&patch_lock);
                        if let Err(err) = module.update_param(param_name, &internal_param) {
                            return Err(anyhow::anyhow!("Failed to update param {}.{}: {}", id, param_name, err));
                        }
                    }
                }
            }
        }
    }
    
    Ok(())
}

// Task to forward output messages to broadcast channel
pub async fn forward_output_messages(
    output_rx: Receiver<OutputMessage>,
    broadcast_tx: broadcast::Sender<OutputMessage>,
) {
    loop {
        match output_rx.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(msg) => {
                // Ignore send errors (no subscribers is fine)
                let _ = broadcast_tx.send(msg);
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                continue;
            }
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                error!("Output channel disconnected");
                break;
            }
        }
    }
}
