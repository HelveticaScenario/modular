//! Example: Simple sine wave using declarative SetPatch API
//! 
//! This example demonstrates the new declarative patch API where
//! you send the complete desired state rather than imperative commands.

use std::collections::HashMap;
use std::sync::mpsc;
use std::time::Duration;

use modular_client::http_client::spawn_client;
use modular_core::types::{ModuleState, Param, PatchGraph};
use modular_server::{
    protocol::{InputMessage, OutputMessage},
    run_server, ServerConfig,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Spawn the server
    tokio::spawn(async {
        let config = ServerConfig { port: 7812, patch_file: None };
        if let Err(e) = run_server(config).await {
            eprintln!("Server error: {}", e);
        }
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(1000)).await;

    let (incoming_tx, incoming_rx) = mpsc::channel();
    let (outgoing_tx, outgoing_rx) = mpsc::channel();

    let (_receiving_client_handle, _sending_client_handle) = spawn_client(
        "127.0.0.1:7812".to_owned(),
        incoming_tx,
        outgoing_rx,
    );

    // Give client time to connect
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Build patch helper function
    let build_patch = |note: u8, amp: f32| -> PatchGraph {
        PatchGraph {
            modules: vec![
                ModuleState {
                    id: "osc".to_string(),
                    module_type: "sine-oscillator".to_string(),
                    params: HashMap::from([
                        ("freq".to_string(), Param::Note { value: note }),
                    ]),
                },
                ModuleState {
                    id: "amp".to_string(),
                    module_type: "scale-and-shift".to_string(),
                    params: HashMap::from([
                        ("input".to_string(), Param::Cable {
                            module: "osc".to_string(),
                            port: "output".to_string(),
                        }),
                        ("scale".to_string(), Param::Value { value: amp }),
                    ]),
                },
                ModuleState {
                    id: "root".to_string(),
                    module_type: "signal".to_string(),
                    params: HashMap::from([
                        ("source".to_string(), Param::Cable {
                            module: "amp".to_string(),
                            port: "output".to_string(),
                        }),
                    ]),
                },
            ],
        }
    };

    // Send initial patch
    outgoing_tx.send(InputMessage::SetPatch {
        patch: build_patch(69, 5.0), // A4
    })?;

    // Wait for response
    match incoming_rx.recv_timeout(Duration::from_millis(500)) {
        Ok(OutputMessage::PatchState { patch }) => {
            println!("Patch initialized with {} modules", patch.modules.len());
        }
        Ok(msg) => println!("Unexpected response: {:?}", msg),
        Err(e) => println!("No response: {}", e),
    }

    // Play a simple melody
    const A: u8 = 69;
    const B: u8 = 67;
    const C: u8 = 65;
    let melody = [A, B, C, A, B, C, A, B, C];
    
    for note in melody.iter() {
        outgoing_tx.send(InputMessage::SetPatch {
            patch: build_patch(*note, 5.0),
        })?;
        tokio::time::sleep(Duration::from_millis(400)).await;
    }

    tokio::time::sleep(Duration::from_millis(2000)).await;
    println!("Melody complete!");
    
    Ok(())
}
