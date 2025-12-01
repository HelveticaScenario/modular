use std::{collections::HashMap, sync::mpsc, time::Duration};

use modular_client::http_client::spawn_client;
use modular_core::types::{ModuleState, Param, PatchGraph};
use modular_server::{protocol::InputMessage, run_server, ServerConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tokio::spawn(async {
        if let Err(e) = run_server(ServerConfig { port: 7812, patch_file: None }).await {
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

    println!("=== Pulse Oscillator Demo ===");
    println!("Demonstrating pulse width and PWM modulation");
    println!();

    // Helper to build basic patch
    let build_basic_patch = |note: u8, width: f32, pwm_cable: Option<(String, String)>| -> PatchGraph {
        let mut osc_params = HashMap::from([
            ("freq".to_string(), Param::Note { value: note }),
            ("width".to_string(), Param::Value { value: width }),
        ]);
        
        if let Some((module, port)) = pwm_cable {
            osc_params.insert("pwm".to_string(), Param::Cable { module, port });
        }
        
        PatchGraph {
            modules: vec![
                ModuleState {
                    id: "pulse-osc".to_string(),
                    module_type: "pulse-oscillator".to_string(),
                    params: osc_params,
                },
                ModuleState {
                    id: "amp".to_string(),
                    module_type: "scale-and-shift".to_string(),
                    params: HashMap::from([
                        ("input".to_string(), Param::Cable {
                            module: "pulse-osc".to_string(),
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

    println!("Part 1: Static Pulse Widths");
    
    println!("  Square wave (width=2.5, 50% duty cycle)");
    outgoing_tx.send(InputMessage::SetPatch {
        patch: build_basic_patch(57, 2.5, None),
    })?;
    tokio::time::sleep(Duration::from_millis(1200)).await;

    println!("  Narrow pulse (width=0.5, 10% duty cycle)");
    outgoing_tx.send(InputMessage::SetPatch {
        patch: build_basic_patch(57, 0.5, None),
    })?;
    tokio::time::sleep(Duration::from_millis(1200)).await;

    println!("  Wide pulse (width=4.5, 90% duty cycle)");
    outgoing_tx.send(InputMessage::SetPatch {
        patch: build_basic_patch(57, 4.5, None),
    })?;
    tokio::time::sleep(Duration::from_millis(1200)).await;

    println!();
    println!("Part 2: Pulse Width Sweep");
    println!("  Sweeping from narrow to wide pulse");
    for i in 0..100 {
        let width = 0.5 + (i as f32 / 100.0) * 4.0;
        outgoing_tx.send(InputMessage::SetPatch {
            patch: build_basic_patch(57, width, None),
        })?;
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    tokio::time::sleep(Duration::from_millis(500)).await;

    println!();
    println!("Part 3: PWM (Pulse Width Modulation) with LFO");
    
    // Build patch with PWM LFO
    let build_pwm_patch = |note: u8, lfo_freq: f32| -> PatchGraph {
        PatchGraph {
            modules: vec![
                ModuleState {
                    id: "pwm-lfo".to_string(),
                    module_type: "saw-oscillator".to_string(),
                    params: HashMap::from([
                        ("freq".to_string(), Param::Value { value: lfo_freq }),
                        ("shape".to_string(), Param::Value { value: 2.5 }), // Triangle
                    ]),
                },
                ModuleState {
                    id: "pulse-osc".to_string(),
                    module_type: "pulse-oscillator".to_string(),
                    params: HashMap::from([
                        ("freq".to_string(), Param::Note { value: note }),
                        ("width".to_string(), Param::Value { value: 2.5 }),
                        ("pwm".to_string(), Param::Cable {
                            module: "pwm-lfo".to_string(),
                            port: "output".to_string(),
                        }),
                    ]),
                },
                ModuleState {
                    id: "amp".to_string(),
                    module_type: "scale-and-shift".to_string(),
                    params: HashMap::from([
                        ("input".to_string(), Param::Cable {
                            module: "pulse-osc".to_string(),
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
    
    println!("  Playing with slow PWM modulation");
    outgoing_tx.send(InputMessage::SetPatch {
        patch: build_pwm_patch(57, -1.0), // ~13.75 Hz
    })?;
    tokio::time::sleep(Duration::from_millis(3000)).await;

    println!();
    println!("Part 4: Fast PWM (Creates chorus effect)");
    println!("  PWM LFO at ~84 Hz");
    outgoing_tx.send(InputMessage::SetPatch {
        patch: build_pwm_patch(57, 1.5), // ~84.43 Hz
    })?;
    tokio::time::sleep(Duration::from_millis(2000)).await;

    println!();
    println!("Part 5: PWM Melody");
    println!("  Playing melody with PWM");
    
    let melody = [57, 60, 64, 67, 64, 60, 57]; // A3, C4, E4, G4, E4, C4, A3
    
    for note in melody.iter() {
        outgoing_tx.send(InputMessage::SetPatch {
            patch: build_pwm_patch(*note, 0.0), // ~27.5 Hz
        })?;
        tokio::time::sleep(Duration::from_millis(400)).await;
    }

    tokio::time::sleep(Duration::from_millis(500)).await;

    println!();
    println!("Part 6: Chord with Different Pulse Widths");
    
    // Build three-voice chord patch
    let chord_patch = PatchGraph {
        modules: vec![
            ModuleState {
                id: "pulse-osc".to_string(),
                module_type: "pulse-oscillator".to_string(),
                params: HashMap::from([
                    ("freq".to_string(), Param::Note { value: 57 }), // A3
                    ("width".to_string(), Param::Value { value: 2.5 }), // Square
                ]),
            },
            ModuleState {
                id: "pulse-osc2".to_string(),
                module_type: "pulse-oscillator".to_string(),
                params: HashMap::from([
                    ("freq".to_string(), Param::Note { value: 60 }), // C4
                    ("width".to_string(), Param::Value { value: 1.0 }), // Narrow
                ]),
            },
            ModuleState {
                id: "pulse-osc3".to_string(),
                module_type: "pulse-oscillator".to_string(),
                params: HashMap::from([
                    ("freq".to_string(), Param::Note { value: 64 }), // E4
                    ("width".to_string(), Param::Value { value: 4.0 }), // Wide
                ]),
            },
            ModuleState {
                id: "amp".to_string(),
                module_type: "scale-and-shift".to_string(),
                params: HashMap::from([
                    ("input".to_string(), Param::Cable {
                        module: "pulse-osc".to_string(),
                        port: "output".to_string(),
                    }),
                    ("scale".to_string(), Param::Value { value: 3.0 }),
                ]),
            },
            ModuleState {
                id: "amp2".to_string(),
                module_type: "scale-and-shift".to_string(),
                params: HashMap::from([
                    ("input".to_string(), Param::Cable {
                        module: "pulse-osc2".to_string(),
                        port: "output".to_string(),
                    }),
                    ("scale".to_string(), Param::Value { value: 3.0 }),
                ]),
            },
            ModuleState {
                id: "amp3".to_string(),
                module_type: "scale-and-shift".to_string(),
                params: HashMap::from([
                    ("input".to_string(), Param::Cable {
                        module: "pulse-osc3".to_string(),
                        port: "output".to_string(),
                    }),
                    ("scale".to_string(), Param::Value { value: 3.0 }),
                ]),
            },
            ModuleState {
                id: "mixer".to_string(),
                module_type: "sum".to_string(),
                params: HashMap::from([
                    ("input-1".to_string(), Param::Cable {
                        module: "amp".to_string(),
                        port: "output".to_string(),
                    }),
                    ("input-2".to_string(), Param::Cable {
                        module: "amp2".to_string(),
                        port: "output".to_string(),
                    }),
                    ("input-3".to_string(), Param::Cable {
                        module: "amp3".to_string(),
                        port: "output".to_string(),
                    }),
                ]),
            },
            ModuleState {
                id: "root".to_string(),
                module_type: "signal".to_string(),
                params: HashMap::from([
                    ("source".to_string(), Param::Cable {
                        module: "mixer".to_string(),
                        port: "output".to_string(),
                    }),
                ]),
            },
        ],
    };
    
    println!("  Playing A minor chord (A3, C4, E4) with different pulse widths");
    outgoing_tx.send(InputMessage::SetPatch {
        patch: chord_patch,
    })?;
    tokio::time::sleep(Duration::from_millis(2500)).await;

    println!();
    println!("Demo complete!");
    tokio::time::sleep(Duration::from_millis(500)).await;

    Ok(())
}
