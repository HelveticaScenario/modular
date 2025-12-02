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

    // Additional protocol tests
    #[test]
    fn test_yaml_parse_get_schemas() {
        let yaml = "type: get-schemas\n";
        let msg: InputMessage = deserialize_message(yaml).unwrap();
        assert!(matches!(msg, InputMessage::GetSchemas));
    }

    #[test]
    fn test_yaml_parse_get_patch() {
        let yaml = "type: get-patch\n";
        let msg: InputMessage = deserialize_message(yaml).unwrap();
        assert!(matches!(msg, InputMessage::GetPatch));
    }

    #[test]
    fn test_yaml_parse_unmute() {
        let yaml = "type: unmute\n";
        let msg: InputMessage = deserialize_message(yaml).unwrap();
        assert!(matches!(msg, InputMessage::Unmute));
    }

    #[test]
    fn test_yaml_parse_unsubscribe_audio() {
        let yaml = r#"
type: unsubscribe-audio
subscription_id: sub-123
"#;
        let msg: InputMessage = deserialize_message(yaml).unwrap();
        match msg {
            InputMessage::UnsubscribeAudio { subscription_id } => {
                assert_eq!(subscription_id, "sub-123");
            }
            _ => panic!("Expected UnsubscribeAudio message"),
        }
    }

    #[test]
    fn test_yaml_parse_start_recording() {
        let yaml = r#"
type: start-recording
filename: test.wav
"#;
        let msg: InputMessage = deserialize_message(yaml).unwrap();
        match msg {
            InputMessage::StartRecording { filename } => {
                assert_eq!(filename, Some("test.wav".to_string()));
            }
            _ => panic!("Expected StartRecording message"),
        }
    }

    #[test]
    fn test_yaml_parse_start_recording_no_filename() {
        let yaml = "type: start-recording\n";
        let msg: InputMessage = deserialize_message(yaml).unwrap();
        match msg {
            InputMessage::StartRecording { filename } => {
                assert!(filename.is_none());
            }
            _ => panic!("Expected StartRecording message"),
        }
    }

    #[test]
    fn test_yaml_parse_stop_recording() {
        let yaml = "type: stop-recording\n";
        let msg: InputMessage = deserialize_message(yaml).unwrap();
        assert!(matches!(msg, InputMessage::StopRecording));
    }

    #[test]
    fn test_yaml_parse_create_track() {
        let yaml = r#"
type: create-track
id: track-1
"#;
        let msg: InputMessage = deserialize_message(yaml).unwrap();
        match msg {
            InputMessage::CreateTrack { id } => {
                assert_eq!(id, "track-1");
            }
            _ => panic!("Expected CreateTrack message"),
        }
    }

    #[test]
    fn test_yaml_parse_get_tracks() {
        let yaml = "type: get-tracks\n";
        let msg: InputMessage = deserialize_message(yaml).unwrap();
        assert!(matches!(msg, InputMessage::GetTracks));
    }

    #[test]
    fn test_yaml_parse_get_track() {
        let yaml = r#"
type: get-track
id: my-track
"#;
        let msg: InputMessage = deserialize_message(yaml).unwrap();
        match msg {
            InputMessage::GetTrack { id } => {
                assert_eq!(id, "my-track");
            }
            _ => panic!("Expected GetTrack message"),
        }
    }

    #[test]
    fn test_yaml_parse_delete_track() {
        let yaml = r#"
type: delete-track
id: track-to-delete
"#;
        let msg: InputMessage = deserialize_message(yaml).unwrap();
        match msg {
            InputMessage::DeleteTrack { id } => {
                assert_eq!(id, "track-to-delete");
            }
            _ => panic!("Expected DeleteTrack message"),
        }
    }

    // Output message tests
    #[test]
    fn test_serialize_unmuted() {
        let msg = OutputMessage::Unmuted;
        let yaml = serialize_message(&msg).unwrap();
        assert!(yaml.contains("type: unmuted"));
    }

    #[test]
    fn test_serialize_audio_subscribed() {
        let msg = OutputMessage::AudioSubscribed { subscription_id: "sub-456".to_string() };
        let yaml = serialize_message(&msg).unwrap();
        assert!(yaml.contains("type: audio-subscribed"));
        assert!(yaml.contains("subscription_id: sub-456"));
    }

    #[test]
    fn test_serialize_audio_buffer() {
        let msg = OutputMessage::AudioBuffer { 
            subscription_id: "sub-1".to_string(), 
            samples: vec![0.1, 0.2, 0.3] 
        };
        let yaml = serialize_message(&msg).unwrap();
        assert!(yaml.contains("type: audio-buffer"));
        assert!(yaml.contains("subscription_id: sub-1"));
    }

    #[test]
    fn test_serialize_recording_started() {
        let msg = OutputMessage::RecordingStarted { filename: "output.wav".to_string() };
        let yaml = serialize_message(&msg).unwrap();
        assert!(yaml.contains("type: recording-started"));
        assert!(yaml.contains("filename: output.wav"));
    }

    #[test]
    fn test_serialize_recording_stopped() {
        let msg = OutputMessage::RecordingStopped { filename: "output.wav".to_string() };
        let yaml = serialize_message(&msg).unwrap();
        assert!(yaml.contains("type: recording-stopped"));
        assert!(yaml.contains("filename: output.wav"));
    }

    #[test]
    fn test_serialize_error_without_details() {
        let msg = OutputMessage::Error { 
            message: "Something went wrong".to_string(),
            errors: None,
        };
        let yaml = serialize_message(&msg).unwrap();
        assert!(yaml.contains("type: error"));
        assert!(yaml.contains("message: Something went wrong"));
    }

    #[test]
    fn test_serialize_error_with_multiple_errors() {
        let errors = vec![
            ValidationError::new("field1", "error 1"),
            ValidationError::with_location("field2", "error 2", "modules.test"),
        ];
        let msg = OutputMessage::Error { 
            message: "Multiple errors".to_string(),
            errors: Some(errors),
        };
        let yaml = serialize_message(&msg).unwrap();
        assert!(yaml.contains("type: error"));
        assert!(yaml.contains("field1"));
        assert!(yaml.contains("field2"));
    }

    // ValidationError tests
    #[test]
    fn test_validation_error_new() {
        let err = ValidationError::new("param", "Invalid value");
        assert_eq!(err.field, "param");
        assert_eq!(err.message, "Invalid value");
        assert!(err.location.is_none());
    }

    #[test]
    fn test_validation_error_with_location() {
        let err = ValidationError::with_location("module_type", "Unknown type", "modules.sine-1");
        assert_eq!(err.field, "module_type");
        assert_eq!(err.message, "Unknown type");
        assert_eq!(err.location, Some("modules.sine-1".to_string()));
    }

    #[test]
    fn test_validation_error_display_without_location() {
        let err = ValidationError::new("field", "error message");
        let display = format!("{}", err);
        assert_eq!(display, "field: error message");
    }

    #[test]
    fn test_validation_error_display_with_location() {
        let err = ValidationError::with_location("field", "error message", "loc");
        let display = format!("{}", err);
        assert_eq!(display, "field: error message (at loc)");
    }

    // Complex patch serialization tests
    #[test]
    fn test_set_patch_with_cables() {
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
    - id: root
      module_type: signal
      params:
        source:
          param_type: cable
          module: sine-1
          port: output
"#;
        let msg: InputMessage = deserialize_message(yaml).unwrap();
        match msg {
            InputMessage::SetPatch { patch } => {
                assert_eq!(patch.modules.len(), 2);
                let root = patch.modules.iter().find(|m| m.id == "root").unwrap();
                match root.params.get("source") {
                    Some(modular_core::types::Param::Cable { module, port }) => {
                        assert_eq!(module, "sine-1");
                        assert_eq!(port, "output");
                    }
                    _ => panic!("Expected cable param"),
                }
            }
            _ => panic!("Expected SetPatch message"),
        }
    }

    #[test]
    fn test_invalid_yaml_returns_error() {
        let yaml = "invalid: yaml: {[";
        let result: Result<InputMessage, _> = deserialize_message(yaml);
        assert!(result.is_err());
    }

    #[test]
    fn test_unknown_message_type() {
        let yaml = "type: unknown-type\n";
        let result: Result<InputMessage, _> = deserialize_message(yaml);
        assert!(result.is_err());
    }
}
