use crate::db::{Database, ScanRunDetail, ScanRunSummary};
use crate::network::dhcp::{self, DhcpDiscoverResult};
use crate::network::dns::{self, DnsQueryResult};
use crate::network::identity;
use crate::network::ping::{self, PingOutcome};
use crate::network::portscan::{self, PortScanResult};
use crate::network::scan;
use crate::network::wol::{self, WakeResult};
use crate::network::{Device, NetworkInfo, ScanResult};
use crate::store::AppState;
use std::net::Ipv4Addr;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::{AppHandle, State};

fn apply_db_metadata(db: &Database, mut info: NetworkInfo) -> NetworkInfo {
    if let Ok(Some(record)) = db.get_network_by_fingerprint(&info.fingerprint) {
        info.db_id = Some(record.id);
        info.display_name = Some(
            record
                .display_name
                .clone()
                .unwrap_or_else(|| record.auto_name.clone()),
        );
        if info.external_ip.is_none() {
            info.external_ip = record.external_ip;
        }
    } else {
        info.display_name = Some(info.auto_name.clone());
    }
    info
}

#[tauri::command]
pub fn get_network_info(state: State<'_, AppState>) -> Result<NetworkInfo, String> {
    let info = scan::network_info()?;
    Ok(apply_db_metadata(&state.db.lock(), info))
}

#[tauri::command]
pub fn list_networks(state: State<'_, AppState>) -> Result<Vec<NetworkInfo>, String> {
    let db = state.db.lock();
    let list = scan::list_networks()?
        .into_iter()
        .map(|info| apply_db_metadata(&db, info))
        .collect();
    Ok(list)
}

#[tauri::command]
pub async fn start_scan(
    app: AppHandle,
    state: State<'_, AppState>,
    network: Option<NetworkInfo>,
) -> Result<ScanResult, String> {
    let cancel = state.cancel.clone();
    cancel.store(false, Ordering::SeqCst);

    let started_at = chrono::Utc::now().timestamp();
    let nicknames = state.nicknames();

    // Scope previous devices to this network's fingerprint so offline merge
    // never pulls hosts from another LAN.
    let previous = {
        let db = state.db.lock();
        match &network {
            Some(n) => db
                .devices_for_network(&n.fingerprint)?
                .into_iter()
                .map(|d| (d.ip.clone(), d))
                .collect(),
            None => Default::default(),
        }
    };

    let mut result = scan::run_scan(app, cancel, nicknames, previous, network).await?;

    // Refresh WAN IP and persist network + devices + history (non-cancelled).
    if let Some(ext) = identity::fetch_external_ip().await {
        result.network.external_ip = Some(ext);
    }

    {
        let db = state.db.lock();
        let record = db.upsert_network(&result.network)?;
        result.network.db_id = Some(record.id);
        result.network.display_name = Some(
            record
                .display_name
                .clone()
                .unwrap_or_else(|| record.auto_name.clone()),
        );
        if result.network.external_ip.is_none() {
            result.network.external_ip = record.external_ip.clone();
        } else if let Some(ext) = &result.network.external_ip {
            let _ = db.set_network_external_ip(&result.network.fingerprint, Some(ext));
        }

        if !result.cancelled {
            db.replace_network_devices(record.id, &result.devices)?;
            let finished_at = chrono::Utc::now().timestamp();
            let _ = db.insert_scan_run(
                record.id,
                &result.network,
                &result.devices,
                started_at,
                finished_at,
                false,
            )?;
        }
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
pub fn get_devices(state: State<'_, AppState>, fingerprint: String) -> Result<Vec<Device>, String> {
    state.db.lock().devices_for_network(&fingerprint)
}

#[tauri::command]
pub fn clear_devices(state: State<'_, AppState>, fingerprint: String) -> Result<(), String> {
    state.db.lock().clear_network_devices(&fingerprint)
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
pub fn set_network_display_name(
    state: State<'_, AppState>,
    fingerprint: String,
    display_name: Option<String>,
) -> Result<NetworkInfo, String> {
    let db = state.db.lock();
    // Ensure the network row exists so rename works before the first completed scan.
    if db.get_network_by_fingerprint(&fingerprint)?.is_none() {
        if let Some(live) = scan::list_networks()?
            .into_iter()
            .find(|n| n.fingerprint == fingerprint)
        {
            let _ = db.upsert_network(&live)?;
        }
    }
    let record = db.set_network_display_name(&fingerprint, display_name)?;
    let mut live = scan::list_networks()?
        .into_iter()
        .find(|n| n.fingerprint == fingerprint)
        .unwrap_or(NetworkInfo {
            interface_name: record.interface_name.clone(),
            local_ip: record.local_ip.clone(),
            cidr: record.cidr.clone(),
            prefix: record
                .cidr
                .split('/')
                .nth(1)
                .and_then(|p| p.parse().ok())
                .unwrap_or(24),
            gateway: record.gateway.clone(),
            host_count: 0,
            fingerprint: record.fingerprint.clone(),
            display_name: None,
            auto_name: record.auto_name.clone(),
            media: record.media.clone(),
            ssid: record.ssid.clone(),
            search_domain: record.search_domain.clone(),
            external_ip: record.external_ip.clone(),
            db_id: Some(record.id),
        });
    live = apply_db_metadata(&db, live);
    Ok(live)
}

#[tauri::command]
pub async fn refresh_external_ip(
    state: State<'_, AppState>,
    fingerprint: String,
) -> Result<Option<String>, String> {
    let ip = identity::fetch_external_ip().await;
    if let Some(ref value) = ip {
        state
            .db
            .lock()
            .set_network_external_ip(&fingerprint, Some(value))?;
    }
    Ok(ip)
}

#[tauri::command]
pub fn list_scan_runs(
    state: State<'_, AppState>,
    limit: Option<i64>,
    offset: Option<i64>,
) -> Result<Vec<ScanRunSummary>, String> {
    state
        .db
        .lock()
        .list_scan_runs(limit.unwrap_or(50), offset.unwrap_or(0))
}

#[tauri::command]
pub fn get_scan_run(state: State<'_, AppState>, id: i64) -> Result<Option<ScanRunDetail>, String> {
    state.db.lock().get_scan_run(id)
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

#[tauri::command]
pub async fn dhcp_discover(
    local_ip: String,
    timeout_ms: Option<u32>,
) -> Result<DhcpDiscoverResult, String> {
    let addr: Ipv4Addr = local_ip
        .parse()
        .map_err(|e| format!("Invalid local IP {local_ip}: {e}"))?;
    let wait = Duration::from_millis(timeout_ms.unwrap_or(4000) as u64);
    Ok(dhcp::discover(addr, wait).await)
}

#[tauri::command]
pub fn dhcp_privilege_note() -> String {
    dhcp::privilege_note()
}

#[tauri::command]
pub async fn scan_ports(ip: String, ports: String) -> Result<PortScanResult, String> {
    let addr: Ipv4Addr = ip
        .parse()
        .map_err(|e| format!("Invalid IPv4 address {ip}: {e}"))?;
    let parsed = portscan::parse_ports_spec(&ports)?;
    Ok(portscan::scan_ports(addr, parsed).await)
}

#[tauri::command]
pub async fn wake_on_lan(
    mac: String,
    broadcast: Option<String>,
) -> Result<WakeResult, String> {
    wol::wake_on_lan(&mac, broadcast.as_deref()).await
}
