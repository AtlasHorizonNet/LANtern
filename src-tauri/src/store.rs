use crate::network::Device;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceStoreData {
    /// Keyed by MAC when known, otherwise IP
    pub nicknames: HashMap<String, String>,
    pub devices: HashMap<String, Device>,
}

pub struct AppState {
    pub cancel: Arc<std::sync::atomic::AtomicBool>,
    pub store: Mutex<DeviceStoreData>,
    pub path: PathBuf,
}

impl AppState {
    pub fn load(path: PathBuf) -> Self {
        let store = if path.exists() {
            fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            DeviceStoreData::default()
        };

        Self {
            cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            store: Mutex::new(store),
            path,
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let data = self.store.lock();
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("create store dir: {e}"))?;
        }
        let json = serde_json::to_string_pretty(&*data).map_err(|e| e.to_string())?;
        fs::write(&self.path, json).map_err(|e| format!("write store: {e}"))?;
        Ok(())
    }

    pub fn upsert_devices(&self, devices: &[Device]) {
        let mut data = self.store.lock();
        for d in devices {
            let key = d.mac.clone().unwrap_or_else(|| d.ip.clone());
            data.devices.insert(key, d.clone());
            // also index by IP for offline merge convenience
            data.devices.insert(d.ip.clone(), d.clone());
        }
    }

    pub fn set_nickname(&self, key: &str, nickname: Option<String>) -> Result<(), String> {
        let mut data = self.store.lock();
        let applied = match nickname {
            Some(n) if !n.trim().is_empty() => {
                let value = n.trim().to_string();
                data.nicknames.insert(key.to_string(), value.clone());
                Some(value)
            }
            _ => {
                data.nicknames.remove(key);
                None
            }
        };
        // apply to cached devices
        for device in data.devices.values_mut() {
            let device_key = device.mac.clone().unwrap_or_else(|| device.ip.clone());
            if device_key == key || device.ip == key || device.mac.as_deref() == Some(key) {
                device.nickname = applied.clone();
            }
        }
        drop(data);
        self.save()
    }

    pub fn nicknames(&self) -> HashMap<String, String> {
        self.store.lock().nicknames.clone()
    }

    pub fn previous_by_ip(&self) -> HashMap<String, Device> {
        let data = self.store.lock();
        let mut map = HashMap::new();
        for d in data.devices.values() {
            map.insert(d.ip.clone(), d.clone());
        }
        map
    }

    pub fn all_devices(&self) -> Vec<Device> {
        let data = self.store.lock();
        let mut seen = HashMap::new();
        for d in data.devices.values() {
            seen.insert(d.ip.clone(), d.clone());
        }
        let mut list: Vec<_> = seen.into_values().collect();
        list.sort_by(|a, b| a.ip.cmp(&b.ip));
        list
    }
}
