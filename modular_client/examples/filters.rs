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

    println!("=== Filter Showcase ===");
    println!("Demonstrating key filter types");
    println!();

    // Helper to build a patch with oscillator, filter, and amp
    let build_patch = |filter_type: &str, filter_id: &str, filter_params: HashMap<String, Param>| -> PatchGraph {
        let modules = vec![
            // Sawtooth oscillator
            ModuleState {
                id: "osc".to_string(),
                module_type: "saw-oscillator".to_string(),
                params: HashMap::from([
                    ("freq".to_string(), Param::Note { value: 57 }), // A3
                    ("shape".to_string(), Param::Value { value: 0.0 }), // Pure saw
                ]),
            },
            // Filter
            ModuleState {
                id: filter_id.to_string(),
                module_type: filter_type.to_string(),
                params: filter_params,
            },
            // Output amp
            ModuleState {
                id: "amp".to_string(),
                module_type: "scale-and-shift".to_string(),
                params: HashMap::from([
                    ("input".to_string(), Param::Cable {
                        module: filter_id.to_string(),
                        port: "output".to_string(),
                    }),
                    ("scale".to_string(), Param::Value { value: 3.0 }),
                ]),
            },
            // Root output
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
        ];
        PatchGraph { modules }
    };

    // Demo 1: Moog Ladder
    println!("1. MOOG LADDER FILTER");
    println!("   24dB/oct with self-oscillation");
    sweep_filter("moog-ladder-filter", "moog", HashMap::from([
        ("input".to_string(), Param::Cable { module: "osc".to_string(), port: "output".to_string() }),
        ("resonance".to_string(), Param::Value { value: 4.2 }),
    ]), &build_patch, &outgoing_tx).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Demo 2: TB-303
    println!("\n2. TB-303 FILTER");
    println!("   Classic acid bass sound");
    sweep_filter("tb303-filter", "tb303", HashMap::from([
        ("input".to_string(), Param::Cable { module: "osc".to_string(), port: "output".to_string() }),
        ("resonance".to_string(), Param::Value { value: 4.0 }),
    ]), &build_patch, &outgoing_tx).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Demo 3: MS-20
    println!("\n3. MS-20 FILTER");
    println!("   Aggressive with hard clipping");
    sweep_filter("ms20-filter", "ms20", HashMap::from([
        ("input".to_string(), Param::Cable { module: "osc".to_string(), port: "output".to_string() }),
        ("resonance".to_string(), Param::Value { value: 4.5 }),
    ]), &build_patch, &outgoing_tx).await?;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Demo 4: SEM with mode morphing
    println!("\n4. SEM FILTER");
    println!("   Mode morphing: LP -> BP -> HP -> Notch");
    for i in 0..4 {
        let mode = i as f32 * 1.25;
        let params = HashMap::from([
            ("input".to_string(), Param::Cable { module: "osc".to_string(), port: "output".to_string() }),
            ("resonance".to_string(), Param::Value { value: 3.0 }),
            ("cutoff".to_string(), Param::Value { value: 5.5 }),
            ("mode".to_string(), Param::Value { value: mode }),
        ]);
        outgoing_tx.send(InputMessage::SetPatch {
            patch: build_patch("sem-filter", "sem", params),
        })?;
        tokio::time::sleep(Duration::from_millis(600)).await;
    }
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Demo 5: Formant filter
    println!("\n5. FORMANT FILTER");
    println!("   Vowel sounds: A -> E -> I -> O -> U");
    let vowels = ["A", "E", "I", "O", "U"];
    for (i, vowel) in vowels.iter().enumerate() {
        println!("   {}", vowel);
        let params = HashMap::from([
            ("input".to_string(), Param::Cable { module: "osc".to_string(), port: "output".to_string() }),
            ("vowel".to_string(), Param::Value { value: i as f32 }),
        ]);
        outgoing_tx.send(InputMessage::SetPatch {
            patch: build_patch("formant-filter", "formant", params),
        })?;
        tokio::time::sleep(Duration::from_millis(700)).await;
    }
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Demo 6: State Variable (using lowpass output)
    println!("\n6. STATE VARIABLE FILTER");
    println!("   Multi-output filter (using lowpass)");
    // Special case: need to wire to "lowpass" output instead of "output"
    let svf_patch = {
        let modules = vec![
            ModuleState {
                id: "osc".to_string(),
                module_type: "saw-oscillator".to_string(),
                params: HashMap::from([
                    ("freq".to_string(), Param::Note { value: 57 }),
                    ("shape".to_string(), Param::Value { value: 0.0 }),
                ]),
            },
            ModuleState {
                id: "svf".to_string(),
                module_type: "state-variable-filter".to_string(),
                params: HashMap::from([
                    ("input".to_string(), Param::Cable { module: "osc".to_string(), port: "output".to_string() }),
                    ("resonance".to_string(), Param::Value { value: 3.5 }),
                ]),
            },
            ModuleState {
                id: "amp".to_string(),
                module_type: "scale-and-shift".to_string(),
                params: HashMap::from([
                    ("input".to_string(), Param::Cable {
                        module: "svf".to_string(),
                        port: "lowpass".to_string(),
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
        ];
        PatchGraph { modules }
    };
    
    for i in 0..40 {
        let cutoff = 2.5 + (i as f32 / 40.0) * 4.0;
        let mut patch = svf_patch.clone();
        patch.modules[1].params.insert("cutoff".to_string(), Param::Value { value: cutoff });
        outgoing_tx.send(InputMessage::SetPatch { patch: patch })?;
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Demo 7: Sallen-Key with mode changes
    println!("\n7. SALLEN-KEY FILTER");
    println!("   Smooth analog-style response");
    
    println!("   Mode: Lowpass");
    sweep_filter("sallen-key-filter", "sk", HashMap::from([
        ("input".to_string(), Param::Cable { module: "osc".to_string(), port: "output".to_string() }),
        ("resonance".to_string(), Param::Value { value: 2.5 }),
        ("mode".to_string(), Param::Value { value: 0.0 }),
    ]), &build_patch, &outgoing_tx).await?;
    tokio::time::sleep(Duration::from_millis(300)).await;
    
    println!("   Mode: Highpass");
    sweep_filter("sallen-key-filter", "sk", HashMap::from([
        ("input".to_string(), Param::Cable { module: "osc".to_string(), port: "output".to_string() }),
        ("resonance".to_string(), Param::Value { value: 2.5 }),
        ("mode".to_string(), Param::Value { value: 2.5 }),
    ]), &build_patch, &outgoing_tx).await?;
    tokio::time::sleep(Duration::from_millis(300)).await;
    
    println!("   Mode: Bandpass");
    sweep_filter("sallen-key-filter", "sk", HashMap::from([
        ("input".to_string(), Param::Cable { module: "osc".to_string(), port: "output".to_string() }),
        ("resonance".to_string(), Param::Value { value: 2.5 }),
        ("mode".to_string(), Param::Value { value: 5.0 }),
    ]), &build_patch, &outgoing_tx).await?;

    println!("\nDemo complete!");
    tokio::time::sleep(Duration::from_millis(500)).await;

    Ok(())
}

async fn sweep_filter(
    filter_type: &str,
    filter_id: &str,
    mut base_params: HashMap<String, Param>,
    build_patch: &impl Fn(&str, &str, HashMap<String, Param>) -> PatchGraph,
    outgoing_tx: &mpsc::Sender<InputMessage>,
) -> anyhow::Result<()> {
    for i in 0..40 {
        let cutoff = 2.5 + (i as f32 / 40.0) * 4.0;
        base_params.insert("cutoff".to_string(), Param::Value { value: cutoff });
        outgoing_tx.send(InputMessage::SetPatch {
            patch: build_patch(filter_type, filter_id, base_params.clone()),
        })?;
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
    Ok(())
}
