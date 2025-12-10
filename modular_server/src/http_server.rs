use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result};
use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
    routing::get,
};
use modular_core::types::ScopeItem;
use modular_core::{
    PatchGraph,
    dsp::{get_constructors, schema},
};
use tokio::sync::mpsc::{self, Sender};
use tokio::task::JoinHandle;

use tower_http::cors::CorsLayer;
use tower_http::services::{ServeDir, ServeFile};
use tracing::{error, info};

use crate::{
    AudioState,
    protocol::{InputMessage, OutputMessage},
    validation::validate_patch,
};

// Shared server state
#[derive(Clone)]
pub struct AppState {
    pub audio_state: Arc<AudioState>,
}

// Build the Axum router
pub fn create_router(state: AppState) -> Router {
    // Serve static files from the static directory
    // Falls back to index.html for SPA routing
    // let static_dir = std::env::current_dir().unwrap_or_default().join("static");

    let serve_dir =
        ServeDir::new("./static").not_found_service(ServeFile::new("./static/index.html"));

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
async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_subscription(
    audio_state: Arc<AudioState>,
    subscription: ScopeItem,
    socket_tx: Sender<Message>,
) {
    let (tx, mut rx) = mpsc::channel(32);
    audio_state.add_subscription(subscription, tx).await;
    while let Some(msg) = rx.recv().await {
        match msg {
            OutputMessage::AudioBuffer {
                subscription,
                samples,
            } => {
                let mut bytes = Vec::new();

                match subscription {
                    ScopeItem::ModuleOutput {
                        module_id,
                        port_name,
                    } => {
                        bytes.extend_from_slice(module_id.as_bytes());
                        bytes.push(0);
                        bytes.extend_from_slice(port_name.as_bytes());
                    }
                    ScopeItem::Track { track_id } => {
                        bytes.extend_from_slice(track_id.as_bytes());
                        bytes.push(0);
                        // Use empty port segment to signal track samples
                    }
                }

                bytes.push(0); // null terminator between metadata and payload
                for sample in samples {
                    bytes.extend_from_slice(&sample.to_le_bytes());
                }
                if socket_tx.send(Message::Binary(bytes)).await.is_err() {
                    info!("Failed to send audio buffer message, closing subscription");
                    break;
                }
            }
            _ => {
                info!("Ignoring non-audio output message");
            }
        }
    }
    // Implementation for handling subscriptions
}

async fn send_message(socket_tx: Sender<Message>, message: OutputMessage) -> anyhow::Result<()> {
    let json = serde_json::to_string(&message)?;
    socket_tx.send(Message::Text(json)).await?;
    Ok(())
}

async fn handle_set_patch(
    patch: &PatchGraph,
    audio_state: &Arc<AudioState>,
    sample_rate: f32,
    socket_tx: Sender<Message>,
) -> bool {
    // Validate patch
    let schemas = schema();
    if let Err(errors) = validate_patch(patch, &schemas) {
        let _ = send_message(
            socket_tx,
            OutputMessage::Error {
                message: "Validation failed".to_string(),
                errors: Some(errors),
            },
        )
        .await;
        return false;
    }

    // Apply patch
    if let Err(e) = apply_patch(audio_state, patch, sample_rate).await {
        let _ = send_message(
            socket_tx,
            OutputMessage::Error {
                message: format!("Failed to apply patch: {}", e),
                errors: None,
            },
        )
        .await;
        return false;
    }

    // Auto-unmute on SetPatch to match prior imperative flow
    audio_state.set_muted(false);
    let _ = send_message(socket_tx, OutputMessage::MuteState { muted: false }).await;

    true
}

async fn sync_scopes_for_connection(
    scopes: &[ScopeItem],
    audio_state: Arc<AudioState>,
    subscriptions: Arc<tokio::sync::Mutex<HashMap<ScopeItem, JoinHandle<()>>>>,
    socket_tx: Sender<Message>,
) {
    let desired: HashSet<ScopeItem> = scopes.iter().cloned().collect();

    let mut guard = subscriptions.lock().await;
    let existing: HashSet<ScopeItem> = guard.keys().cloned().collect();

    for sub in existing.difference(&desired) {
        if let Some(handle) = guard.remove(sub) {
            handle.abort();
        }
    }

    for sub in desired.difference(&existing) {
        let sub = sub.clone();
        let sub_for_task = sub.clone();
        let audio_state = audio_state.clone();
        let socket_tx = socket_tx.clone();

        let handle = tokio::spawn(async move {
            handle_subscription(audio_state, sub_for_task, socket_tx).await;
        });

        guard.insert(sub, handle);
    }
}

async fn handle_message(
    message: InputMessage,
    audio_state: &Arc<AudioState>,
    sample_rate: f32,
    socket_tx: Sender<Message>,
) {
    match message {
        InputMessage::GetSchemas => {
            let _ = send_message(socket_tx, OutputMessage::Schemas { schemas: schema() }).await;
        }
        InputMessage::GetPatch => {
            // GetPatch is deprecated - DSL scripts are the source of truth on the client
            // This is kept for backwards compatibility but does nothing
        }
        InputMessage::SetPatch { patch } => {
            let _ = handle_set_patch(&patch, audio_state, sample_rate, socket_tx).await;
        }
        InputMessage::ListFiles => match list_dsl_files() {
            Ok(files) => {
                let _ = send_message(socket_tx, OutputMessage::FileList { files }).await;
            }
            Err(e) => {
                let _ = send_message(
                    socket_tx,
                    OutputMessage::Error {
                        message: format!("Failed to list files: {}", e),
                        errors: None,
                    },
                )
                .await;
            }
        },
        InputMessage::ReadFile { path } => match read_dsl_file(&path) {
            Ok(content) => {
                let _ = send_message(
                    socket_tx,
                    OutputMessage::FileContent {
                        path: path.clone(),
                        content,
                    },
                )
                .await;
            }
            Err(e) => {
                let _ = send_message(
                    socket_tx,
                    OutputMessage::Error {
                        message: format!("Failed to read file: {}", e),
                        errors: None,
                    },
                )
                .await;
            }
        },
        InputMessage::WriteFile { path, content } => {
            match write_dsl_file(&path, &content) {
                Ok(_) => {
                    // Success - no response needed
                }
                Err(e) => {
                    let _ = send_message(
                        socket_tx,
                        OutputMessage::Error {
                            message: format!("Failed to write file: {}", e),
                            errors: None,
                        },
                    )
                    .await;
                }
            }
        }
        InputMessage::RenameFile { from, to } => match rename_dsl_file(&from, &to) {
            Ok(_) => match list_dsl_files() {
                Ok(files) => {
                    let _ = send_message(socket_tx, OutputMessage::FileList { files }).await;
                }
                Err(e) => {
                    let _ = send_message(
                        socket_tx,
                        OutputMessage::Error {
                            message: format!("Renamed file but failed to refresh list: {}", e),
                            errors: None,
                        },
                    )
                    .await;
                }
            },
            Err(e) => {
                let _ = send_message(
                    socket_tx,
                    OutputMessage::Error {
                        message: format!("Failed to rename file: {}", e),
                        errors: None,
                    },
                )
                .await;
            }
        },
        InputMessage::DeleteFile { path } => {
            match delete_dsl_file(&path) {
                Ok(_) => {
                    // Success - no response needed
                }
                Err(e) => {
                    let _ = send_message(
                        socket_tx,
                        OutputMessage::Error {
                            message: format!("Failed to delete file: {}", e),
                            errors: None,
                        },
                    )
                    .await;
                }
            }
        }
        InputMessage::Mute => {
            audio_state.set_muted(true);
            let _ = send_message(socket_tx, OutputMessage::MuteState { muted: true }).await;
        }
        InputMessage::Unmute => {
            audio_state.set_muted(false);
            let _ = send_message(socket_tx, OutputMessage::MuteState { muted: false }).await;
        }
        InputMessage::StartRecording { filename } => {
            match audio_state.start_recording(filename).await {
                Ok(path) => {
                    info!("Recording started ${}", path);
                }
                Err(e) => {
                    error!("Failed to start recording: {}", e);
                    let _ = send_message(
                        socket_tx,
                        OutputMessage::Error {
                            message: format!("Failed to start recording: {}", e),
                            errors: None,
                        },
                    )
                    .await;
                }
            }
        }
        InputMessage::StopRecording => match audio_state.stop_recording().await {
            Ok(Some(path)) => {
                info!("Recording stopped: {}", path);
            }
            Ok(None) => {
                let _ = send_message(
                    socket_tx,
                    OutputMessage::Error {
                        message: "No recording in progress".to_string(),
                        errors: None,
                    },
                )
                .await;
            }
            Err(e) => {
                match send_message(
                    socket_tx,
                    OutputMessage::Error {
                        message: format!("Failed to stop recording: {:?}", e),
                        errors: None,
                    },
                )
                .await
                {
                    Ok(_) => {}
                    Err(e) => {
                        error!("Failed to send Error message: {:?}", e);
                    }
                }
            }
        },
    }
}

async fn apply_patch(
    audio_state: &Arc<AudioState>,
    desired_graph: &PatchGraph,
    sample_rate: f32,
) -> anyhow::Result<()> {
    let mut patch_lock = audio_state.patch.lock().await;

    // Build maps for efficient lookup
    let desired_modules: HashMap<String, _> = desired_graph
        .modules
        .iter()
        .map(|m| (m.id.clone(), m))
        .collect();

    let current_ids: HashSet<String> = patch_lock.sampleables.keys().cloned().collect();
    let desired_ids: HashSet<String> = desired_modules.keys().cloned().collect();
    println!("Current IDs: {:?}", current_ids);
    println!("Desired IDs: {:?}", desired_ids);

    // Find modules to delete (in current but not in desired), excluding root
    let mut to_delete: Vec<String> = current_ids
        .difference(&desired_ids)
        .filter(|id| *id != "root")
        .cloned()
        .collect();

    // Find modules where type changed (same ID but different module_type)
    // These need to be deleted and recreated
    let mut to_recreate: Vec<String> = Vec::new();
    for id in current_ids.intersection(&desired_ids) {
        if *id == "root" {
            continue; // Never recreate root
        }
        if let (Some(current_module), Some(desired_module)) =
            (patch_lock.sampleables.get(id), desired_modules.get(id))
        {
            let current_state = current_module.get_state();
            if current_state.module_type != desired_module.module_type {
                to_recreate.push(id.clone());
                to_delete.push(id.clone());
            }
        }
    }

    println!("To delete: {:?}", to_delete);

    // Find modules to create (in desired but not in current, plus recreated modules)
    let mut to_create: Vec<String> = desired_ids.difference(&current_ids).cloned().collect();
    to_create.extend(to_recreate);

    println!("To create: {:?}", to_create);

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
                return Err(anyhow::anyhow!(
                    "{} is not a valid module type",
                    desired_module.module_type
                ));
            }
        }
    }

    // ===== TRACK LIFECYCLE =====

    // Build maps for efficient track lookup
    let desired_tracks: HashMap<String, _> = desired_graph
        .tracks
        .iter()
        .map(|t| (t.id.clone(), t))
        .collect();

    let current_track_ids: HashSet<String> = patch_lock.tracks.keys().cloned().collect();
    let desired_track_ids: HashSet<String> = desired_tracks.keys().cloned().collect();

    // Delete removed tracks (in current but not in desired)
    let tracks_to_delete: Vec<String> = current_track_ids
        .difference(&desired_track_ids)
        .cloned()
        .collect();

    println!("Tracks to delete: {:?}", tracks_to_delete);

    for track_id in tracks_to_delete {
        patch_lock.tracks.remove(&track_id);
    }

    // Two-pass track creation to handle keyframes that reference other tracks

    // PASS 1: Create/update track shells (without configuration or keyframes)
    for track in &desired_graph.tracks {
        match patch_lock.tracks.get(&track.id) {
            Some(existing_track) => {
                // Existing track: clear all keyframes (will re-add in pass 2)
                println!("Updating track: {}", track.id);
                let current_keyframes = existing_track.to_track().keyframes;
                for kf in current_keyframes {
                    existing_track.remove_keyframe(kf.id);
                }
            }
            None => {
                // Create new track shell with a disconnected playhead param
                println!("Creating track: {}", track.id);
                let default_playhead_param =
                    modular_core::Param::Disconnected.to_internal_param(&patch_lock);
                let internal_track = Arc::new(modular_core::types::InternalTrack::new(
                    track.id.clone(),
                    default_playhead_param,
                    track.interpolation_type,
                ));
                patch_lock.tracks.insert(track.id.clone(), internal_track);
            }
        }
    }

    // PASS 2: Configure tracks and add keyframes (all tracks now exist for Track param resolution)
    for track in &desired_graph.tracks {
        if let Some(internal_track) = patch_lock.tracks.get(&track.id) {
            // Configure playhead parameter and interpolation type
            let playhead_param = track.playhead.to_internal_param(&patch_lock);
            internal_track.configure(playhead_param, track.interpolation_type);

            // Add keyframes (params may reference other tracks, which now exist)
            for kf in &track.keyframes {
                let internal_kf = kf.to_internal_keyframe(&patch_lock);
                internal_track.add_keyframe(internal_kf);
            }
        }
    }

    // Update parameters for all desired modules (both new and existing)
    // This happens AFTER tracks are created so Track params can resolve
    for id in desired_ids.iter() {
        if let Some(desired_module) = desired_modules.get(id) {
            if let Some(module) = patch_lock.sampleables.get(id) {
                for (param_name, param) in &desired_module.params {
                    let internal_param = param.to_internal_param(&patch_lock);
                    if let Err(err) = module.update_param(param_name, &internal_param) {
                        return Err(anyhow::anyhow!(
                            "Failed to update param {}.{}: {}",
                            id,
                            param_name,
                            err
                        ));
                    }
                }
            }
        }
    }

    Ok(())
}
// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, state: AppState) {
    info!("New WebSocket connection established");

    use futures_util::{SinkExt, StreamExt};

    let subscriptions = Arc::new(tokio::sync::Mutex::new(
        HashMap::<ScopeItem, JoinHandle<()>>::new(),
    ));
    let handles: Arc<tokio::sync::Mutex<Vec<JoinHandle<()>>>> =
        Arc::new(tokio::sync::Mutex::new(Vec::new()));
    // Spawn task to forward broadcast messages to this client
    let (mut socket_tx, mut socket_rx) = socket.split();
    let (fan_tx, mut fan_rx) = mpsc::channel::<Message>(1024);

    // Send initial mute state to the client on connect
    let _ = send_message(
        fan_tx.clone(),
        OutputMessage::MuteState {
            muted: state.audio_state.is_muted(),
        },
    )
    .await;

    let mut socket_task = tokio::spawn(async move {
        // Implementation for sending messages to client
        while let Some(message) = fan_rx.recv().await {
            match socket_tx.send(message).await {
                Ok(_) => {}
                Err(e) => {
                    error!("Failed to send message to WebSocket client: {}", e);
                    break;
                }
            }
        }
    });

    let mut clean_task = {
        let subscriptions = subscriptions.clone();
        let handles = handles.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(60)).await;
                info!("Cleaning up finished message handles");
                subscriptions
                    .lock()
                    .await
                    .retain(|_, handle| !handle.is_finished());
                handles.lock().await.retain(|handle| handle.is_finished());
            }
        })
    };
    let mut recv_task = {
        let subscriptions = subscriptions.clone();
        let handles = handles.clone();
        tokio::spawn(async move {
            while let Some(Ok(msg)) = socket_rx.next().await {
                println!("Received WebSocket message: {:?}", msg);
                match msg {
                    Message::Text(text) => match serde_json::from_str::<InputMessage>(&text) {
                        Ok(InputMessage::SetPatch { patch }) => {
                            info!("Received input message: SetPatch");
                            let audio_state = state.audio_state.clone();
                            let sample_rate = state.audio_state.sample_rate;
                            let socket_tx = fan_tx.clone();
                            let subscriptions = subscriptions.clone();
                            handles.lock().await.push(tokio::spawn(async move {
                                if handle_set_patch(
                                    &patch,
                                    &audio_state,
                                    sample_rate,
                                    socket_tx.clone(),
                                )
                                .await
                                {
                                    sync_scopes_for_connection(
                                        &patch.scopes,
                                        audio_state,
                                        subscriptions,
                                        socket_tx,
                                    )
                                    .await;
                                }
                            }));
                        }
                        Ok(input_msg) => {
                            info!("Received input message: {:?}", input_msg);
                            let audio_state = state.audio_state.clone();
                            let sample_rate = state.audio_state.sample_rate;
                            let socket_tx = fan_tx.clone();
                            handles.lock().await.push(tokio::spawn({
                                async move {
                                    handle_message(input_msg, &audio_state, sample_rate, socket_tx)
                                        .await;
                                }
                            }));
                        }
                        Err(e) => {
                            error!("Failed to parse input message: {}", e);
                        }
                    },
                    Message::Close(close_frame) => {
                        info!("WebSocket closed: {:?}", close_frame);
                        break;
                    }
                    msg => {
                        info!("Ignoring non-binary WebSocket message: {:?}", msg);
                    }
                }
            }
        })
    };
    // // Handle incoming messages from client

    // // Wait for either task to complete (disconnect)
    tokio::select! {
        _ = (&mut socket_task) => socket_task.abort(),
        _ = (&mut clean_task) => clean_task.abort(),
        _ = (&mut recv_task) => recv_task.abort(),
    }
    info!("WebSocket connection handler ending, cleaning up");

    handles
        .lock()
        .await
        .iter()
        .for_each(|handle| handle.abort());
    subscriptions
        .lock()
        .await
        .iter()
        .for_each(|(_, handle)| handle.abort());

    info!("WebSocket connection closed");
}

