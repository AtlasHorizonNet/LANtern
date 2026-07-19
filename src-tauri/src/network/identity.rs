//! Network identity helpers: SSID / search-domain detection, fingerprinting,
//! friendly auto-names, and external (WAN) IP lookup.

use std::process::Command;

/// Best-effort Wi-Fi SSID for the given interface.
pub fn detect_ssid(interface: &str) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        if let Some(ssid) = run_capture("networksetup", &["-getairportnetwork", interface])
            .and_then(|out| {
                out.strip_prefix("Current Wi-Fi Network: ")
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty() && !s.contains("not associated"))
            })
        {
            return Some(ssid);
        }
        // Fallback used on newer macOS where networksetup may not work.
        run_capture("ipconfig", &["getsummary", interface]).and_then(|out| {
            out.lines().find_map(|line| {
                let line = line.trim();
                line.strip_prefix("SSID : ")
                    .or_else(|| line.strip_prefix("SSID: "))
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            })
        })
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(ssid) = run_capture("iwgetid", &["-r", interface])
            .or_else(|| run_capture("iwgetid", &["-r"]))
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
        {
            return Some(ssid);
        }
        // NetworkManager fallback.
        if let Some(out) = run_capture("nmcli", &["-t", "-f", "DEVICE,ACTIVE,SSID", "dev", "wifi"])
        {
            for line in out.lines() {
                let parts: Vec<_> = line.splitn(3, ':').collect();
                if parts.len() == 3 && parts[0] == interface && parts[1] == "yes" {
                    let ssid = parts[2].trim();
                    if !ssid.is_empty() {
                        return Some(ssid.to_string());
                    }
                }
            }
        }
        None
    }

    #[cfg(target_os = "windows")]
    {
        let _ = interface;
        run_capture("netsh", &["wlan", "show", "interfaces"]).and_then(|out| {
            out.lines().find_map(|line| {
                let line = line.trim();
                let lower = line.to_ascii_lowercase();
                if lower.starts_with("ssid") && !lower.starts_with("bssid") {
                    line.split_once(':')
                        .map(|(_, v)| v.trim().to_string())
                        .filter(|s| !s.is_empty())
                } else {
                    None
                }
            })
        })
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        let _ = interface;
        None
    }
}

/// DNS search/domain from the system resolver configuration.
pub fn detect_search_domain() -> Option<String> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let contents = std::fs::read_to_string("/etc/resolv.conf").ok()?;
        for line in contents.lines() {
            let line = line.trim();
            if let Some(rest) = line
                .strip_prefix("search ")
                .or_else(|| line.strip_prefix("domain "))
            {
                let domain = rest.split_whitespace().next()?.trim().to_string();
                if !domain.is_empty() && domain != "." {
                    return Some(domain);
                }
            }
        }
        None
    }

    #[cfg(target_os = "windows")]
    {
        // Best-effort: parse `ipconfig /all` for "Connection-specific DNS Suffix".
        run_capture("ipconfig", &["/all"]).and_then(|out| {
            out.lines().find_map(|line| {
                let lower = line.to_ascii_lowercase();
                if lower.contains("connection-specific dns suffix")
                    || lower.contains("primary dns suffix")
                {
                    line.split_once(':')
                        .map(|(_, v)| v.trim().to_string())
                        .filter(|s| !s.is_empty())
                } else {
                    None
                }
            })
        })
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        None
    }
}

pub fn classify_media(interface: &str, ssid: Option<&str>) -> &'static str {
    if ssid.is_some() {
        return "wifi";
    }
    let lower = interface.to_ascii_lowercase();
    if lower.starts_with("wl")
        || lower.contains("wlan")
        || lower.contains("wifi")
        || lower.contains("wi-fi")
    {
        return "wifi";
    }
    if lower.starts_with("eth")
        || lower.starts_with("enp")
        || lower.starts_with("ens")
        || lower.starts_with("eno")
        || lower.starts_with("em")
        || lower.contains("ethernet")
    {
        return "ethernet";
    }
    // macOS en* and many others are ambiguous without SSID.
    "unknown"
}

