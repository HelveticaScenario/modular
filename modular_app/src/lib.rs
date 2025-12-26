use std::thread;

use modular_server::{ServerConfig, run_server};
use tauri::{
    Manager,
    menu::{Menu, MenuItem},
    path::BaseDirectory,
    tray::TrayIconBuilder,
};
use tauri_plugin_opener::OpenerExt;
use tokio::runtime::Runtime;

#[cfg(dev)]
const URL: &str = "http://localhost:5173";
#[cfg(not(dev))]
const URL: &str = "http://localhost:7812";

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            const QUIT_ID: &str = "quit";
            const OPEN_ID: &str = "open";
            let open = MenuItem::with_id(app, OPEN_ID, "Open", true, None::<&str>)
                .expect("error creating menu item");
            let quit = MenuItem::with_id(app, QUIT_ID, "Quit", true, None::<&str>)
                .expect("error creating menu item");
            let menu = Menu::with_items(app, &[&open, &quit]).expect("error creating menu");
            let _tray = TrayIconBuilder::new()
                .tooltip("Modular")
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(|app, event| match event.id.as_ref() {
                    QUIT_ID => {
                        println!("quit menu item was clicked");
                        app.exit(0);
                    }
                    OPEN_ID => {
                        println!("open menu item was clicked");
                        app.opener().open_url(URL, None::<&str>).unwrap();
                    }
                    _ => {
                        println!("menu item {:?} not handled", event.id);
                    }
                })
                .menu(&menu)
                .build(app)?;

            let serve_dir = app
                .path()
                .resolve("../dist", BaseDirectory::Resource)?
                .to_string_lossy()
                .to_string();
            // Spawn a standard OS thread to run a dedicated Tokio runtime
            let _ = thread::spawn(move || {
                let rt = Runtime::new().expect("Failed to create runtime");
                rt.block_on(async {
                    println!("Serving from directory: {}", serve_dir);
                    println!("Runtime running in a separate thread");
                    run_server(ServerConfig {
                        port: 7812,
                        patch_file: None,
                        serve_dir,
                    })
                    .await
                    .unwrap();
                });
            });

            // IMPORTANT: Do NOT create any windows
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("error running tray app");
}
