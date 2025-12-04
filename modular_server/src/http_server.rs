use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use axum::{
    Router,
    extract::{
        State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
    routing::get,
};
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
    protocol::{AudioSubscription, InputMessage, OutputMessage},
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
    subscription: AudioSubscription,
    socket_tx: Sender<Message>,
) {
    let (tx, mut rx) = mpsc::channel(32);
    audio_state.add_subscription(subscription, tx).await;
    while let Some(msg) = rx.recv().await {
        match msg {
            OutputMessage::AudioBuffer {
                subscription: AudioSubscription { module_id, port },
                samples,
            } => {
                // println!("Sending audio buffer for subscription: {}", subscription_id);
                // Convert to binary: subscription_id as null-terminated string + f32 samples
                let mut bytes = module_id.as_bytes().to_vec();
                bytes.push(0); // null terminator
                bytes.extend_from_slice(port.as_bytes());
                bytes.push(0); // null terminator
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

async fn handle_message(
    message: InputMessage,
    audio_state: &Arc<AudioState>,
    sample_rate: f32,
    socket_tx: Sender<Message>,
) {
    match message {
        InputMessage::Echo { message } => {
            let _ = send_message(
                socket_tx,
                OutputMessage::Echo {
                    message: format!("{}!", message),
                },
            )
            .await;
        }
        InputMessage::GetSchemas => {
            let _ = send_message(socket_tx, OutputMessage::Schemas { schemas: schema() }).await;
        }
        InputMessage::GetPatch => {
            let _ = send_message(
                socket_tx,
                OutputMessage::Patch {
                    patch: audio_state.patch_code.clone(),
                },
            );
        }
        InputMessage::SetPatch { yaml } => {
            // Parse YAML into PatchGraph
            let patch: PatchGraph = match serde_yaml::from_str(&yaml) {
                Ok(p) => p,
                Err(e) => {
                    let _ = send_message(
                        socket_tx,
                        OutputMessage::Error {
                            message: format!("YAML parse error: {}", e),
                            errors: None,
                        },
                    )
                    .await;
                    return;
                }
            };

            println!("Patch {:?}", patch);

            // Validate patch
            let schemas = schema();
            if let Err(errors) = validate_patch(&patch, &schemas) {
                let _ = send_message(
                    socket_tx,
                    OutputMessage::Error {
                        message: "Validation failed".to_string(),
                        errors: Some(errors),
                    },
                )
                .await;
                return;
            }

            // Apply patch
            if let Err(e) = apply_patch(audio_state, &patch, sample_rate).await {
                let _ = send_message(
                    socket_tx,
                    OutputMessage::Error {
                        message: format!("Failed to apply patch: {}", e),
                        errors: None,
                    },
                )
                .await;
                return;
            }

            // Auto-unmute on SetPatch - convenient for live editing workflows
            // where you typically want to hear changes immediately
            audio_state.set_muted(false);
        }
        InputMessage::Mute => {
            audio_state.set_muted(true);
        }
        InputMessage::Unmute => {
            audio_state.set_muted(false);
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
        _ => {
            info!("Unhandled input message: {:?}", message);
        }
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
        if id == "root" {
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

    // Update parameters for all desired modules (both new and existing)
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

    let subscriptions = Arc::new(tokio::sync::Mutex::new(HashMap::<
        AudioSubscription,
        JoinHandle<()>,
    >::new()));
    let handles: Arc<tokio::sync::Mutex<Vec<JoinHandle<()>>>> =
        Arc::new(tokio::sync::Mutex::new(Vec::new()));
    // Spawn task to forward broadcast messages to this client
    let (mut socket_tx, mut socket_rx) = socket.split();
    let (fan_tx, mut fan_rx) = mpsc::channel::<Message>(1024);
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
                        Ok(InputMessage::SubscribeAudio { subscription }) => {
                            let audio_state = state.audio_state.clone();
                            let socket_tx = fan_tx.clone();
                            subscriptions.lock().await.insert(
                                subscription.clone(),
                                tokio::spawn(async move {
                                    handle_subscription(audio_state, subscription, socket_tx).await;
                                }),
                            );
                        }

                        Ok(InputMessage::UnsubscribeAudio { subscription }) => {
                            match subscriptions.lock().await.remove(&subscription) {
                                Some(handle) => {
                                    handle.abort();
                                    info!("Unsubscribed from audio subscription");
                                }
                                None => {
                                    info!("No active subscription found to unsubscribe");
                                }
                            }
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
                        info!("Ignoring non-text WebSocket message: {:?}", msg);
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