/// Stable-ish identity key used to reattach renames and scope device caches.
///
/// Prefer SSID when present, else DNS search domain, else interface name —
/// always combined with CIDR and gateway so identical guest SSIDs on different
/// LANs do not collide as easily.
pub fn fingerprint(
    media: &str,
    ssid: Option<&str>,
    search_domain: Option<&str>,
    interface: &str,
    cidr: &str,
    gateway: Option<&str>,
) -> String {
    let gw = gateway.unwrap_or("");
    if let Some(ssid) = ssid.filter(|s| !s.is_empty()) {
        return format!("wifi:{}|{cidr}|{gw}", ssid.to_ascii_lowercase());
    }
    if let Some(domain) = search_domain.filter(|s| !s.is_empty()) {
        return format!("lan:{}|{cidr}|{gw}", domain.to_ascii_lowercase());
    }
    format!("{media}:{interface}|{cidr}|{gw}")
}

pub fn auto_name(
    ssid: Option<&str>,
    search_domain: Option<&str>,
    interface: &str,
    cidr: &str,
) -> String {
    if let Some(ssid) = ssid.filter(|s| !s.is_empty()) {
        return ssid.to_string();
    }
    if let Some(domain) = search_domain.filter(|s| !s.is_empty()) {
        return domain.to_string();
    }
    format!("{interface} ({cidr})")
}

/// Public WAN IP via a couple of lightweight HTTPS endpoints.
pub async fn fetch_external_ip() -> Option<String> {
    const ENDPOINTS: &[&str] = &[
        "https://api.ipify.org",
        "https://ifconfig.me/ip",
        "https://icanhazip.com",
    ];
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .user_agent("LANtern/0.4")
        .build()
        .ok()?;

    for url in ENDPOINTS {
        if let Ok(resp) = client.get(*url).send().await {
            if let Ok(text) = resp.text().await {
                let ip = text.trim();
                if is_plausible_ip(ip) {
                    return Some(ip.to_string());
                }
            }
        }
    }
    None
}

fn is_plausible_ip(value: &str) -> bool {
    if value.parse::<std::net::Ipv4Addr>().is_ok() {
        return true;
    }
    value.parse::<std::net::Ipv6Addr>().is_ok()
}

fn run_capture(program: &str, args: &[&str]) -> Option<String> {
    let output = Command::new(program).args(args).output().ok()?;
    if !output.status.success() && output.stdout.is_empty() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).to_string();
    if text.trim().is_empty() {
        None
    } else {
        Some(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_prefers_ssid() {
        let fp = fingerprint(
            "wifi",
            Some("Cafe"),
            Some("lan"),
            "wlan0",
            "192.168.1.0/24",
            Some("192.168.1.1"),
        );
        assert_eq!(fp, "wifi:cafe|192.168.1.0/24|192.168.1.1");
    }

    #[test]
    fn fingerprint_falls_back_to_search_domain() {
        let fp = fingerprint(
            "ethernet",
            None,
            Some("office.local"),
            "eth0",
            "10.0.0.0/24",
            Some("10.0.0.1"),
        );
        assert_eq!(fp, "lan:office.local|10.0.0.0/24|10.0.0.1");
    }

    #[test]
    fn auto_name_priority() {
        assert_eq!(
            auto_name(Some("Home"), Some("lan"), "wlan0", "192.168.1.0/24"),
            "Home"
        );
        assert_eq!(
            auto_name(None, Some("lan"), "eth0", "192.168.1.0/24"),
            "lan"
        );
        assert_eq!(
            auto_name(None, None, "eth0", "192.168.1.0/24"),
            "eth0 (192.168.1.0/24)"
        );
    }
}
