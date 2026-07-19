mod commands;
pub mod db;
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
            let data_dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let db_path = data_dir.join("lantern.db");
            let legacy_json = data_dir.join("devices.json");
            app.manage(AppState::load(db_path, legacy_json)?);

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
            commands::clear_devices,
            commands::set_device_nickname,
            commands::set_network_display_name,
            commands::refresh_external_ip,
            commands::list_scan_runs,
            commands::get_scan_run,
            commands::dns_lookup,
            commands::dns_reverse,
            commands::dhcp_discover,
            commands::dhcp_privilege_note,
            commands::scan_ports,
            commands::wake_on_lan,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
