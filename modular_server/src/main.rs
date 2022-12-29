use std::{net::SocketAddr, sync::Arc};

use axum::{extract::State, http::StatusCode, response::IntoResponse, routing::post, Router};
use axum_macros::debug_handler;
use modular_core::{types::Param, uuid::Uuid, Modular};
use serde::Serialize;

extern crate anyhow;
extern crate clap;
extern crate modular_core;

#[derive(Serialize)]
struct JsonResponse {
    msg: String,
}

#[tokio::main]
async fn main() {
    println!("hello");
    let modular = Arc::new(Modular::new());

    let app = Router::new()
        .route("/play", post(play))
        .route("/pause", post(pause))
        .route("/demo", post(demo))
        .with_state(modular);

    let addr = SocketAddr::from(([127, 0, 0, 1], 7812));

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
    let sine_id = Uuid::new_v4();
    patch.create_module("sine-oscillator".into(), sine_id.clone())?;
    patch.update_param(sine_id.clone(), "freq".into(), Param::Note { value: 69 })?;
    patch.update_param(
        Uuid::nil(),
        "source".into(),
        Param::Cable {
            module: sine_id.clone(),
            port: "output".into(),
        },
    )?;
    Ok(())
}
