pub mod dhcp;
pub mod dns;
pub mod identity;
pub mod interfaces;
pub mod neighbors;
pub mod oui;
pub mod ping;
pub mod portscan;
pub mod scan;
pub mod wol;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInfo {
    pub interface_name: String,
    pub local_ip: String,
    pub cidr: String,
    pub prefix: u8,
    pub gateway: Option<String>,
    pub host_count: u32,
    /// Stable identity key used for DB scoping / renames.
    pub fingerprint: String,
    /// User override when set; otherwise mirrors `auto_name`.
    pub display_name: Option<String>,
    pub auto_name: String,
    /// `wifi` | `ethernet` | `other` | `unknown`
    pub media: String,
    pub ssid: Option<String>,
    pub search_domain: Option<String>,
    pub external_ip: Option<String>,
    /// SQLite `networks.id` when known.
    pub db_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub ip: String,
    pub mac: Option<String>,
    pub hostname: Option<String>,
    pub vendor: Option<String>,
    pub nickname: Option<String>,
    pub online: bool,
    pub last_seen: i64,
    pub is_gateway: bool,
    pub is_local: bool,
}

/// Stable identity for nicknames / DB rows: MAC when known, otherwise IP.
pub fn device_identity_key(device: &Device) -> String {
    device
        .mac
        .as_deref()
        .map(str::trim)
        .filter(|m| !m.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| device.ip.clone())
}

/// Collapse duplicates that share a MAC (or IP when MAC is missing).
/// Prefers online over offline, then newer `last_seen`, then local/gateway.
pub fn dedupe_devices(devices: Vec<Device>) -> Vec<Device> {
    let mut best: HashMap<String, (usize, Device)> = HashMap::new();
    for (idx, device) in devices.into_iter().enumerate() {
        let key = device_identity_key(&device);
        match best.get(&key) {
            None => {
                best.insert(key, (idx, device));
            }
            Some((_, existing)) => {
                let prefer_new = (device.online && !existing.online)
                    || (device.online == existing.online && device.last_seen > existing.last_seen)
                    || (device.online == existing.online
                        && device.last_seen == existing.last_seen
                        && ((device.is_local && !existing.is_local)
                            || (device.is_gateway && !existing.is_gateway && !existing.is_local)));
                if prefer_new {
                    best.insert(key, (idx, device));
                }
            }
        }
    }
    let mut out: Vec<(usize, Device)> = best.into_values().collect();
    out.sort_by_key(|(idx, _)| *idx);
    out.into_iter().map(|(_, d)| d).collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub checked: u32,
    pub total: u32,
    pub found: u32,
    pub phase: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub network: NetworkInfo,
    pub devices: Vec<Device>,
    pub cancelled: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(ip: &str, mac: Option<&str>, online: bool, last_seen: i64) -> Device {
        Device {
            ip: ip.into(),
            mac: mac.map(str::to_string),
            hostname: None,
            vendor: None,
            nickname: None,
            online,
            last_seen,
            is_gateway: false,
            is_local: false,
        }
    }

    #[test]
    fn dedupe_prefers_online_same_mac() {
        let devices = dedupe_devices(vec![
            device("10.0.0.50", Some("AA:BB:CC:DD:EE:01"), false, 100),
            device("10.0.0.60", Some("AA:BB:CC:DD:EE:01"), true, 200),
        ]);
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].ip, "10.0.0.60");
        assert!(devices[0].online);
    }

    #[test]
    fn identity_key_falls_back_to_ip() {
        let d = device("10.0.0.9", None, true, 1);
        assert_eq!(device_identity_key(&d), "10.0.0.9");
    }
}
