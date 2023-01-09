use std::{net::SocketAddr, sync::Arc, time::Duration};

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use axum_macros::debug_handler;
use modular_core::{
    types::{Keyframe, ModuleSchema, ModuleState, Param, Playmode, TrackUpdate, ROOT_ID},
    Modular,
};
use serde::{Deserialize, Serialize};
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
        .route("/demo-commands", get(get_commands))
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
    patch.update_param(&sine_id, &"freq", &Param::Note { value: 69 })?;
    patch.update_param(
        &ROOT_ID,
        &"source",
        &Param::Cable {
            module: sine_id.clone(),
            port: "output".into(),
        },
    )?;
    Ok(())
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModuleParam {
    name: String,
    param: Param,
}

impl ModuleParam {
    pub fn to_update_param(self, id: &str) -> UpdateParam {
        UpdateParam {
            id: id.into(),
            name: self.name,
            param: self.param,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UpdateParam {
    id: String,
    name: String,
    #[serde(flatten)]
    param: Param,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Module {
    id: String,
    module_type: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(tag = "type", rename_all = "kebab-case", content = "payload")]
pub enum UpdateCommand {
    #[serde(rename_all = "camelCase")]
    CreateModule {
        #[serde(flatten)]
        module: Module,
        params: Option<Vec<ModuleParam>>,
    },
    #[serde(rename_all = "camelCase")]
    UpdateParam(UpdateParam),
    DeleteModule {
        id: String,
    },
    CreateTrack {
        id: String,
    },
    #[serde(rename_all = "camelCase")]
    UpdateTrack {
        id: String,
        track_update: TrackUpdate,
    },
    DeleteTrack {
        id: String,
    },
    UpsertKeyframe {
        keyframe: Keyframe,
    },
    #[serde(rename_all = "camelCase")]
    DeleteKeyframe {
        id: String,
        track_id: String,
    },
}

async fn update(
    State(modular): State<Arc<Modular>>,
    Json(commands): Json<Vec<UpdateCommand>>,
) -> Result<(), AppError> {
    let (mut normalized_commands, extracted_update_commands) = commands.into_iter().fold(
        (vec![], vec![]),
        |(mut normalized_commands, mut extracted_update_commands): (
            Vec<UpdateCommand>,
            Vec<UpdateCommand>,
        ),
         command| {
            normalized_commands.push(match command {
                UpdateCommand::CreateModule { module, params } => {
                    if let Some(params) = params {
                        extracted_update_commands.extend(params.into_iter().map(|param| {
                            UpdateCommand::UpdateParam(param.to_update_param(&module.id))
                        }));
                    }

                    UpdateCommand::CreateModule {
                        module,
                        params: None,
                    }
                }
                uc => uc,
            });
            (normalized_commands, extracted_update_commands)
        },
    );
    normalized_commands.extend(extracted_update_commands);
    let mut patch = modular.patch.lock();
    for command in normalized_commands.into_iter() {
        match command {
            UpdateCommand::CreateModule {
                module: Module { id, module_type },
                params: _,
            } => patch.create_module(&module_type, &id)?,
            UpdateCommand::UpdateParam(UpdateParam { id, name, param }) => {
                patch.update_param(&id, &name, &param)?
            }
            UpdateCommand::DeleteModule { id } => patch.delete_module(&id),
            UpdateCommand::CreateTrack { id } => patch.create_track(&id),
            UpdateCommand::UpdateTrack { id, track_update } => {
                patch.update_track(&id, &track_update)?
            }
            UpdateCommand::DeleteTrack { id } => patch.delete_module(&id),
            UpdateCommand::UpsertKeyframe { keyframe } => patch.upsert_keyframe(&keyframe)?,
            UpdateCommand::DeleteKeyframe { id, track_id } => patch.delete_keyframe(&id, &track_id),
        }
    }
    Ok(())
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

#[debug_handler]
async fn get_commands() -> Json<Vec<UpdateCommand>> {
    Json(vec![
        UpdateCommand::CreateModule {
            module: Module {
                module_type: "sine-osc".into(),
                id: "sine".into(),
            },
            params: Some(vec![
                ModuleParam {
                    name: "signal".into(),
                    param: Param::Cable {
                        module: "sine".into(),
                        port: "output".into(),
                    },
                },
                ModuleParam {
                    name: "signal".into(),
                    param: Param::Value { value: 1.234 },
                },
                ModuleParam {
                    name: "signal".into(),
                    param: Param::Note { value: 69 },
                },
                ModuleParam {
                    name: "signal".into(),
                    param: Param::Track {
                        track: "track1".into(),
                    },
                },
                ModuleParam {
                    name: "signal".into(),
                    param: Param::Disconnected,
                },
            ]),
        },
        UpdateCommand::UpdateParam(UpdateParam {
            id: "sine".into(),
            name: "output".into(),
            param: Param::Disconnected,
        }),
        UpdateCommand::DeleteModule { id: "sine".into() },
        UpdateCommand::CreateTrack {
            id: "track1".into(),
        },
        UpdateCommand::UpdateTrack {
            id: "track1".into(),
            track_update: TrackUpdate {
                length: None,
                play_mode: Some(Playmode::Once),
            },
        },
        UpdateCommand::UpdateTrack {
            id: "track1".into(),
            track_update: TrackUpdate {
                length: Some(Duration::from_millis(123)),
                play_mode: Some(Playmode::Loop),
            },
        },
        UpdateCommand::DeleteTrack {
            id: "track1".into(),
        },
        UpdateCommand::UpsertKeyframe {
            keyframe: Keyframe::new("kf1", "track1", Duration::from_secs(1), Param::Disconnected),
        },
        UpdateCommand::DeleteKeyframe {
            id: "kf1".into(),
            track_id: "track1".into(),
        },
    ])
}
