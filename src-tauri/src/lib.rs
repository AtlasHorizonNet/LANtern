mod commands;
pub mod network;
pub mod store;

use store::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let path = app
                .path()
                .app_data_dir()
                .map(|p| p.join("devices.json"))
                .unwrap_or_else(|_| std::path::PathBuf::from("devices.json"));
            app.manage(AppState::load(path));

            // Ensure the window/taskbar icon matches the bundled LANtern mark.
            // On Windows the title-bar and shell icons can diverge unless set
            // explicitly from the same default window icon asset.
            if let (Some(window), Some(icon)) = (
                app.get_webview_window("main"),
                app.default_window_icon().cloned(),
            ) {
                let _ = window.set_icon(icon);
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_network_info,
            commands::list_networks,
            commands::start_scan,
            commands::cancel_scan,
            commands::ping_device,
            commands::get_devices,
            commands::set_device_nickname,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
