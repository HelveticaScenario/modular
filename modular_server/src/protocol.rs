use modular_core::types::{ModuleSchema, PatchGraph, ScopeItem};
use serde::{Deserialize, Serialize};

use ts_rs::TS;

use crate::collab::CollabInitPayload;

/// Input messages from clients
#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export, export_to = "../../modular_web/src/types/generated/")]
pub enum InputMessage {
    Echo {
        message: String,
    },
    GetSchemas,
    GetPatch,

    SetPatch {
        patch: PatchGraph,
    },

    // Collaborative editing powered by yrs
    CollabYrsJoin {
        doc_id: String,
        client_id: String,
        /// Client-side awareness id (usually yjs doc.clientID) so the server can clean up on disconnect.
        #[ts(type = "number")]
        awareness_client_id: u64,
        awareness_update: Option<Vec<u8>>,
    },
    CollabYrsUpdate {
        doc_id: String,
        update: Vec<u8>,
    },
    CollabYrsAwareness {
        doc_id: String,
        update: Vec<u8>,
    },
    CollabYrsLeave {
        doc_id: String,
        #[ts(type = "number")]
        awareness_client_id: u64,
    },

    // Audio control
    Mute,
    Unmute,

    // Recording
    StartRecording {
        filename: Option<String>,
    },
    StopRecording,

    // File operations
    ListFiles,
    ReadFile {
        path: String,
    },
    WriteFile {
        path: String,
        content: String,
    },
    DeleteFile {
        path: String,
    },
}

/// Output messages to clients
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
#[ts(export, export_to = "../../modular_web/src/types/generated/")]
pub enum OutputMessage {
    Echo {
        message: String,
    },
    Schemas {
        schemas: Vec<ModuleSchema>,
    },
    CollabYrsInit {
        init: CollabInitPayload,
    },
    CollabYrsUpdate {
        doc_id: String,
        update: Vec<u8>,
    },
    CollabYrsAwareness {
        doc_id: String,
        update: Vec<u8>,
    },
    Error {
        message: String,
        errors: Option<Vec<ValidationError>>,
    },

    /// Current mute state of the audio engine
    MuteState {
        muted: bool,
    },

    // Audio streaming
    AudioBuffer {
        subscription: ScopeItem,
        #[ts(type = "Float32Array")]
        samples: Vec<f32>,
    },

    // File operations
    FileList {
        files: Vec<String>,
    },
    FileContent {
        path: String,
        content: String,
    },
}

/// Detailed validation error for patch validation
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../modular_web/src/types/generated/")]
pub struct ValidationError {
    pub field: String,
    pub message: String,
    pub location: Option<String>,
}

impl ValidationError {
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            location: None,
        }
    }

    pub fn with_location(
        field: impl Into<String>,
        message: impl Into<String>,
        location: impl Into<String>,
    ) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
            location: Some(location.into()),
        }
    }
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref location) = self.location {
            write!(f, "{}: {} (at {})", self.field, self.message, location)
        } else {
            write!(f, "{}: {}", self.field, self.message)
        }
    }
}
