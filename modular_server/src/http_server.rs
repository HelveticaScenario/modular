use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Router,
};
use modular_core::{
    crossbeam_channel::{Receiver, Sender},
    message::{InputMessage, OutputMessage},
};
use std::sync::Arc;
use tokio::sync::{broadcast, Mutex as TokioMutex};
use tower_http::cors::CorsLayer;
use tracing::{error, info};

// Shared server state
#[derive(Clone)]
pub struct AppState {
    pub input_tx: Arc<TokioMutex<Sender<InputMessage>>>,
    pub broadcast_tx: broadcast::Sender<OutputMessage>,
}

// Build the Axum router
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // WebSocket endpoint
        .route("/ws", get(ws_handler))
        // Health check
        .route("/health", get(health_check))
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
                // Try YAML serialization first for non-binary messages
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
    let input_tx = state.input_tx.clone();
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                let input_tx = input_tx.lock().await;
                // Try YAML first, then fallback to JSON for backward compatibility
                let message: Result<InputMessage, _> = serde_yaml::from_str(&text)
                    .or_else(|_| serde_json::from_str(&text));
                
                match message {
                    Ok(message) => {
                        if let Err(e) = input_tx.send(message) {
                            error!("Failed to send message to core: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse message (YAML/JSON): {}", e);
                    }
                }
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

// Task to forward output messages to broadcast channel
pub async fn forward_output_messages(
    output_rx: Arc<TokioMutex<Receiver<OutputMessage>>>,
    broadcast_tx: broadcast::Sender<OutputMessage>,
) {
    loop {
        let msg = {
            let rx = output_rx.lock().await;
            rx.recv_timeout(std::time::Duration::from_millis(100))
        };
        
        match msg {
            Ok(msg) => {
                // Ignore send errors (no subscribers is fine)
                let _ = broadcast_tx.send(msg);
            }
            Err(modular_core::crossbeam_channel::RecvTimeoutError::Timeout) => {
                tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                continue;
            }
            Err(modular_core::crossbeam_channel::RecvTimeoutError::Disconnected) => {
                error!("Output channel disconnected");
                break;
            }
        }
    }
}
