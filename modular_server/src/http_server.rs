use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Path, State,
    },
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{delete, get, post, put},
    Json, Router,
};
use modular_core::{
    crossbeam_channel::{Receiver, Sender},
    message::{InputMessage, OutputMessage},
    types::{Param, PatchGraph},
};
use serde::{Deserialize, Serialize};
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

// REST API request/response types
#[derive(Debug, Serialize, Deserialize)]
pub struct CreateModuleRequest {
    module_type: String,
    id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateParamRequest {
    param: Param,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SetPatchRequest {
    graph: PatchGraph,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    error: String,
}

// Build the Axum router
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Primary declarative API
        .route("/patch", put(set_patch))
        // REST endpoints (legacy/convenience)
        .route("/schemas", get(get_schemas))
        .route("/modules", post(create_module))
        .route("/modules/:id", delete(delete_module))
        .route("/params/:id/:param_name", put(update_param))
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

// PUT /patch - Set complete patch state (declarative API)
async fn set_patch(
    State(state): State<AppState>,
    Json(payload): Json<SetPatchRequest>,
) -> Result<StatusCode, AppError> {
    let input_tx = state.input_tx.lock().await;
    
    input_tx
        .send(InputMessage::SetPatch {
            graph: payload.graph,
        })
        .map_err(|e| AppError::Internal(format!("Failed to send message: {}", e)))?;

    // Response with updated state will be sent via WebSocket
    Ok(StatusCode::ACCEPTED)
}

// GET /schemas - Get all available module schemas
async fn get_schemas(State(state): State<AppState>) -> Result<StatusCode, AppError> {
    let input_tx = state.input_tx.lock().await;
    
    input_tx
        .send(InputMessage::Schema)
        .map_err(|e| AppError::Internal(format!("Failed to send message: {}", e)))?;

    // Response will be sent via WebSocket
    Ok(StatusCode::ACCEPTED)
}

// POST /modules - Create a new module
async fn create_module(
    State(state): State<AppState>,
    Json(payload): Json<CreateModuleRequest>,
) -> Result<StatusCode, AppError> {
    let input_tx = state.input_tx.lock().await;
    
    input_tx
        .send(InputMessage::CreateModule {
            module_type: payload.module_type.clone(),
            id: payload.id.clone(),
        })
        .map_err(|e| AppError::Internal(format!("Failed to send message: {}", e)))?;

    // Response will be sent via WebSocket
    Ok(StatusCode::ACCEPTED)
}

// DELETE /modules/:id - Delete a module
async fn delete_module(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, AppError> {
    let input_tx = state.input_tx.lock().await;
    
    input_tx
        .send(InputMessage::DeleteModule { id })
        .map_err(|e| AppError::Internal(format!("Failed to send message: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
}

// PUT /params/:id/:param_name - Update a parameter
async fn update_param(
    State(state): State<AppState>,
    Path((id, param_name)): Path<(String, String)>,
    Json(payload): Json<UpdateParamRequest>,
) -> Result<StatusCode, AppError> {
    let input_tx = state.input_tx.lock().await;
    
    input_tx
        .send(InputMessage::UpdateParam {
            id,
            param_name,
            param: payload.param,
        })
        .map_err(|e| AppError::Internal(format!("Failed to send message: {}", e)))?;

    Ok(StatusCode::NO_CONTENT)
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
                // Regular JSON message
                let json = match serde_json::to_string(&msg) {
                    Ok(j) => j,
                    Err(e) => {
                        error!("Failed to serialize message: {}", e);
                        continue;
                    }
                };
                Message::Text(json)
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
                match serde_json::from_str::<InputMessage>(&text) {
                    Ok(message) => {
                        if let Err(e) = input_tx.send(message) {
                            error!("Failed to send message to core: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        error!("Failed to parse message: {}", e);
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

// Error handling
pub enum AppError {
    BadRequest(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(ErrorResponse {
            error: error_message,
        });

        (status, body).into_response()
    }
}
