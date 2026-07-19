//! Wake-on-LAN magic packet sender.

use crate::network::neighbors::normalize_mac;
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, SocketAddrV4};
use tokio::net::UdpSocket;

const WOL_PORT: u16 = 9;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WakeResult {
    pub success: bool,
    pub mac: String,
    pub broadcast: String,
    pub message: String,
}

/// Build and broadcast a WoL magic packet for `mac`.
///
/// `broadcast` defaults to `255.255.255.255` when omitted. The target NIC must
/// support Wake-on-LAN and usually must be enabled in firmware/OS power settings.
pub async fn wake_on_lan(mac: &str, broadcast: Option<&str>) -> Result<WakeResult, String> {
    let normalized = normalize_mac(mac).ok_or_else(|| format!("Invalid MAC address: {mac}"))?;
    let bytes = mac_bytes(&normalized)?;
    let packet = magic_packet(&bytes);

    let bcast: Ipv4Addr = match broadcast.map(str::trim).filter(|s| !s.is_empty()) {
        Some(raw) => raw
            .parse()
            .map_err(|e| format!("Invalid broadcast address {raw}: {e}"))?,
        None => Ipv4Addr::BROADCAST,
    };

    let socket = UdpSocket::bind(SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 0))
        .await
        .map_err(|e| format!("Failed to bind UDP socket: {e}"))?;
    socket
        .set_broadcast(true)
        .map_err(|e| format!("Failed to enable broadcast: {e}"))?;

    let dest = SocketAddrV4::new(bcast, WOL_PORT);
    socket
        .send_to(&packet, dest)
        .await
        .map_err(|e| format!("Failed to send magic packet: {e}"))?;

    // Also try the alternate WoL port used by some stacks.
    let _ = socket
        .send_to(&packet, SocketAddrV4::new(bcast, 7))
        .await;

    Ok(WakeResult {
        success: true,
        message: format!("Magic packet sent to {normalized} via {bcast}"),
        mac: normalized,
        broadcast: bcast.to_string(),
    })
}

fn mac_bytes(normalized: &str) -> Result<[u8; 6], String> {
    let mut out = [0u8; 6];
    for (i, part) in normalized.split(':').enumerate() {
        out[i] = u8::from_str_radix(part, 16)
            .map_err(|e| format!("Invalid MAC octet '{part}': {e}"))?;
    }
    Ok(out)
}

fn magic_packet(mac: &[u8; 6]) -> [u8; 102] {
    let mut packet = [0u8; 102];
    packet[..6].fill(0xff);
    for i in 0..16 {
        let start = 6 + i * 6;
        packet[start..start + 6].copy_from_slice(mac);
    }
    packet
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magic_packet_layout() {
        let mac = [0x01, 0x23, 0x45, 0x67, 0x89, 0xab];
        let pkt = magic_packet(&mac);
        assert!(pkt[..6].iter().all(|b| *b == 0xff));
        assert_eq!(&pkt[6..12], &mac);
        assert_eq!(&pkt[96..102], &mac);
        assert_eq!(pkt.len(), 102);
    }

    #[test]
    fn rejects_bad_mac() {
        assert!(normalize_mac("zz:zz:zz:zz:zz:zz").is_none());
    }
}
