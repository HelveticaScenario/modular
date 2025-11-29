use std::collections::HashMap;
use std::sync::mpsc;
use std::time::Duration;

use modular_client::http_client::spawn_client;
use modular_core::{
    message::{InputMessage, OutputMessage},
    types::{ModuleState, Param, PatchGraph},
};
use modular_server::run_server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Spawn the server
    tokio::spawn(async {
        if let Err(e) = run_server(7812).await {
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

    // Build the initial patch graph declaratively
    let build_patch = |note: u8, amp: f32| -> PatchGraph {
        let mut modules = Vec::new();

        // Oscillator
        modules.push(ModuleState {
            id: "osc".to_string(),
            module_type: "sine-oscillator".to_string(),
            params: HashMap::from([
                ("freq".to_string(), Param::Cable {
                    module: "freq-sum".to_string(),
                    port: "output".to_string(),
                }),
            ]),
        });

        // Vibrato (modulates frequency)
        modules.push(ModuleState {
            id: "vibrato".to_string(),
            module_type: "scale-and-shift".to_string(),
            params: HashMap::from([
                ("input".to_string(), Param::Cable {
                    module: "osc".to_string(),
                    port: "output".to_string(),
                }),
                ("scale".to_string(), Param::Value { value: 1.0 }),
            ]),
        });

        // Sum (adds vibrato to base note)
        modules.push(ModuleState {
            id: "freq-sum".to_string(),
            module_type: "sum".to_string(),
            params: HashMap::from([
                ("input-1".to_string(), Param::Cable {
                    module: "vibrato".to_string(),
                    port: "output".to_string(),
                }),
                ("input-2".to_string(), Param::Note { value: note }),
            ]),
        });

        // Amplifier
        modules.push(ModuleState {
            id: "amp".to_string(),
            module_type: "scale-and-shift".to_string(),
            params: HashMap::from([
                ("input".to_string(), Param::Cable {
                    module: "osc".to_string(),
                    port: "output".to_string(),
                }),
                ("scale".to_string(), Param::Value { value: amp }),
            ]),
        });

        // Root (audio output)
        modules.push(ModuleState {
            id: "root".to_string(),
            module_type: "signal".to_string(),
            params: HashMap::from([
                ("source".to_string(), Param::Cable {
                    module: "amp".to_string(),
                    port: "output".to_string(),
                }),
            ]),
        });

        PatchGraph { modules }
    };

    // Set initial patch
    outgoing_tx.send(InputMessage::SetPatch {
        graph: build_patch(69, 5.0),
    })?;

    // Wait for response
    match incoming_rx.recv_timeout(Duration::from_millis(500)) {
        Ok(OutputMessage::PatchState { modules }) => {
            println!("Patch initialized with {} modules", modules.len());
        }
        Ok(msg) => println!("Unexpected response: {:?}", msg),
        Err(e) => println!("No response: {}", e),
    }

    // Play melody by sending complete patch updates
    const A: u8 = 69;
    const B: u8 = 67;
    const C: u8 = 65;
    let part1 = [A, B, C];
    
    for _ in 0..2 {
        for note in part1.iter() {
            outgoing_tx.send(InputMessage::SetPatch {
                graph: build_patch(*note, 5.0),
            })?;
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    let part2 = [C, C, C, C, B, B, B, B];
    for note in part2.iter() {
        // Accent pattern with amplitude
        outgoing_tx.send(InputMessage::SetPatch {
            graph: build_patch(*note, 5.0),
        })?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        outgoing_tx.send(InputMessage::SetPatch {
            graph: build_patch(*note, 4.0),
        })?;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    for note in part1.iter() {
        outgoing_tx.send(InputMessage::SetPatch {
            graph: build_patch(*note, 5.0),
        })?;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    tokio::time::sleep(Duration::from_millis(3000)).await;
    
    println!("Melody complete!");
    Ok(())
}
