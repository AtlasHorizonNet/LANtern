use crate::network::identity;
use crate::network::NetworkInfo;
use ipnetwork::Ipv4Network;
use network_interface::{NetworkInterface, NetworkInterfaceConfig};
use std::net::Ipv4Addr;

/// Maximum hosts we will scan without an explicit override.
pub const MAX_SCAN_HOSTS: u32 = 1024;

/// List every scannable IPv4 network, best candidate first.
///
/// Preference order: private (RFC 1918) ranges before public ones, then
/// smaller subnets (more likely a home/office LAN than a corporate /8).
pub fn list_networks() -> Result<Vec<NetworkInfo>, String> {
    let interfaces =
        NetworkInterface::show().map_err(|e| format!("Failed to list interfaces: {e}"))?;

    let mut candidates = Vec::new();
    let search_domain = identity::detect_search_domain();

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

    candidates.sort_by(|a, b| {
        let score = |ip: Ipv4Addr, prefix: u8| -> (u8, u8) {
            let class = if is_rfc1918(ip) { 0 } else { 1 };
            (class, 32u8.saturating_sub(prefix))
        };
        score(a.1, a.3).cmp(&score(b.1, b.3))
    });

    Ok(candidates
        .into_iter()
        .map(|(name, local_ip, network, prefix)| {
            let host_count = network_host_count(network);
            let gateway = guess_gateway(network);
            let cidr = format!("{}/{}", network.network(), prefix);
            enrich_network(
                name,
                local_ip.to_string(),
                cidr,
                prefix,
                gateway.map(|g| g.to_string()),
                host_count,
                search_domain.clone(),
            )
        })
        .collect())
}

pub fn detect_network() -> Result<NetworkInfo, String> {
    list_networks()?
        .into_iter()
        .next()
        .ok_or_else(|| "No suitable IPv4 network interface found".to_string())
}

/// Validate a caller-supplied network selection and normalize derived fields.
///
/// The frontend passes back one of the `NetworkInfo` values from
/// `list_networks`, but nothing stops a stale or hand-crafted value from
/// arriving, so re-derive everything that scanning depends on from the CIDR
/// and make sure the local IP actually belongs to that subnet.
pub fn sanitize_network(info: NetworkInfo) -> Result<NetworkInfo, String> {
    let network: Ipv4Network = info
        .cidr
        .parse()
        .map_err(|e| format!("Invalid CIDR {}: {e}", info.cidr))?;

    let local: Ipv4Addr = info
        .local_ip
        .parse()
        .map_err(|e| format!("Invalid local IP {}: {e}", info.local_ip))?;

    if !network.contains(local) {
        return Err(format!(
            "Local IP {} is not inside subnet {}",
            info.local_ip, info.cidr
        ));
    }

    let prefix = network.prefix();
    if !(8..=30).contains(&prefix) {
        return Err(format!("Unsupported prefix length /{prefix}"));
    }

    let gateway = info
        .gateway
        .as_deref()
        .and_then(|g| g.parse::<Ipv4Addr>().ok())
        .filter(|g| network.contains(*g))
        .or_else(|| guess_gateway(network));

    let cidr = format!("{}/{}", network.network(), prefix);
    let search_domain = info
        .search_domain
        .clone()
        .or_else(identity::detect_search_domain);

    let mut enriched = enrich_network(
        info.interface_name,
        local.to_string(),
        cidr,
        prefix,
        gateway.map(|g| g.to_string()),
        network_host_count(network),
        search_domain,
    );
    // Preserve a caller-supplied SSID if detection fails mid-scan.
    if enriched.ssid.is_none() {
        enriched.ssid = info.ssid;
        if let Some(ssid) = enriched.ssid.clone() {
            enriched.media = identity::classify_media(&enriched.interface_name, Some(&ssid)).into();
            enriched.fingerprint = identity::fingerprint(
                &enriched.media,
                Some(&ssid),
                enriched.search_domain.as_deref(),
                &enriched.interface_name,
                &enriched.cidr,
                enriched.gateway.as_deref(),
            );
            enriched.auto_name = identity::auto_name(
                Some(&ssid),
                enriched.search_domain.as_deref(),
                &enriched.interface_name,
                &enriched.cidr,
            );
        }
    }
    if info.display_name.is_some() {
        enriched.display_name = info.display_name;
    }
    if info.external_ip.is_some() {
        enriched.external_ip = info.external_ip;
    }
    enriched.db_id = info.db_id;
    Ok(enriched)
}

fn enrich_network(
    interface_name: String,
    local_ip: String,
    cidr: String,
    prefix: u8,
    gateway: Option<String>,
    host_count: u32,
    search_domain: Option<String>,
) -> NetworkInfo {
    let ssid = identity::detect_ssid(&interface_name);
    let media = identity::classify_media(&interface_name, ssid.as_deref()).to_string();
    let fingerprint = identity::fingerprint(
        &media,
        ssid.as_deref(),
        search_domain.as_deref(),
        &interface_name,
        &cidr,
        gateway.as_deref(),
    );
    let auto_name = identity::auto_name(
        ssid.as_deref(),
        search_domain.as_deref(),
        &interface_name,
        &cidr,
    );
    NetworkInfo {
        interface_name,
        local_ip,
        cidr,
        prefix,
        gateway,
        host_count,
        fingerprint,
        display_name: Some(auto_name.clone()),
        auto_name,
        media,
        ssid,
        search_domain,
        external_ip: None,
        db_id: None,
    }
}

fn network_host_count(network: Ipv4Network) -> u32 {
    (network.size() as u64)
        .saturating_sub(2)
        .min(u32::MAX as u64) as u32
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
