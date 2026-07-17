use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::process::Command;

/// Read the OS neighbor/ARP table. Keys are IPv4 strings, values are normalized MAC strings.
pub fn read_neighbors() -> HashMap<String, String> {
    #[cfg(target_os = "linux")]
    {
        read_linux_proc_arp()
            .or_else(read_ip_neigh)
            .unwrap_or_default()
    }
    #[cfg(target_os = "macos")]
    {
        read_arp_an().unwrap_or_default()
    }
    #[cfg(target_os = "windows")]
    {
        read_windows_arp()
            .or_else(read_arp_a)
            .unwrap_or_default()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        HashMap::new()
    }
}

#[cfg(target_os = "linux")]
fn read_linux_proc_arp() -> Option<HashMap<String, String>> {
    let content = std::fs::read_to_string("/proc/net/arp").ok()?;
    Some(parse_proc_arp(&content))
}

#[cfg(target_os = "linux")]
fn read_ip_neigh() -> Option<HashMap<String, String>> {
    let output = Command::new("ip").args(["neigh", "show"]).output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(parse_ip_neigh(&String::from_utf8_lossy(&output.stdout)))
}

/// Parse `/proc/net/arp` text into IP → MAC.
pub fn parse_proc_arp(content: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in content.lines().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 4 {
            continue;
        }
        let ip = cols[0];
        let mac = cols[3];
        if let Some(norm) = normalize_mac(mac) {
            map.insert(ip.to_string(), norm);
        }
    }
    map
}

/// Parse `ip neigh show` text into IP → MAC.
pub fn parse_ip_neigh(text: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 5 {
            continue;
        }
        let ip = cols[0];
        if ip.parse::<Ipv4Addr>().is_err() {
            continue;
        }
        if let Some(idx) = cols.iter().position(|c| *c == "lladdr") {
            if let Some(mac) = cols.get(idx + 1).and_then(|m| normalize_mac(m)) {
                map.insert(ip.to_string(), mac);
            }
        }
    }
    map
}

#[cfg(target_os = "macos")]
fn read_arp_an() -> Option<HashMap<String, String>> {
    let output = Command::new("arp").arg("-an").output().ok()?;
    Some(parse_bsd_arp(&String::from_utf8_lossy(&output.stdout)))
}

#[cfg(target_os = "windows")]
fn read_windows_arp() -> Option<HashMap<String, String>> {
    let output = Command::new("arp").arg("-a").output().ok()?;
    if !output.status.success() {
        return None;
    }
    Some(parse_windows_arp(&String::from_utf8_lossy(&output.stdout)))
}

#[cfg(target_os = "windows")]
fn read_arp_a() -> Option<HashMap<String, String>> {
    read_windows_arp()
}

/// Parse BSD-style `arp -an` output into IP → MAC.
pub fn parse_bsd_arp(text: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in text.lines() {
        let Some(ip) = extract_ip_in_parens(line) else {
            continue;
        };
        let Some(mac) = extract_mac(line) else {
            continue;
        };
        map.insert(ip, mac);
    }
    map
}

fn extract_ip_in_parens(line: &str) -> Option<String> {
    let start = line.find('(')? + 1;
    let end = line[start..].find(')')? + start;
    let ip = &line[start..end];
    ip.parse::<Ipv4Addr>().ok()?;
    Some(ip.to_string())
}

fn extract_mac(line: &str) -> Option<String> {
    for tok in line.split_whitespace() {
        if let Some(mac) = normalize_mac(tok) {
            return Some(mac);
        }
    }
    None
}

/// Parse Windows `arp -a` text into IP → MAC.
pub fn parse_windows_arp(text: &str) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for line in text.lines() {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.len() < 2 {
            continue;
        }
        if cols[0].parse::<Ipv4Addr>().is_err() {
            continue;
        }
        if let Some(mac) = normalize_mac(cols[1]) {
            map.insert(cols[0].to_string(), mac);
        }
    }
    map
}

/// Normalize MAC addresses from common OS formats into `AA:BB:CC:DD:EE:FF`.
pub fn normalize_mac(raw: &str) -> Option<String> {
    let cleaned: String = raw
        .trim()
        .chars()
        .filter(|c| c.is_ascii_hexdigit())
        .collect::<String>()
        .to_uppercase();

    if cleaned.len() != 12 || cleaned == "000000000000" {
        return None;
    }

    Some(
        cleaned
            .as_bytes()
            .chunks(2)
            .map(|c| std::str::from_utf8(c).unwrap_or("00"))
            .collect::<Vec<_>>()
            .join(":"),
    )
}
