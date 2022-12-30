use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum_macros::debug_handler;
use modular_core::{
    types::{ModuleSchema, ModuleState, Param, ROOT_ID},
    Modular,
};
use serde::Serialize;
use tracing::metadata::LevelFilter;
use tracing_subscriber::EnvFilter;

extern crate anyhow;
extern crate clap;
extern crate modular_core;

#[derive(Serialize)]
struct JsonResponse {
    msg: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(
                    if cfg!(debug_assertions) {
                        LevelFilter::INFO
                    } else {
                        LevelFilter::ERROR
                    }
                    .into(),
                )
                .from_env_lossy(),
        )
        .init();

    let modular = Arc::new(Modular::new());

    let app = Router::new()
        .route("/", post(update))
        .route("/schema", get(get_schema))
        .route("/play", post(play))
        .route("/pause", post(pause))
        .route("/demo", post(demo))
        .route("/modules", get(get_modules))
        .route("/module/:id", get(get_module))
        .with_state(modular);

    let addr = SocketAddr::from(([127, 0, 0, 1], 7812));
    tracing::info!("listening on {}", addr);
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

struct AppError(anyhow::Error);

impl From<anyhow::Error> for AppError {
    fn from(inner: anyhow::Error) -> Self {
        AppError(inner)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Something went wrong: {}", self.0.to_string()),
        )
            .into_response()
    }
}

#[debug_handler]
async fn play(State(modular): State<Arc<Modular>>) -> (StatusCode, std::string::String) {
    match modular.play() {
        Ok(_) => (StatusCode::OK, "Ok".into()),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

#[debug_handler]
async fn pause(State(modular): State<Arc<Modular>>) -> (StatusCode, std::string::String) {
    match modular.pause() {
        Ok(_) => (StatusCode::OK, "Ok".into()),
        Err(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
    }
}

#[debug_handler]
async fn demo(State(modular): State<Arc<Modular>>) -> Result<(), AppError> {
    let mut patch = modular.patch.lock();
    let sine_id: String = "sine".into();
    patch.create_module("sine-oscillator".into(), &sine_id)?;
    patch.update_param(&sine_id, &"freq".into(), &Param::Note { value: 69 })?;
    patch.update_param(
        &ROOT_ID,
        &"source".into(),
        &Param::Cable {
            module: sine_id.clone(),
            port: "output".into(),
        },
    )?;
    Ok(())
}
#[debug_handler]
async fn update(State(_modular): State<Arc<Modular>>) -> Result<(), AppError> {
    unimplemented!()
}

#[debug_handler]
async fn get_modules(State(modular): State<Arc<Modular>>) -> Json<Vec<ModuleState>> {
    Json(modular.patch.lock().get_modules())
}

#[debug_handler]
async fn get_module(
    State(modular): State<Arc<Modular>>,
    Path(id): Path<String>,
) -> Json<Option<ModuleState>> {
    Json(modular.patch.lock().get_module(&id))
}

#[debug_handler]
async fn get_schema(State(modular): State<Arc<Modular>>) -> Json<Vec<ModuleSchema>> {
    Json(modular.schema.clone())
}
