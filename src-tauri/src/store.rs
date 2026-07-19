use crate::db::Database;
use crate::network::Device;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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
    pub db: Mutex<Database>,
}

impl AppState {
    pub fn load(db_path: PathBuf, legacy_json: PathBuf) -> Result<Self, String> {
        let db = Database::open(db_path)?;
        // One-time nickname import from the old devices.json snapshot.
        let _ = db.migrate_nicknames_from_json(&legacy_json);
        Ok(Self {
            cancel: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            db: Mutex::new(db),
        })
    }

    pub fn nicknames(&self) -> HashMap<String, String> {
        self.db.lock().nicknames().unwrap_or_default()
    }

    pub fn set_nickname(&self, key: &str, nickname: Option<String>) -> Result<(), String> {
        self.db.lock().set_nickname(key, nickname)
    }
}