// File operation functions for DSL persistence

/// Get the patches directory path (current working directory)
fn get_patches_dir() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Validate that a path is safe (no path traversal attacks)
fn is_safe_path(path: &str) -> bool {
    // Reject paths with .. or absolute paths
    !path.contains("..") && !Path::new(path).is_absolute()
}

/// List all .js files in the patches directory
fn list_dsl_files() -> Result<Vec<String>> {
    let patches_dir = get_patches_dir();
    let mut files = Vec::new();

    for entry in fs::read_dir(&patches_dir).context("Failed to read patches directory")? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "mjs" {
                    if let Some(filename) = path.file_name() {
                        if let Some(name) = filename.to_str() {
                            files.push(name.to_string());
                        }
                    }
                }
            }
        }
    }

    files.sort();
    Ok(files)
}

/// Read a DSL file from the patches directory
fn read_dsl_file(filename: &str) -> Result<String> {
    if !is_safe_path(filename) {
        anyhow::bail!("Invalid file path");
    }

    let patches_dir = get_patches_dir();
    let file_path = patches_dir.join(filename);

    fs::read_to_string(&file_path).with_context(|| format!("Failed to read file: {}", filename))
}

/// Write a DSL file to the patches directory
fn write_dsl_file(filename: &str, content: &str) -> Result<()> {
    if !is_safe_path(filename) {
        anyhow::bail!("Invalid file path");
    }

    // Ensure filename ends with .mjs
    if !filename.ends_with(".mjs") {
        anyhow::bail!("File must have .mjs extension");
    }

    let patches_dir = get_patches_dir();
    let file_path = patches_dir.join(filename);

    fs::write(&file_path, content).with_context(|| format!("Failed to write file: {}", filename))
}

/// Delete a DSL file from the patches directory
fn delete_dsl_file(filename: &str) -> Result<()> {
    if !is_safe_path(filename) {
        anyhow::bail!("Invalid file path");
    }

    let patches_dir = get_patches_dir();
    let file_path = patches_dir.join(filename);

    fs::remove_file(&file_path).with_context(|| format!("Failed to delete file: {}", filename))
}

/// Rename a DSL file within the patches directory
fn rename_dsl_file(from: &str, to: &str) -> Result<()> {
    if !is_safe_path(from) || !is_safe_path(to) {
        anyhow::bail!("Invalid file path");
    }

    if !from.ends_with(".mjs") || !to.ends_with(".mjs") {
        anyhow::bail!("File must have .mjs extension");
    }

    let patches_dir = get_patches_dir();
    let from_path = patches_dir.join(from);
    let to_path = patches_dir.join(to);

    if !from_path.exists() {
        anyhow::bail!("Source file does not exist");
    }

    if to_path.exists() {
        anyhow::bail!("A file with that name already exists");
    }

    fs::rename(&from_path, &to_path)
        .with_context(|| format!("Failed to rename file: {} -> {}", from, to))
}
