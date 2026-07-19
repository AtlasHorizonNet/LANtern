//! DHCP discover-only diagnostics.
//!
//! Sends a DHCPDISCOVER and listens for DHCPOFFER responses. Never sends
//! DHCPREQUEST, so the machine's active lease is not accepted or replaced.

use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::time::{Duration, Instant};
use tokio::net::UdpSocket;
use tokio::time::timeout;

const DHCP_SERVER_PORT: u16 = 67;
const DHCP_CLIENT_PORT: u16 = 68;
const MAGIC_COOKIE: [u8; 4] = [0x63, 0x82, 0x53, 0x63];
const BOOTREQUEST: u8 = 1;
const BOOTREPLY: u8 = 2;
const HTYPE_ETHERNET: u8 = 1;
const DHCPDISCOVER: u8 = 1;
const DHCPOFFER: u8 = 2;
const DHCPNAK: u8 = 6;
const OPT_MESSAGE_TYPE: u8 = 53;
const OPT_SERVER_ID: u8 = 54;
const OPT_LEASE_TIME: u8 = 51;
const OPT_SUBNET_MASK: u8 = 1;
const OPT_ROUTER: u8 = 3;
const OPT_DNS: u8 = 6;
const OPT_DOMAIN: u8 = 15;
const OPT_PARAM_REQUEST: u8 = 55;
const OPT_END: u8 = 255;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DhcpOffer {
    pub server_ip: Option<String>,
    pub offered_ip: String,
    pub lease_seconds: Option<u32>,
    pub subnet_mask: Option<String>,
    pub gateway: Option<String>,
    pub dns_servers: Vec<String>,
    pub domain: Option<String>,
    pub latency_ms: f64,
    pub message_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DhcpDiscoverResult {
    pub success: bool,
    pub privilege_note: String,
    pub timeout_ms: u32,
    pub offers: Vec<DhcpOffer>,
    pub error: Option<String>,
}

pub fn privilege_note() -> String {
    "This test binds UDP port 68 briefly and only sends DHCPDISCOVER (no REQUEST), so it will not take a lease. Binding port 68 often requires administrator / root privileges on macOS and Linux.".into()
}

/// Run a discover-only DHCP probe on the given local interface IP.
pub async fn discover(local_ip: Ipv4Addr, wait: Duration) -> DhcpDiscoverResult {
    let note = privilege_note();
    let mac = local_mac_hint(local_ip).unwrap_or([0x02, 0x00, 0x00, 0x00, 0x00, 0x01]);
    let xid = rand::thread_rng().next_u32();

    let socket = match bind_client_socket(local_ip).await {
        Ok(s) => s,
        Err(err) => {
            return DhcpDiscoverResult {
                success: false,
                privilege_note: note,
                timeout_ms: wait.as_millis() as u32,
                offers: Vec::new(),
                error: Some(err),
            };
        }
    };

    let packet = build_discover(xid, &mac);
    if let Err(e) = socket
        .send_to(
            &packet,
            SocketAddrV4::new(Ipv4Addr::BROADCAST, DHCP_SERVER_PORT),
        )
        .await
    {
        return DhcpDiscoverResult {
            success: false,
            privilege_note: note,
            timeout_ms: wait.as_millis() as u32,
            offers: Vec::new(),
            error: Some(format!("Failed to send DHCPDISCOVER: {e}")),
        };
    }

    let started = Instant::now();
    let deadline = started + wait;
    let mut offers = Vec::new();
    let mut saw_nak = false;
    let mut buf = [0u8; 1500];

    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match timeout(remaining, socket.recv_from(&mut buf)).await {
            Ok(Ok((len, _src))) => {
                if let Some(offer) = parse_offer(&buf[..len], xid, started.elapsed()) {
                    if offer.message_type == "NAK" {
                        saw_nak = true;
                    } else {
                        offers.push(offer);
                    }
                }
            }
            Ok(Err(e)) => {
                return DhcpDiscoverResult {
                    success: false,
                    privilege_note: note,
                    timeout_ms: wait.as_millis() as u32,
                    offers,
                    error: Some(format!("Receive error: {e}")),
                };
            }
            Err(_) => break, // overall timeout
        }
    }

    if offers.is_empty() {
        let error = if saw_nak {
            Some("DHCP server responded with NAK (no offer).".into())
        } else {
            Some("No DHCP offer received before timeout.".into())
        };
        return DhcpDiscoverResult {
            success: false,
            privilege_note: note,
            timeout_ms: wait.as_millis() as u32,
            offers,
            error,
        };
    }

    DhcpDiscoverResult {
        success: true,
        privilege_note: note,
        timeout_ms: wait.as_millis() as u32,
        offers,
        error: None,
    }
}

