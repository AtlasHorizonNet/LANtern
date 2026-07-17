pub mod dns;
pub mod interfaces;
pub mod neighbors;
pub mod oui;
pub mod ping;
pub mod scan;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInfo {
    pub interface_name: String,
    pub local_ip: String,
    pub cidr: String,
    pub prefix: u8,
    pub gateway: Option<String>,
    pub host_count: u32,
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
