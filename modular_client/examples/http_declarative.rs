use std::collections::HashMap;
use std::time::Duration;

use modular_core::types::{ModuleState, Param, PatchGraph};
use modular_server::run_server;
use reqwest::Client;

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

    let client = Client::new();
    let url = "http://127.0.0.1:7812/patch";

    // Helper to build patch graph
    let build_patch = |note: u8, amp: f32| -> PatchGraph {
        PatchGraph {
            modules: vec![
                ModuleState {
                    id: "osc".to_string(),
                    module_type: "sine-oscillator".to_string(),
                    params: HashMap::from([
                        ("freq".to_string(), Param::Cable {
                            module: "freq-sum".to_string(),
                            port: "output".to_string(),
                        }),
                    ]),
                },
                ModuleState {
                    id: "vibrato".to_string(),
                    module_type: "scale-and-shift".to_string(),
                    params: HashMap::from([
                        ("input".to_string(), Param::Cable {
                            module: "osc".to_string(),
                            port: "output".to_string(),
                        }),
                        ("scale".to_string(), Param::Value { value: 1.0 }),
                    ]),
                },
                ModuleState {
                    id: "freq-sum".to_string(),
                    module_type: "sum".to_string(),
                    params: HashMap::from([
                        ("input-1".to_string(), Param::Cable {
                            module: "vibrato".to_string(),
                            port: "output".to_string(),
                        }),
                        ("input-2".to_string(), Param::Note { value: note }),
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

    // Set initial patch via HTTP
    let response = client
        .put(url)
        .json(&serde_json::json!({
            "graph": build_patch(69, 5.0)
        }))
        .send()
        .await?;

    println!("Initial patch set: {}", response.status());

    // Play melody via HTTP requests
    const A: u8 = 69;
    const B: u8 = 67;
    const C: u8 = 65;
    let part1 = [A, B, C];
    
    for _ in 0..2 {
        for note in part1.iter() {
            client
                .put(url)
                .json(&serde_json::json!({
                    "graph": build_patch(*note, 5.0)
                }))
                .send()
                .await?;
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    let part2 = [C, C, C, C, B, B, B, B];
    for note in part2.iter() {
        client
            .put(url)
            .json(&serde_json::json!({
                "graph": build_patch(*note, 5.0)
            }))
            .send()
            .await?;
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        client
            .put(url)
            .json(&serde_json::json!({
                "graph": build_patch(*note, 4.0)
            }))
            .send()
            .await?;
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    for note in part1.iter() {
        client
            .put(url)
            .json(&serde_json::json!({
                "graph": build_patch(*note, 5.0)
            }))
            .send()
            .await?;
        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    tokio::time::sleep(Duration::from_millis(3000)).await;
    
    println!("Melody complete!");
    Ok(())
}