async fn bind_client_socket(local_ip: Ipv4Addr) -> Result<UdpSocket, String> {
    let addr = SocketAddrV4::new(local_ip, DHCP_CLIENT_PORT);
    let socket = match UdpSocket::bind(addr).await {
        Ok(s) => s,
        Err(_) => {
            // Fallback: any address on port 68 (still privileged on many systems).
            UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, DHCP_CLIENT_PORT))
                .await
                .map_err(|e| {
                    format!(
                        "Could not bind UDP port {DHCP_CLIENT_PORT} ({e}). Run LANtern with elevated privileges to run the DHCP discover test."
                    )
                })?
        }
    };
    socket
        .set_broadcast(true)
        .map_err(|e| format!("Failed to enable broadcast: {e}"))?;
    Ok(socket)
}

fn build_discover(xid: u32, mac: &[u8; 6]) -> Vec<u8> {
    let mut pkt = vec![0u8; 240];
    pkt[0] = BOOTREQUEST;
    pkt[1] = HTYPE_ETHERNET;
    pkt[2] = 6; // hlen
    pkt[4..8].copy_from_slice(&xid.to_be_bytes());
    // Broadcast flag so replies are easier to receive without a claimed address.
    pkt[10] = 0x80;
    pkt[28..34].copy_from_slice(mac);
    pkt[236..240].copy_from_slice(&MAGIC_COOKIE);

    // Options
    pkt.push(OPT_MESSAGE_TYPE);
    pkt.push(1);
    pkt.push(DHCPDISCOVER);

    pkt.push(OPT_PARAM_REQUEST);
    pkt.push(4);
    pkt.extend_from_slice(&[OPT_SUBNET_MASK, OPT_ROUTER, OPT_DNS, OPT_DOMAIN]);

    pkt.push(OPT_END);
    // Pad to a reasonable minimum payload.
    while pkt.len() < 300 {
        pkt.push(0);
    }
    pkt
}

fn parse_offer(data: &[u8], expected_xid: u32, elapsed: Duration) -> Option<DhcpOffer> {
    if data.len() < 240 {
        return None;
    }
    if data[0] != BOOTREPLY {
        return None;
    }
    let xid = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
    if xid != expected_xid {
        return None;
    }
    if data[236..240] != MAGIC_COOKIE {
        return None;
    }

    let offered = Ipv4Addr::new(data[16], data[17], data[18], data[19]);
    let siaddr = Ipv4Addr::new(data[20], data[21], data[22], data[23]);

    let mut message_type = None;
    let mut server_id = None;
    let mut lease_seconds = None;
    let mut subnet_mask = None;
    let mut gateway = None;
    let mut dns_servers = Vec::new();
    let mut domain = None;

    let mut i = 240;
    while i < data.len() {
        let code = data[i];
        if code == OPT_END {
            break;
        }
        if code == 0 {
            i += 1;
            continue;
        }
        if i + 1 >= data.len() {
            break;
        }
        let len = data[i + 1] as usize;
        if i + 2 + len > data.len() {
            break;
        }
        let val = &data[i + 2..i + 2 + len];
        match code {
            OPT_MESSAGE_TYPE if len == 1 => message_type = Some(val[0]),
            OPT_SERVER_ID if len == 4 => {
                server_id = Some(Ipv4Addr::new(val[0], val[1], val[2], val[3]).to_string());
            }
            OPT_LEASE_TIME if len == 4 => {
                lease_seconds = Some(u32::from_be_bytes([val[0], val[1], val[2], val[3]]));
            }
            OPT_SUBNET_MASK if len == 4 => {
                subnet_mask = Some(Ipv4Addr::new(val[0], val[1], val[2], val[3]).to_string());
            }
            OPT_ROUTER if len >= 4 => {
                gateway = Some(Ipv4Addr::new(val[0], val[1], val[2], val[3]).to_string());
            }
            OPT_DNS => {
                for chunk in val.chunks_exact(4) {
                    dns_servers
                        .push(Ipv4Addr::new(chunk[0], chunk[1], chunk[2], chunk[3]).to_string());
                }
            }
            OPT_DOMAIN => {
                if let Ok(s) = std::str::from_utf8(val) {
                    let t = s.trim();
                    if !t.is_empty() {
                        domain = Some(t.to_string());
                    }
                }
            }
            _ => {}
        }
        i += 2 + len;
    }

    let message_type = match message_type? {
        DHCPOFFER => "OFFER".to_string(),
        DHCPNAK => "NAK".to_string(),
        _ => return None,
    };

    if message_type == "OFFER" && offered.is_unspecified() {
        return None;
    }

    Some(DhcpOffer {
        server_ip: server_id.or_else(|| {
            if siaddr.is_unspecified() {
                None
            } else {
                Some(siaddr.to_string())
            }
        }),
        offered_ip: offered.to_string(),
        lease_seconds,
        subnet_mask,
        gateway,
        dns_servers,
        domain,
        latency_ms: elapsed.as_secs_f64() * 1000.0,
        message_type,
    })
}

