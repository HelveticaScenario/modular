// Example: Create a sine wave for oscilloscope testing
use anyhow::Result;
use modular_client::http_client;
use modular_core::message::{InputMessage, OutputMessage};
use modular_core::types::{Param, PatchGraph, ModuleState};
use std::collections::HashMap;
use std::sync::mpsc::channel;
use std::thread::sleep;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<()> {
    let server_address = "localhost:7812";
    let (tx, rx) = channel();
    let (input_tx, input_rx) = channel();

    // Spawn HTTP client tasks
    let (_recv_handle, _send_handle) =
        http_client::spawn_client(server_address.to_string(), tx, input_rx);

    println!("Creating sine wave oscillator for oscilloscope...");
    sleep(Duration::from_millis(500));

    // Create a sine wave module
    let sine_module = ModuleState {
        id: "sine-1".to_string(),
        module_type: "sine-oscillator".to_string(),
        params: {
            let mut map = HashMap::new();
            // freq parameter expects voltage (v/oct), where 4.0v = 440Hz
            map.insert("freq".to_string(), Param::Value { value: 4.0 });
            map
        },
    };

    // Connect sine to root output
    let root_module = ModuleState {
        id: "root".to_string(),
        module_type: "signal".to_string(),
        params: {
            let mut map = HashMap::new();
            map.insert(
                "source".to_string(),
                Param::Cable {
                    module: "sine-1".to_string(),
                    port: "output".to_string(),
                },
            );
            map
        },
    };

    // Send complete patch
    input_tx.send(InputMessage::SetPatch {
        graph: PatchGraph {
            modules: vec![sine_module, root_module],
        },
    })?;

    println!("✓ Sine oscillator created at 440Hz (4.0v)");
    println!("✓ Connected to root output");
    println!("\nYou can now:");
    println!("  1. Open oscilloscope.html in your browser");
    println!("  2. Connect to ws://localhost:7812/ws");
    println!("  3. Module ID: 'sine-1' or 'root'");
    println!("  4. Port: 'output'");
    println!("  5. Click 'Subscribe to Audio'");
    println!("\nPress Ctrl+C to exit...");

    // Monitor responses
    for response in rx {
        match response {
            OutputMessage::PatchState { modules } => {
                println!("Patch updated with {} modules", modules.len());
            }
            OutputMessage::Error { message } => {
                eprintln!("Error: {}", message);
            }
            other => {
                println!("Response: {:?}", other);
            }
        }
    }

    Ok(())
}
