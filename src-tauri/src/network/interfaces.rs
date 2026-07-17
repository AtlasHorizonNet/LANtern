use crate::network::NetworkInfo;
use ipnetwork::Ipv4Network;
use network_interface::{NetworkInterface, NetworkInterfaceConfig};
use std::net::Ipv4Addr;

/// Maximum hosts we will scan without an explicit override.
pub const MAX_SCAN_HOSTS: u32 = 1024;

pub fn detect_network() -> Result<NetworkInfo, String> {
    let interfaces =
        NetworkInterface::show().map_err(|e| format!("Failed to list interfaces: {e}"))?;

    let mut candidates = Vec::new();

    for iface in interfaces {
        if iface.name.starts_with("lo")
            || iface.name.starts_with("docker")
            || iface.name.starts_with("br-")
            || iface.name.starts_with("veth")
            || iface.name.starts_with("vmnet")
            || iface.name.starts_with("utun")
            || iface.name.starts_with("awdl")
            || iface.name.starts_with("llw")
        {
            continue;
        }

        for addr in &iface.addr {
            let ip = match addr.ip() {
                std::net::IpAddr::V4(v4) => v4,
                _ => continue,
            };

            if ip.is_loopback() || ip.is_link_local() || ip.is_unspecified() {
                continue;
            }

            let prefix = match addr.netmask() {
                Some(std::net::IpAddr::V4(mask)) => mask_to_prefix(mask),
                _ => continue,
            };

            if !(8..=30).contains(&prefix) {
                continue;
            }

            let network = Ipv4Network::new(ip, prefix)
                .map_err(|e| format!("Invalid network for {ip}/{prefix}: {e}"))?;

            candidates.push((iface.name.clone(), ip, network, prefix));
        }
    }

    // Prefer private LAN ranges, then smaller subnets (more likely home/office).
    candidates.sort_by(|a, b| {
        let score = |ip: Ipv4Addr, prefix: u8| -> (u8, u8) {
            let class = if is_rfc1918(ip) { 0 } else { 1 };
            (class, 32u8.saturating_sub(prefix))
        };
        score(a.1, a.3).cmp(&score(b.1, b.3))
    });

    let (name, local_ip, network, prefix) = candidates
        .into_iter()
        .next()
        .ok_or_else(|| "No suitable IPv4 network interface found".to_string())?;

    let host_count = (network.size() as u64)
        .saturating_sub(2)
        .min(u32::MAX as u64) as u32;
    let gateway = guess_gateway(network);

    Ok(NetworkInfo {
        interface_name: name,
        local_ip: local_ip.to_string(),
        cidr: format!("{}/{}", network.network(), prefix),
        prefix,
        gateway: gateway.map(|g| g.to_string()),
        host_count,
    })
}

pub fn mask_to_prefix(mask: Ipv4Addr) -> u8 {
    u32::from(mask).count_ones() as u8
}

pub fn is_rfc1918(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    o[0] == 10 || (o[0] == 172 && (16..=31).contains(&o[1])) || (o[0] == 192 && o[1] == 168)
}

pub fn guess_gateway(network: Ipv4Network) -> Option<Ipv4Addr> {
    // Common home-router convention: first usable host.
    network.nth(1)
}

pub fn hosts_to_scan(info: &NetworkInfo) -> Result<Vec<Ipv4Addr>, String> {
    let network: Ipv4Network = info
        .cidr
        .parse()
        .map_err(|e| format!("Invalid CIDR {}: {e}", info.cidr))?;

    if info.host_count > MAX_SCAN_HOSTS {
        return Err(format!(
            "Subnet {} has {} hosts (max {}). Narrower networks are required for a full scan.",
            info.cidr, info.host_count, MAX_SCAN_HOSTS
        ));
    }

    let local: Ipv4Addr = info
        .local_ip
        .parse()
        .map_err(|e| format!("Invalid local IP: {e}"))?;

    let mut hosts = Vec::new();
    for ip in network.iter() {
        if ip == network.network() || ip == network.broadcast() {
            continue;
        }
        // Always include local and likely gateway even if somehow filtered.
        let _ = local;
        hosts.push(ip);
    }
    Ok(hosts)
}