fn local_mac_hint(local_ip: Ipv4Addr) -> Option<[u8; 6]> {
    let map = crate::network::neighbors::read_neighbors();
    let mac = map.get(&local_ip.to_string())?;
    let parts = crate::network::neighbors::normalize_mac(mac)?;
    let bytes: Vec<u8> = parts
        .split(':')
        .filter_map(|p| u8::from_str_radix(p, 16).ok())
        .collect();
    if bytes.len() != 6 {
        return None;
    }
    Some([bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5]])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_discover_with_magic_cookie_and_type() {
        let pkt = build_discover(0x01020304, &[0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
        assert_eq!(pkt[0], BOOTREQUEST);
        assert_eq!(&pkt[4..8], &[1, 2, 3, 4]);
        assert_eq!(&pkt[28..34], &[0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]);
        assert_eq!(&pkt[236..240], &MAGIC_COOKIE);
        assert!(pkt
            .windows(3)
            .any(|w| w == [OPT_MESSAGE_TYPE, 1, DHCPDISCOVER]));
    }

    #[test]
    fn parses_minimal_offer() {
        let xid = 0x11223344u32;
        let mut pkt = build_discover(xid, &[1, 2, 3, 4, 5, 6]);
        pkt[0] = BOOTREPLY;
        // yiaddr 192.168.1.50
        pkt[16..20].copy_from_slice(&[192, 168, 1, 50]);
        // siaddr 192.168.1.1
        pkt[20..24].copy_from_slice(&[192, 168, 1, 1]);
        // Replace options with OFFER + server id + lease
        pkt.truncate(240);
        pkt.extend_from_slice(&[OPT_MESSAGE_TYPE, 1, DHCPOFFER]);
        pkt.extend_from_slice(&[OPT_SERVER_ID, 4, 192, 168, 1, 1]);
        pkt.extend_from_slice(&[OPT_LEASE_TIME, 4, 0, 0, 0x0e, 0x10]); // 3600
        pkt.extend_from_slice(&[OPT_ROUTER, 4, 192, 168, 1, 1]);
        pkt.extend_from_slice(&[OPT_DNS, 4, 1, 1, 1, 1]);
        pkt.push(OPT_END);

        let offer = parse_offer(&pkt, xid, Duration::from_millis(12)).unwrap();
        assert_eq!(offer.message_type, "OFFER");
        assert_eq!(offer.offered_ip, "192.168.1.50");
        assert_eq!(offer.server_ip.as_deref(), Some("192.168.1.1"));
        assert_eq!(offer.lease_seconds, Some(3600));
        assert_eq!(offer.gateway.as_deref(), Some("192.168.1.1"));
        assert_eq!(offer.dns_servers, vec!["1.1.1.1".to_string()]);
    }
}
