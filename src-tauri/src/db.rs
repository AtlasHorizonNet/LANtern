//! Local SQLite persistence for networks, live device caches, nicknames, and
//! scan-run history. The database file lives in the Tauri app data directory
//! (`lantern.db`) and never leaves the device.

use crate::network::{Device, NetworkInfo};
use rusqlite::{params, Connection, OptionalExtension};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const SCHEMA_VERSION: i32 = 1;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkRecord {
    pub id: i64,
    pub fingerprint: String,
    pub display_name: Option<String>,
    pub auto_name: String,
    pub media: String,
    pub ssid: Option<String>,
    pub search_domain: Option<String>,
    pub interface_name: String,
    pub cidr: String,
    pub gateway: Option<String>,
    pub local_ip: String,
    pub external_ip: Option<String>,
    pub last_seen_at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRunSummary {
    pub id: i64,
    pub network_id: i64,
    pub network_name: String,
    pub started_at: i64,
    pub finished_at: i64,
    pub interface_name: String,
    pub local_ip: String,
    pub cidr: String,
    pub gateway: Option<String>,
    pub external_ip: Option<String>,
    pub device_count: i64,
    pub online_count: i64,
    pub cancelled: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRunDetail {
    pub run: ScanRunSummary,
    pub devices: Vec<Device>,
}

pub struct Database {
    conn: Connection,
    pub path: PathBuf,
}

impl Database {
    pub fn open(path: PathBuf) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("create db dir: {e}"))?;
        }
        let conn = Connection::open(&path).map_err(|e| format!("open sqlite: {e}"))?;
        let mut db = Self { conn, path };
        db.migrate()?;
        Ok(db)
    }

    pub fn open_in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| format!("open memory sqlite: {e}"))?;
        let mut db = Self {
            conn,
            path: PathBuf::from(":memory:"),
        };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&mut self) -> Result<(), String> {
        let version: i32 = self
            .conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .map_err(|e| format!("read user_version: {e}"))?;

        if version < 1 {
            self.conn
                .execute_batch(
                    r#"
                    CREATE TABLE IF NOT EXISTS networks (
                        id INTEGER PRIMARY KEY,
                        fingerprint TEXT NOT NULL UNIQUE,
                        display_name TEXT,
                        auto_name TEXT NOT NULL,
                        media TEXT NOT NULL,
                        ssid TEXT,
                        search_domain TEXT,
                        interface_name TEXT NOT NULL,
                        cidr TEXT NOT NULL,
                        gateway TEXT,
                        local_ip TEXT NOT NULL,
                        external_ip TEXT,
                        last_seen_at INTEGER NOT NULL
                    );

                    CREATE TABLE IF NOT EXISTS nicknames (
                        device_key TEXT PRIMARY KEY,
                        nickname TEXT NOT NULL
                    );

                    CREATE TABLE IF NOT EXISTS network_devices (
                        network_id INTEGER NOT NULL REFERENCES networks(id) ON DELETE CASCADE,
                        device_key TEXT NOT NULL,
                        ip TEXT NOT NULL,
                        mac TEXT,
                        hostname TEXT,
                        vendor TEXT,
                        online INTEGER NOT NULL,
                        last_seen INTEGER NOT NULL,
                        is_gateway INTEGER NOT NULL,
                        is_local INTEGER NOT NULL,
                        PRIMARY KEY (network_id, device_key)
                    );

                    CREATE TABLE IF NOT EXISTS scan_runs (
                        id INTEGER PRIMARY KEY,
                        network_id INTEGER NOT NULL REFERENCES networks(id),
                        started_at INTEGER NOT NULL,
                        finished_at INTEGER NOT NULL,
                        interface_name TEXT NOT NULL,
                        local_ip TEXT NOT NULL,
                        cidr TEXT NOT NULL,
                        gateway TEXT,
                        external_ip TEXT,
                        device_count INTEGER NOT NULL,
                        online_count INTEGER NOT NULL,
                        cancelled INTEGER NOT NULL DEFAULT 0
                    );

                    CREATE TABLE IF NOT EXISTS scan_run_devices (
                        id INTEGER PRIMARY KEY,
                        run_id INTEGER NOT NULL REFERENCES scan_runs(id) ON DELETE CASCADE,
                        ip TEXT NOT NULL,
                        mac TEXT,
                        hostname TEXT,
                        vendor TEXT,
                        nickname TEXT,
                        online INTEGER NOT NULL,
                        last_seen INTEGER NOT NULL,
                        is_gateway INTEGER NOT NULL,
                        is_local INTEGER NOT NULL
                    );

                    CREATE INDEX IF NOT EXISTS idx_scan_runs_finished
                        ON scan_runs(finished_at DESC);
                    CREATE INDEX IF NOT EXISTS idx_network_devices_network
                        ON network_devices(network_id);
                    "#,
                )
                .map_err(|e| format!("create schema: {e}"))?;
            self.conn
                .pragma_update(None, "user_version", SCHEMA_VERSION)
                .map_err(|e| format!("set user_version: {e}"))?;
        }
        Ok(())
    }

    pub fn migrate_nicknames_from_json(&self, json_path: &Path) -> Result<(), String> {
        if !json_path.exists() {
            return Ok(());
        }
        let raw = std::fs::read_to_string(json_path).map_err(|e| e.to_string())?;
        let data: crate::store::DeviceStoreData =
            serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        for (key, nickname) in data.nicknames {
            self.set_nickname(&key, Some(nickname))?;
        }
        Ok(())
    }

    pub fn upsert_network(&self, info: &NetworkInfo) -> Result<NetworkRecord, String> {
        let now = chrono::Utc::now().timestamp();
        self.conn
            .execute(
                r#"
                INSERT INTO networks (
                    fingerprint, display_name, auto_name, media, ssid, search_domain,
                    interface_name, cidr, gateway, local_ip, external_ip, last_seen_at
                ) VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                ON CONFLICT(fingerprint) DO UPDATE SET
                    auto_name = excluded.auto_name,
                    media = excluded.media,
                    ssid = excluded.ssid,
                    search_domain = excluded.search_domain,
                    interface_name = excluded.interface_name,
                    cidr = excluded.cidr,
                    gateway = excluded.gateway,
                    local_ip = excluded.local_ip,
                    external_ip = COALESCE(excluded.external_ip, networks.external_ip),
                    last_seen_at = excluded.last_seen_at
                "#,
                params![
                    info.fingerprint,
                    info.auto_name,
                    info.media,
                    info.ssid,
                    info.search_domain,
                    info.interface_name,
                    info.cidr,
                    info.gateway,
                    info.local_ip,
                    info.external_ip,
                    now,
                ],
            )
            .map_err(|e| format!("upsert network: {e}"))?;

        self.get_network_by_fingerprint(&info.fingerprint)?
            .ok_or_else(|| "network missing after upsert".into())
    }

    pub fn get_network_by_fingerprint(
        &self,
        fingerprint: &str,
    ) -> Result<Option<NetworkRecord>, String> {
        self.conn
            .query_row(
                r#"
                SELECT id, fingerprint, display_name, auto_name, media, ssid, search_domain,
                       interface_name, cidr, gateway, local_ip, external_ip, last_seen_at
                FROM networks WHERE fingerprint = ?1
                "#,
                params![fingerprint],
                |row| {
                    Ok(NetworkRecord {
                        id: row.get(0)?,
                        fingerprint: row.get(1)?,
                        display_name: row.get(2)?,
                        auto_name: row.get(3)?,
                        media: row.get(4)?,
                        ssid: row.get(5)?,
                        search_domain: row.get(6)?,
                        interface_name: row.get(7)?,
                        cidr: row.get(8)?,
                        gateway: row.get(9)?,
                        local_ip: row.get(10)?,
                        external_ip: row.get(11)?,
                        last_seen_at: row.get(12)?,
                    })
                },
            )
            .optional()
            .map_err(|e| format!("get network: {e}"))
    }

    pub fn set_network_display_name(
        &self,
        fingerprint: &str,
        display_name: Option<String>,
    ) -> Result<NetworkRecord, String> {
        let name = display_name.and_then(|s| {
            let t = s.trim().to_string();
            if t.is_empty() {
                None
            } else {
                Some(t)
            }
        });
        let changed = self
            .conn
            .execute(
                "UPDATE networks SET display_name = ?1 WHERE fingerprint = ?2",
                params![name, fingerprint],
            )
            .map_err(|e| format!("rename network: {e}"))?;
        if changed == 0 {
            return Err("Unknown network; scan once before renaming".into());
        }
        self.get_network_by_fingerprint(fingerprint)?
            .ok_or_else(|| "network missing after rename".into())
    }

    pub fn set_network_external_ip(
        &self,
        fingerprint: &str,
        external_ip: Option<&str>,
    ) -> Result<(), String> {
        self.conn
            .execute(
                "UPDATE networks SET external_ip = ?1, last_seen_at = ?2 WHERE fingerprint = ?3",
                params![external_ip, chrono::Utc::now().timestamp(), fingerprint],
            )
            .map_err(|e| format!("set external ip: {e}"))?;
        Ok(())
    }

    pub fn nicknames(&self) -> Result<HashMap<String, String>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT device_key, nickname FROM nicknames")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?;
        let mut map = HashMap::new();
        for row in rows {
            let (k, v) = row.map_err(|e| e.to_string())?;
            map.insert(k, v);
        }
        Ok(map)
    }

    pub fn set_nickname(&self, key: &str, nickname: Option<String>) -> Result<(), String> {
        match nickname {
            Some(n) if !n.trim().is_empty() => {
                self.conn
                    .execute(
                        r#"
                        INSERT INTO nicknames (device_key, nickname) VALUES (?1, ?2)
                        ON CONFLICT(device_key) DO UPDATE SET nickname = excluded.nickname
                        "#,
                        params![key, n.trim()],
                    )
                    .map_err(|e| e.to_string())?;
            }
            _ => {
                self.conn
                    .execute("DELETE FROM nicknames WHERE device_key = ?1", params![key])
                    .map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }

    pub fn replace_network_devices(
        &self,
        network_id: i64,
        devices: &[Device],
    ) -> Result<(), String> {
        let tx = self
            .conn
            .unchecked_transaction()
            .map_err(|e| e.to_string())?;
        tx.execute(
            "DELETE FROM network_devices WHERE network_id = ?1",
            params![network_id],
        )
        .map_err(|e| e.to_string())?;

        {
            let mut stmt = tx
                .prepare(
                    r#"
                    INSERT INTO network_devices (
                        network_id, device_key, ip, mac, hostname, vendor,
                        online, last_seen, is_gateway, is_local
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                    "#,
                )
                .map_err(|e| e.to_string())?;
            for d in devices {
                let key = d.mac.clone().unwrap_or_else(|| d.ip.clone());
                stmt.execute(params![
                    network_id,
                    key,
                    d.ip,
                    d.mac,
                    d.hostname,
                    d.vendor,
                    d.online as i64,
                    d.last_seen,
                    d.is_gateway as i64,
                    d.is_local as i64,
                ])
                .map_err(|e| e.to_string())?;
            }
        }
        tx.commit().map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn clear_network_devices(&self, fingerprint: &str) -> Result<(), String> {
        let Some(net) = self.get_network_by_fingerprint(fingerprint)? else {
            return Ok(());
        };
        self.conn
            .execute(
                "DELETE FROM network_devices WHERE network_id = ?1",
                params![net.id],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn devices_for_network(&self, fingerprint: &str) -> Result<Vec<Device>, String> {
        let Some(net) = self.get_network_by_fingerprint(fingerprint)? else {
            return Ok(Vec::new());
        };
        let nicknames = self.nicknames()?;
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT ip, mac, hostname, vendor, online, last_seen, is_gateway, is_local, device_key
                FROM network_devices WHERE network_id = ?1
                ORDER BY ip
                "#,
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![net.id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, String>(8)?,
                ))
            })
            .map_err(|e| e.to_string())?;

        let mut devices = Vec::new();
        for row in rows {
            let (ip, mac, hostname, vendor, online, last_seen, is_gateway, is_local, key) =
                row.map_err(|e| e.to_string())?;
            let nickname = nicknames
                .get(&key)
                .cloned()
                .or_else(|| nicknames.get(&ip).cloned());
            devices.push(Device {
                ip,
                mac,
                hostname,
                vendor,
                nickname,
                online: online != 0,
                last_seen,
                is_gateway: is_gateway != 0,
                is_local: is_local != 0,
            });
        }
        Ok(devices)
    }

    pub fn insert_scan_run(
        &self,
        network_id: i64,
        info: &NetworkInfo,
        devices: &[Device],
        started_at: i64,
        finished_at: i64,
        cancelled: bool,
    ) -> Result<i64, String> {
        let online_count = devices.iter().filter(|d| d.online).count() as i64;
        self.conn
            .execute(
                r#"
                INSERT INTO scan_runs (
                    network_id, started_at, finished_at, interface_name, local_ip, cidr,
                    gateway, external_ip, device_count, online_count, cancelled
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                "#,
                params![
                    network_id,
                    started_at,
                    finished_at,
                    info.interface_name,
                    info.local_ip,
                    info.cidr,
                    info.gateway,
                    info.external_ip,
                    devices.len() as i64,
                    online_count,
                    cancelled as i64,
                ],
            )
            .map_err(|e| format!("insert scan run: {e}"))?;
        let run_id = self.conn.last_insert_rowid();

        let mut stmt = self
            .conn
            .prepare(
                r#"
                INSERT INTO scan_run_devices (
                    run_id, ip, mac, hostname, vendor, nickname, online, last_seen,
                    is_gateway, is_local
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
                "#,
            )
            .map_err(|e| e.to_string())?;
        for d in devices {
            stmt.execute(params![
                run_id,
                d.ip,
                d.mac,
                d.hostname,
                d.vendor,
                d.nickname,
                d.online as i64,
                d.last_seen,
                d.is_gateway as i64,
                d.is_local as i64,
            ])
            .map_err(|e| e.to_string())?;
        }
        Ok(run_id)
    }

    pub fn list_scan_runs(&self, limit: i64, offset: i64) -> Result<Vec<ScanRunSummary>, String> {
        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT
                    r.id, r.network_id,
                    COALESCE(n.display_name, n.auto_name, r.cidr) AS network_name,
                    r.started_at, r.finished_at, r.interface_name, r.local_ip, r.cidr,
                    r.gateway, r.external_ip, r.device_count, r.online_count, r.cancelled
                FROM scan_runs r
                LEFT JOIN networks n ON n.id = r.network_id
                ORDER BY r.finished_at DESC
                LIMIT ?1 OFFSET ?2
                "#,
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![limit, offset], |row| {
                Ok(ScanRunSummary {
                    id: row.get(0)?,
                    network_id: row.get(1)?,
                    network_name: row.get(2)?,
                    started_at: row.get(3)?,
                    finished_at: row.get(4)?,
                    interface_name: row.get(5)?,
                    local_ip: row.get(6)?,
                    cidr: row.get(7)?,
                    gateway: row.get(8)?,
                    external_ip: row.get(9)?,
                    device_count: row.get(10)?,
                    online_count: row.get(11)?,
                    cancelled: row.get::<_, i64>(12)? != 0,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    pub fn get_scan_run(&self, id: i64) -> Result<Option<ScanRunDetail>, String> {
        let run = self
            .conn
            .query_row(
                r#"
                SELECT
                    r.id, r.network_id,
                    COALESCE(n.display_name, n.auto_name, r.cidr) AS network_name,
                    r.started_at, r.finished_at, r.interface_name, r.local_ip, r.cidr,
                    r.gateway, r.external_ip, r.device_count, r.online_count, r.cancelled
                FROM scan_runs r
                LEFT JOIN networks n ON n.id = r.network_id
                WHERE r.id = ?1
                "#,
                params![id],
                |row| {
                    Ok(ScanRunSummary {
                        id: row.get(0)?,
                        network_id: row.get(1)?,
                        network_name: row.get(2)?,
                        started_at: row.get(3)?,
                        finished_at: row.get(4)?,
                        interface_name: row.get(5)?,
                        local_ip: row.get(6)?,
                        cidr: row.get(7)?,
                        gateway: row.get(8)?,
                        external_ip: row.get(9)?,
                        device_count: row.get(10)?,
                        online_count: row.get(11)?,
                        cancelled: row.get::<_, i64>(12)? != 0,
                    })
                },
            )
            .optional()
            .map_err(|e| e.to_string())?;

        let Some(run) = run else {
            return Ok(None);
        };

        let mut stmt = self
            .conn
            .prepare(
                r#"
                SELECT ip, mac, hostname, vendor, nickname, online, last_seen, is_gateway, is_local
                FROM scan_run_devices WHERE run_id = ?1 ORDER BY ip
                "#,
            )
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map(params![id], |row| {
                Ok(Device {
                    ip: row.get(0)?,
                    mac: row.get(1)?,
                    hostname: row.get(2)?,
                    vendor: row.get(3)?,
                    nickname: row.get(4)?,
                    online: row.get::<_, i64>(5)? != 0,
                    last_seen: row.get(6)?,
                    is_gateway: row.get::<_, i64>(7)? != 0,
                    is_local: row.get::<_, i64>(8)? != 0,
                })
            })
            .map_err(|e| e.to_string())?;
        let mut devices = Vec::new();
        for row in rows {
            devices.push(row.map_err(|e| e.to_string())?);
        }
        Ok(Some(ScanRunDetail { run, devices }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_network(fp: &str) -> NetworkInfo {
        NetworkInfo {
            interface_name: "eth0".into(),
            local_ip: "192.168.1.10".into(),
            cidr: "192.168.1.0/24".into(),
            prefix: 24,
            gateway: Some("192.168.1.1".into()),
            host_count: 254,
            fingerprint: fp.into(),
            display_name: None,
            auto_name: "Home".into(),
            media: "ethernet".into(),
            ssid: None,
            search_domain: Some("lan".into()),
            external_ip: Some("203.0.113.9".into()),
            db_id: None,
        }
    }

    fn sample_device(ip: &str) -> Device {
        Device {
            ip: ip.into(),
            mac: Some("aa:bb:cc:dd:ee:ff".into()),
            hostname: Some("host".into()),
            vendor: Some("Vendor".into()),
            nickname: Some("Router".into()),
            online: true,
            last_seen: 1_700_000_000,
            is_gateway: ip.ends_with(".1"),
            is_local: false,
        }
    }

    #[test]
    fn scan_run_round_trip() {
        let db = Database::open_in_memory().unwrap();
        let net = sample_network("lan:home|192.168.1.0/24|192.168.1.1");
        let record = db.upsert_network(&net).unwrap();
        let devices = vec![sample_device("192.168.1.1"), sample_device("192.168.1.20")];
        let run_id = db
            .insert_scan_run(record.id, &net, &devices, 100, 200, false)
            .unwrap();

        let listed = db.list_scan_runs(10, 0).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].id, run_id);
        assert_eq!(listed[0].device_count, 2);
        assert_eq!(listed[0].external_ip.as_deref(), Some("203.0.113.9"));

        let detail = db.get_scan_run(run_id).unwrap().unwrap();
        assert_eq!(detail.devices.len(), 2);
        assert_eq!(detail.run.network_name, "Home");
    }

    #[test]
    fn devices_are_scoped_per_network() {
        let db = Database::open_in_memory().unwrap();
        let a = sample_network("wifi:a|192.168.1.0/24|192.168.1.1");
        let mut b = sample_network("wifi:b|10.0.0.0/24|10.0.0.1");
        b.cidr = "10.0.0.0/24".into();
        b.local_ip = "10.0.0.5".into();
        b.gateway = Some("10.0.0.1".into());

        let ra = db.upsert_network(&a).unwrap();
        let rb = db.upsert_network(&b).unwrap();
        db.replace_network_devices(ra.id, &[sample_device("192.168.1.20")])
            .unwrap();
        db.replace_network_devices(rb.id, &[sample_device("10.0.0.20")])
            .unwrap();

        let a_devices = db.devices_for_network(&a.fingerprint).unwrap();
        let b_devices = db.devices_for_network(&b.fingerprint).unwrap();
        assert_eq!(a_devices.len(), 1);
        assert_eq!(a_devices[0].ip, "192.168.1.20");
        assert_eq!(b_devices.len(), 1);
        assert_eq!(b_devices[0].ip, "10.0.0.20");

        db.clear_network_devices(&a.fingerprint).unwrap();
        assert!(db.devices_for_network(&a.fingerprint).unwrap().is_empty());
        assert_eq!(db.devices_for_network(&b.fingerprint).unwrap().len(), 1);
    }

    #[test]
    fn network_rename_persists() {
        let db = Database::open_in_memory().unwrap();
        let net = sample_network("wifi:cafe|192.168.0.0/24|192.168.0.1");
        db.upsert_network(&net).unwrap();
        let renamed = db
            .set_network_display_name(&net.fingerprint, Some("Cafe Wi-Fi".into()))
            .unwrap();
        assert_eq!(renamed.display_name.as_deref(), Some("Cafe Wi-Fi"));
        let again = db
            .get_network_by_fingerprint(&net.fingerprint)
            .unwrap()
            .unwrap();
        assert_eq!(again.display_name.as_deref(), Some("Cafe Wi-Fi"));
    }
}
