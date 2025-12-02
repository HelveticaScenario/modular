use modular_core::types::{Keyframe, ModuleSchema, PatchGraph, Track, TrackUpdate};
use serde::{Deserialize, Serialize};

/// Input messages from clients (YAML format)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum InputMessage {
    Echo { message: String },
    GetSchemas,
    GetPatch,
    
    // New declarative API - send complete desired state
    SetPatch { patch: PatchGraph },

    // Track operations
    GetTracks,
    GetTrack { id: String },
    CreateTrack { id: String },
    UpdateTrack { id: String, update: TrackUpdate },
    DeleteTrack { id: String },
    UpsertKeyframe { keyframe: Keyframe },
    DeleteKeyframe { track_id: String, keyframe_id: String },
    
    // Audio control
    SubscribeAudio { module_id: String, port: String, buffer_size: usize },
    UnsubscribeAudio { subscription_id: String },
    Mute,
    Unmute,
    
    // Recording
    StartRecording { filename: Option<String> },
    StopRecording,
}

/// Output messages to clients (YAML format)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum OutputMessage {
    Echo { message: String },
    Schemas { schemas: Vec<ModuleSchema> },
    PatchState { patch: PatchGraph },
    Track { track: Track },
    CreateTrack { id: String },
    Error { message: String, errors: Option<Vec<ValidationError>> },
    
    // Audio streaming
    AudioSubscribed { subscription_id: String },
    AudioBuffer { subscription_id: String, samples: Vec<f32> },
    
    // Audio control
    Muted,
    Unmuted,
    
    // Recording
    RecordingStarted { filename: String },
    RecordingStopped { filename: String },
}

/// Detailed validation error for patch validation
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    
    pub fn with_location(field: impl Into<String>, message: impl Into<String>, location: impl Into<String>) -> Self {
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

/// Serialize a message to YAML
pub fn serialize_message<T: Serialize>(message: &T) -> Result<String, serde_yaml::Error> {
    serde_yaml::to_string(message)
}

/// Deserialize a message from YAML
pub fn deserialize_message<T: for<'de> Deserialize<'de>>(yaml: &str) -> Result<T, serde_yaml::Error> {
    serde_yaml::from_str(yaml)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_serialize_input_message() {
        let msg = InputMessage::Echo { message: "hello".to_string() };
        let yaml = serialize_message(&msg).unwrap();
        assert!(yaml.contains("type: echo"));
        assert!(yaml.contains("message: hello"));
    }
    
    #[test]
    fn test_deserialize_input_message() {
        let yaml = "type: echo\nmessage: hello";
        let msg: InputMessage = deserialize_message(yaml).unwrap();
        match msg {
            InputMessage::Echo { message } => assert_eq!(message, "hello"),
            _ => panic!("Expected Echo message"),
        }
    }
    
    #[test]
    fn test_serialize_output_message() {
        let msg = OutputMessage::Muted;
        let yaml = serialize_message(&msg).unwrap();
        assert!(yaml.contains("type: muted"));
    }
    
    #[test]
    fn test_validation_error() {
        let error = ValidationError::with_location("modules.sine-1.type", "Unknown module type 'foo'", "modules.sine-1");
        let msg = OutputMessage::Error { 
            message: "Validation failed".to_string(),
            errors: Some(vec![error]),
        };
        let yaml = serialize_message(&msg).unwrap();
        assert!(yaml.contains("type: error"));
    }
    
    #[test]
    fn test_deserialize_set_patch() {
        let yaml = r#"
type: set-patch
patch:
  modules:
    - id: sine-1
      module_type: sine-oscillator
      params:
        freq:
          param_type: value
          value: 4.0
"#;
        let msg: InputMessage = deserialize_message(yaml).unwrap();
        match msg {
            InputMessage::SetPatch { patch } => {
                assert_eq!(patch.modules.len(), 1);
            }
            _ => panic!("Expected SetPatch message"),
        }
    }
    
    #[test]
    fn test_yaml_parse_mute() {
        let yaml = "type: mute\n";
        let msg: InputMessage = deserialize_message(yaml).unwrap();
        assert!(matches!(msg, InputMessage::Mute));
    }

    #[test]
    fn test_yaml_parse_subscribe_audio() {
        let yaml = r#"
type: subscribe-audio
module_id: sine-1
port: output
buffer_size: 512
"#;
        let msg: InputMessage = deserialize_message(yaml).unwrap();
        match msg {
            InputMessage::SubscribeAudio { module_id, port, buffer_size } => {
                assert_eq!(module_id, "sine-1");
                assert_eq!(port, "output");
                assert_eq!(buffer_size, 512);
            }
            _ => panic!("Expected SubscribeAudio message"),
        }
    }
}
