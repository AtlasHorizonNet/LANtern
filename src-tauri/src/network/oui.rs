use once_cell::sync::Lazy;
use std::collections::HashMap;

static OUI_MAP: Lazy<HashMap<String, String>> = Lazy::new(load_oui);

fn load_oui() -> HashMap<String, String> {
    let raw = include_str!("../../resources/oui.txt");
    let mut map = HashMap::with_capacity(40_000);
    for line in raw.lines() {
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, '\t');
        let Some(oui) = parts.next() else { continue };
        let Some(vendor) = parts.next() else { continue };
        map.insert(oui.to_uppercase(), vendor.to_string());
    }
    map
}

/// Look up a vendor from a MAC address (any common delimiter).
pub fn lookup_vendor(mac: &str) -> Option<String> {
    let cleaned: String = mac
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect::<String>()
        .to_uppercase();
    if cleaned.len() < 6 {
        return None;
    }
    let key = format!("{}:{}:{}", &cleaned[0..2], &cleaned[2..4], &cleaned[4..6]);
    OUI_MAP.get(&key).cloned()
}

pub fn warm_cache() {
    let _ = OUI_MAP.len();
}
