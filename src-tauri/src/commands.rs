use crate::network::dns::{self, DnsQueryResult};
use crate::network::ping::{self, PingOutcome};
use crate::network::scan;
use crate::network::{Device, NetworkInfo, ScanResult};
use crate::store::AppState;
use std::net::Ipv4Addr;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, State};

#[tauri::command]
pub fn get_network_info() -> Result<NetworkInfo, String> {
    scan::network_info()
}

#[tauri::command]
pub fn list_networks() -> Result<Vec<NetworkInfo>, String> {
    scan::list_networks()
}

#[tauri::command]
pub async fn start_scan(
    app: AppHandle,
    state: State<'_, AppState>,
    network: Option<NetworkInfo>,
) -> Result<ScanResult, String> {
    let cancel = state.cancel.clone();
    cancel.store(false, Ordering::SeqCst);

    let nicknames = state.nicknames();
    let previous = state.previous_by_ip();

    let result = scan::run_scan(app, cancel, nicknames, previous, network).await?;

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
pub async fn ping_device(ip: String) -> Result<PingOutcome, String> {
    let addr: Ipv4Addr = ip
        .parse()
        .map_err(|e| format!("Invalid IPv4 address {ip}: {e}"))?;
    Ok(ping::ping_once(addr).await)
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

#[tauri::command]
pub async fn dns_lookup(
    host: String,
    record_type: String,
    server: Option<String>,
) -> Result<DnsQueryResult, String> {
    dns::dns_lookup(&host, &record_type, server.as_deref()).await
}

#[tauri::command]
pub async fn dns_reverse(ip: String, server: Option<String>) -> Result<DnsQueryResult, String> {
    dns::dns_reverse(&ip, server.as_deref()).await
}
