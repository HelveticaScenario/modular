use std::{collections::HashMap, path::PathBuf, sync::{mpsc, Arc, Mutex}, time::Duration};

use modular_client::http_client::spawn_client;
use modular_core::types::{ModuleState, Param, PatchGraph};
use modular_server::{
    protocol::InputMessage,
    run_server, ServerConfig,
};
use notify::{Watcher, RecursiveMode, Result as NotifyResult};
use notify::event::{EventKind, ModifyKind};
use serde::{Deserialize, Serialize};

// File format with modules as an object (keys are IDs)
#[derive(Deserialize, Serialize)]
struct FilePatchGraph {
    modules: HashMap<String, FileModuleState>,
}

#[derive(Deserialize, Serialize)]
struct FileModuleState {
    module_type: String,
    params: HashMap<String, Param>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Get patch file path from command line or use default
    let args: Vec<String> = std::env::args().collect();
    let patch_file = if args.len() > 1 {
        PathBuf::from(&args[1])
    } else {
        PathBuf::from("patch.yaml")
    };

    if !patch_file.exists() {
        eprintln!("Patch file not found: {}", patch_file.display());
        eprintln!("Usage: cargo run --example watch_patch [path/to/patch.yaml]");
        eprintln!("\nCreating example patch.yaml file...");
        create_example_patch(&patch_file)?;
        println!("Created {}", patch_file.display());
    }

    println!("=== Patch File Watcher ===");
    println!("Watching: {}", patch_file.display());
    println!("Edit the file to update the patch in real-time");
    println!();

    // Spawn the server
    tokio::spawn(async {
        let config = ServerConfig { port: 7812, patch_file: None };
        if let Err(e) = run_server(config).await {
            eprintln!("Server error: {}", e);
        }
    });

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(500)).await;

    let (incoming_tx, _incoming_rx) = mpsc::channel();
    let (outgoing_tx, outgoing_rx) = mpsc::channel();

    let (_receiving_client_handle, _sending_client_handle) = spawn_client(
        "127.0.0.1:7812".to_owned(),
        incoming_tx,
        outgoing_rx,
    );

    tokio::time::sleep(Duration::from_millis(100)).await;

    // Load and apply initial patch
    println!("Loading initial patch...");
    match load_and_apply_patch(&patch_file, &outgoing_tx) {
        Ok(_) => println!("✓ Initial patch loaded successfully"),
        Err(e) => eprintln!("✗ Failed to load initial patch: {}", e),
    }
    println!();

    // Set up file watcher
    let (tx, rx) = std::sync::mpsc::channel();
    let rx = Arc::new(Mutex::new(rx));
    
    let mut watcher = notify::recommended_watcher(move |res: NotifyResult<notify::Event>| {
        if let Ok(event) = res {
            // Only trigger on modify events
            if matches!(event.kind, EventKind::Modify(ModifyKind::Data(_))) {
                let _ = tx.send(());
            }
        }
    })?;

    watcher.watch(&patch_file, RecursiveMode::NonRecursive)?;

    println!("Watching for changes... (Press Ctrl+C to exit)");
    println!();

    // Watch for file changes
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!("\nShutting down...");
                break;
            }
            recv_result = tokio::task::spawn_blocking({
                let rx = Arc::clone(&rx);
                move || {
                    let rx = rx.lock().unwrap();
                    rx.recv()
                }
            }) => {
                if recv_result.is_ok() {
                    println!("File changed, reloading patch...");
                    // Small delay to ensure file write is complete
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    
                    match load_and_apply_patch(&patch_file, &outgoing_tx) {
                        Ok(_) => println!("✓ Patch reloaded successfully"),
                        Err(e) => eprintln!("✗ Failed to reload patch: {}", e),
                    }
                    println!();
                }
            }
        }
    }

    Ok(())
}

fn load_and_apply_patch(
    path: &PathBuf,
    outgoing_tx: &mpsc::Sender<InputMessage>,
) -> anyhow::Result<()> {
    let contents = std::fs::read_to_string(path)?;
    let file_graph: FilePatchGraph = serde_yaml::from_str(&contents)?;
    
    // Convert from file format (object with ID keys) to internal format (array with id field)
    let graph = PatchGraph {
        modules: file_graph.modules.into_iter().map(|(id, state)| {
            ModuleState {
                id,
                module_type: state.module_type,
                params: state.params,
            }
        }).collect(),
    };
    
    println!("  Loaded {} modules", graph.modules.len());
    for module in &graph.modules {
        println!("    - {} ({})", module.id, module.module_type);
    }
    
    outgoing_tx.send(InputMessage::SetPatch { patch: graph })?;
    Ok(())
}

fn create_example_patch(path: &PathBuf) -> anyhow::Result<()> {
    // Create file format (object with ID keys)
    let example_graph = FilePatchGraph {
        modules: HashMap::from([
            ("osc".to_string(), FileModuleState {
                module_type: "sine-oscillator".to_string(),
                params: HashMap::from([
                    ("freq".to_string(), Param::Note { value: 69 }), // A4
                ]),
            }),
            ("amp".to_string(), FileModuleState {
                module_type: "scale-and-shift".to_string(),
                params: HashMap::from([
                    ("input".to_string(), Param::Cable {
                        module: "osc".to_string(),
                        port: "output".to_string(),
                    }),
                    ("scale".to_string(), Param::Value { value: 5.0 }),
                ]),
            }),
            ("root".to_string(), FileModuleState {
                module_type: "signal".to_string(),
                params: HashMap::from([
                    ("source".to_string(), Param::Cable {
                        module: "amp".to_string(),
                        port: "output".to_string(),
                    }),
                ]),
            }),
        ]),
    };

    let yaml = serde_yaml::to_string(&example_graph)?;
    std::fs::write(path, yaml)?;
    Ok(())
}
