use std::{collections::HashMap, sync::mpsc, time::Duration};

use modular_client::http_client::spawn_client;
use modular_core::{
    message::InputMessage,
    types::{ModuleState, Param, PatchGraph},
};
use modular_server::run_server;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tokio::spawn(async {
        if let Err(e) = run_server(7812).await {
            eprintln!("Server error: {}", e);
        }
    });
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let (incoming_tx, _incoming_rx) = mpsc::channel();
    let (outgoing_tx, outgoing_rx) = mpsc::channel();

    let (_receiving_client_handle, _sending_client_handle) = spawn_client(
        "127.0.0.1:7812".to_owned(),
        incoming_tx,
        outgoing_rx,
    );

    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    println!("=== Saw Oscillator Demo ===");
    println!("Demonstrating shape morphing and phase sync");
    println!();

    // Helper to build basic patch
    let build_basic_patch = |note: u8, shape: f32, phase_cable: Option<(String, String)>| -> PatchGraph {
        let mut osc_params = HashMap::from([
            ("freq".to_string(), Param::Note { value: note }),
            ("shape".to_string(), Param::Value { value: shape }),
        ]);
        
        if let Some((module, port)) = phase_cable {
            osc_params.insert("phase".to_string(), Param::Cable { module, port });
        }
        
        PatchGraph {
            modules: vec![
                ModuleState {
                    id: "main-osc".to_string(),
                    module_type: "saw-oscillator".to_string(),
                    params: osc_params,
                },
                ModuleState {
                    id: "amp".to_string(),
                    module_type: "scale-and-shift".to_string(),
                    params: HashMap::from([
                        ("input".to_string(), Param::Cable {
                            module: "main-osc".to_string(),
                            port: "output".to_string(),
                        }),
                        ("scale".to_string(), Param::Value { value: 3.0 }),
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

    println!("Part 1: Shape Morphing");
    println!("  Playing pure sawtooth (shape=0)");
    outgoing_tx.send(InputMessage::SetPatch {
        graph: build_basic_patch(57, 0.0, None),
    })?;
    tokio::time::sleep(Duration::from_millis(1500)).await;

    println!("  Morphing to triangle (shape=2.5)");
    outgoing_tx.send(InputMessage::SetPatch {
        graph: build_basic_patch(57, 2.5, None),
    })?;
    tokio::time::sleep(Duration::from_millis(1500)).await;

    println!("  Morphing to ramp/reverse saw (shape=5)");
    outgoing_tx.send(InputMessage::SetPatch {
        graph: build_basic_patch(57, 5.0, None),
    })?;
    tokio::time::sleep(Duration::from_millis(1500)).await;

    println!("  Sweeping through all shapes");
    for i in 0..50 {
        let shape = (i as f32 / 50.0) * 5.0;
        outgoing_tx.send(InputMessage::SetPatch {
            graph: build_basic_patch(57, shape, None),
        })?;
        tokio::time::sleep(Duration::from_millis(40)).await;
    }
    tokio::time::sleep(Duration::from_millis(500)).await;

    println!();
    println!("Part 2: Melodic Sequence with Different Shapes");
    let melody = [57, 60, 64, 67, 72]; // A3, C4, E4, G4, C5
    let shapes = [0.0, 1.25, 2.5, 3.75, 5.0];
    
    for (note, shape) in melody.iter().zip(shapes.iter()) {
        outgoing_tx.send(InputMessage::SetPatch {
            graph: build_basic_patch(*note, *shape, None),
        })?;
        tokio::time::sleep(Duration::from_millis(400)).await;
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    println!();
    println!("Part 3: Phase Sync Demo");
    println!("  Creating LFO to control phase");
    
    // Build patch with LFO for phase sync
    let build_phase_sync_patch = |note: u8, shape: f32| -> PatchGraph {
        PatchGraph {
            modules: vec![
                ModuleState {
                    id: "lfo".to_string(),
                    module_type: "saw-oscillator".to_string(),
                    params: HashMap::from([
                        ("freq".to_string(), Param::Value { value: -3.0 }), // ~3.4 Hz
                        ("shape".to_string(), Param::Value { value: 2.5 }), // Triangle
                    ]),
                },
                ModuleState {
                    id: "lfo-scale".to_string(),
                    module_type: "scale-and-shift".to_string(),
                    params: HashMap::from([
                        ("input".to_string(), Param::Cable {
                            module: "lfo".to_string(),
                            port: "output".to_string(),
                        }),
                        ("scale".to_string(), Param::Value { value: 0.1 }),
                        ("shift".to_string(), Param::Value { value: 2.5 }),
                    ]),
                },
                ModuleState {
                    id: "main-osc".to_string(),
                    module_type: "saw-oscillator".to_string(),
                    params: HashMap::from([
                        ("freq".to_string(), Param::Note { value: note }),
                        ("shape".to_string(), Param::Value { value: shape }),
                        ("phase".to_string(), Param::Cable {
                            module: "lfo-scale".to_string(),
                            port: "output".to_string(),
                        }),
                    ]),
                },
                ModuleState {
                    id: "amp".to_string(),
                    module_type: "scale-and-shift".to_string(),
                    params: HashMap::from([
                        ("input".to_string(), Param::Cable {
                            module: "main-osc".to_string(),
                            port: "output".to_string(),
                        }),
                        ("scale".to_string(), Param::Value { value: 3.0 }),
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
    
    outgoing_tx.send(InputMessage::SetPatch {
        graph: build_phase_sync_patch(69, 0.0),
    })?;
    
    println!("  Oscillator is now synced to LFO phase");
    println!("  Playing notes while phase is being modulated");
    
    for _ in 0..3 {
        tokio::time::sleep(Duration::from_millis(1000)).await;
    }

    println!();
    println!("Demo complete!");
    tokio::time::sleep(Duration::from_millis(500)).await;

    Ok(())
}
