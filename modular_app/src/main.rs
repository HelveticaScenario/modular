// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{thread, time::Duration};

use modular_server::{ServerConfig, run_server};
use tokio::{runtime::Runtime, select, task, time};

// #[tokio::main]
// async fn main() -> anyhow::Result<()> {
//     let taskbar_task = task::spawn_blocking(|| modular_app_lib::run());

//     let server_task = run_server(ServerConfig {
//         port: 7812,
//         patch_file: None,
//     });

//     select! {
//         res = taskbar_task => {
//             res?;
//             Ok(())
//         }
//         res = server_task => {
//             res?;
//             Ok(())
//         }
//     }
// }

fn main() {
    // Spawn a standard OS thread to run a dedicated Tokio runtime
    let tokio_thread = thread::spawn(move || {
        let rt = Runtime::new().expect("Failed to create runtime");
        rt.block_on(async {
            println!("Runtime running in a separate thread");
            run_server(ServerConfig {
                port: 7812,
                patch_file: None,
            })
            .await
            .unwrap();
        });
    });
    modular_app_lib::run();
    tokio_thread.join().expect("Tokio thread panicked");
}
