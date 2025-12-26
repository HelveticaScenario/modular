use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};
use tauri_plugin_opener::OpenerExt;

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
            let tray = TrayIconBuilder::new()
                .tooltip("Modular")
                .icon(app.default_window_icon().unwrap().clone())
                .on_menu_event(|app, event| match event.id.as_ref() {
                    QUIT_ID => {
                        println!("quit menu item was clicked");
                        app.exit(0);
                    }
                    OPEN_ID => {
                        println!("open menu item was clicked");
                        app.opener()
                            .open_url("http://localhost:5173", None::<&str>)
                            .unwrap();
                    }
                    _ => {
                        println!("menu item {:?} not handled", event.id);
                    }
                })
                .menu(&menu)
                .build(app)?;

            // IMPORTANT: Do NOT create any windows
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .run(tauri::generate_context!())
        .expect("error running tray app");
}
