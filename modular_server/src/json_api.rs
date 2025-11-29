use modular_core::message::{InputMessage, OutputMessage};
use serde_json;

pub fn serialize_output_message(message: &OutputMessage) -> String {
    serde_json::to_string(message).unwrap_or_else(|_| {
        serde_json::json!({
            "type": "Error",
            "message": "Failed to serialize message"
        })
        .to_string()
    })
}

pub fn deserialize_input_message(json: &str) -> Result<InputMessage, String> {
    serde_json::from_str(json).map_err(|e| format!("Failed to parse JSON: {}", e))
}
