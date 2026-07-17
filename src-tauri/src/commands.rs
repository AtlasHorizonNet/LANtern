use crate::network::scan;
use crate::network::{Device, NetworkInfo, ScanResult};
use crate::store::AppState;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn get_network_info() -> Result<NetworkInfo, String> {
    scan::network_info()
}

#[tauri::command]
pub async fn start_scan(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<ScanResult, String> {
    let cancel = state.cancel.clone();
    cancel.store(false, Ordering::SeqCst);

    let nicknames = state.nicknames();
    let previous = state.previous_by_ip();

    let result = scan::run_scan(app, cancel, nicknames, previous).await?;

    if !result.cancelled {
        state.upsert_devices(&result.devices);
        let _ = state.save();
    }

    Ok(result)
}

#[tauri::command]
pub fn cancel_scan(state: State<'_, AppState>) -> Result<(), String> {
    state.cancel.store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
pub fn get_devices(state: State<'_, AppState>) -> Result<Vec<Device>, String> {
    Ok(state.all_devices())
}

#[tauri::command]
pub fn set_device_nickname(
    state: State<'_, AppState>,
    key: String,
    nickname: Option<String>,
) -> Result<(), String> {
    state.set_nickname(&key, nickname)
}
