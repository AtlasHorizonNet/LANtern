//! Unit and integration tests for LANtern helpers (no GUI required).

use lantern_lib::network::{self, Device, NetworkInfo};
use lantern_lib::store::AppState;
use std::net::Ipv4Addr;
use std::time::{SystemTime, UNIX_EPOCH};

fn sample_network(cidr: &str, local_ip: &str, host_count: u32) -> NetworkInfo {
    let prefix: u8 = cidr.split('/').nth(1).unwrap().parse().unwrap();
    NetworkInfo {
        interface_name: "eth0".into(),
        local_ip: local_ip.into(),
        cidr: cidr.into(),
        prefix,
        gateway: Some("192.168.1.1".into()),
        host_count,
        fingerprint: format!("net:eth0|{cidr}|192.168.1.1"),
        display_name: Some(format!("eth0 ({cidr})")),
        auto_name: format!("eth0 ({cidr})"),
        media: "ethernet".into(),
        ssid: None,
        search_domain: None,
        external_ip: None,
        db_id: None,
    }
}

fn sample_device(ip: &str, mac: Option<&str>) -> Device {
    Device {
        ip: ip.into(),
        mac: mac.map(str::to_string),
        hostname: None,
        vendor: None,
        nickname: None,
        online: true,
        last_seen: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64,
        is_gateway: false,
        is_local: false,
    }
}

#[test]
fn oui_looks_up_known_xerox_prefix() {
    let vendor = network::oui::lookup_vendor("00:00:01:aa:bb:cc");
    assert!(vendor.is_some(), "expected OUI hit for 00:00:01");
}

#[test]
fn oui_accepts_dashed_and_bare_macs() {
    let a = network::oui::lookup_vendor("00-00-01-aa-bb-cc");
    let b = network::oui::lookup_vendor("000001aabbcc");
    assert_eq!(a, b);
    assert!(a.is_some());
}

#[test]
fn oui_rejects_short_mac() {
    assert!(network::oui::lookup_vendor("00:00").is_none());
}

#[test]
fn normalize_mac_formats() {
    assert_eq!(
        network::neighbors::normalize_mac("aa-bb-cc-dd-ee-ff").as_deref(),
        Some("AA:BB:CC:DD:EE:FF")
    );
    assert_eq!(
        network::neighbors::normalize_mac("aabbccddeeff").as_deref(),
        Some("AA:BB:CC:DD:EE:FF")
    );
    assert!(network::neighbors::normalize_mac("00:00:00:00:00:00").is_none());
    assert!(network::neighbors::normalize_mac("not-a-mac").is_none());
}

#[test]
fn parse_proc_arp_skips_incomplete_entries() {
    let text = "\
IP address       HW type     Flags       HW address            Mask     Device
192.168.1.1      0x1         0x2         aa:bb:cc:dd:ee:ff     *        eth0
192.168.1.2      0x1         0x0         00:00:00:00:00:00     *        eth0
";
    let map = network::neighbors::parse_proc_arp(text);
    assert_eq!(map.len(), 1);
    assert_eq!(map.get("192.168.1.1").unwrap(), "AA:BB:CC:DD:EE:FF");
}

#[test]
fn parse_ip_neigh_extracts_lladdr() {
    let text = "\
192.168.1.1 dev eth0 lladdr aa:bb:cc:dd:ee:01 REACHABLE
192.168.1.5 dev eth0 FAILED
fe80::1 dev eth0 lladdr aa:bb:cc:dd:ee:02 STALE
";
    let map = network::neighbors::parse_ip_neigh(text);
    assert_eq!(map.len(), 1);
    assert_eq!(map.get("192.168.1.1").unwrap(), "AA:BB:CC:DD:EE:01");
}

#[test]
fn parse_bsd_arp_extracts_paren_ips() {
    let text = "? (192.168.1.1) at aa:bb:cc:dd:ee:ff on en0 ifscope [ethernet]\n";
    let map = network::neighbors::parse_bsd_arp(text);
    assert_eq!(map.get("192.168.1.1").unwrap(), "AA:BB:CC:DD:EE:FF");
}

#[test]
fn parse_windows_arp_table() {
    let text = "\
Interface: 192.168.1.10 --- 0xb
  Internet Address      Physical Address      Type
  192.168.1.1           aa-bb-cc-dd-ee-ff     dynamic
  192.168.1.255         ff-ff-ff-ff-ff-ff     static
";
    let map = network::neighbors::parse_windows_arp(text);
    assert_eq!(map.get("192.168.1.1").unwrap(), "AA:BB:CC:DD:EE:FF");
    assert_eq!(map.get("192.168.1.255").unwrap(), "FF:FF:FF:FF:FF:FF");
}

#[test]
fn mask_to_prefix_common_masks() {
    assert_eq!(
        network::interfaces::mask_to_prefix(Ipv4Addr::new(255, 255, 255, 0)),
        24
    );
    assert_eq!(
        network::interfaces::mask_to_prefix(Ipv4Addr::new(255, 255, 0, 0)),
        16
    );
    assert_eq!(
        network::interfaces::mask_to_prefix(Ipv4Addr::new(255, 255, 255, 252)),
        30
    );
}

#[test]
fn rfc1918_classification() {
    assert!(network::interfaces::is_rfc1918(Ipv4Addr::new(10, 0, 0, 1)));
    assert!(network::interfaces::is_rfc1918(Ipv4Addr::new(
        172, 16, 0, 1
    )));
    assert!(network::interfaces::is_rfc1918(Ipv4Addr::new(
        192, 168, 1, 1
    )));
    assert!(!network::interfaces::is_rfc1918(Ipv4Addr::new(8, 8, 8, 8)));
    assert!(!network::interfaces::is_rfc1918(Ipv4Addr::new(
        172, 15, 0, 1
    )));
}

#[test]
fn guess_gateway_is_first_usable_host() {
    let net: ipnetwork::Ipv4Network = "192.168.1.0/24".parse().unwrap();
    assert_eq!(
        network::interfaces::guess_gateway(net),
        Some(Ipv4Addr::new(192, 168, 1, 1))
    );
}

#[test]
fn hosts_to_scan_excludes_network_and_broadcast() {
    let info = sample_network("192.168.1.0/30", "192.168.1.2", 2);
    let hosts = network::interfaces::hosts_to_scan(&info).unwrap();
    assert_eq!(hosts.len(), 2);
    assert!(!hosts.contains(&Ipv4Addr::new(192, 168, 1, 0)));
    assert!(!hosts.contains(&Ipv4Addr::new(192, 168, 1, 3)));
}

#[test]
fn hosts_to_scan_rejects_oversized_subnets() {
    let info = sample_network("10.0.0.0/16", "10.0.0.1", 65_534);
    let err = network::interfaces::hosts_to_scan(&info).unwrap_err();
    assert!(err.contains("max"));
}

#[test]
fn device_store_persists_nicknames_per_network() {
    let dir = std::env::temp_dir().join(format!(
        "lantern-test-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let db_path = dir.join("lantern.db");
    let legacy = dir.join("devices.json");
    let state = AppState::load(db_path.clone(), legacy.clone()).unwrap();

    let mut net = sample_network("192.168.1.0/24", "192.168.1.10", 254);
    net.fingerprint = "wifi:home|192.168.1.0/24|192.168.1.1".into();
    let record = state.db.lock().upsert_network(&net).unwrap();

    let device = sample_device("192.168.1.20", Some("AA:BB:CC:DD:EE:FF"));
    state
        .db
        .lock()
        .replace_network_devices(record.id, std::slice::from_ref(&device))
        .unwrap();
    state
        .set_nickname("AA:BB:CC:DD:EE:FF", Some("  Living room TV  ".into()))
        .unwrap();

    let nicknames = state.nicknames();
    assert_eq!(
        nicknames.get("AA:BB:CC:DD:EE:FF").map(String::as_str),
        Some("Living room TV")
    );

    let listed = state
        .db
        .lock()
        .devices_for_network(&net.fingerprint)
        .unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].nickname.as_deref(), Some("Living room TV"));

    // Reload from disk
    let reloaded = AppState::load(db_path, legacy).unwrap();
    assert_eq!(
        reloaded
            .nicknames()
            .get("AA:BB:CC:DD:EE:FF")
            .map(String::as_str),
        Some("Living room TV")
    );
    assert_eq!(
        reloaded
            .db
            .lock()
            .devices_for_network(&net.fingerprint)
            .unwrap()
            .len(),
        1
    );

    // Clearing nickname
    reloaded
        .set_nickname("AA:BB:CC:DD:EE:FF", Some("".into()))
        .unwrap();
    assert!(reloaded.nicknames().is_empty());

    let _ = std::fs::remove_dir_all(dir);
}

fn bare_network(
    interface_name: &str,
    cidr: &str,
    local_ip: &str,
    gateway: Option<&str>,
    host_count: u32,
) -> NetworkInfo {
    let prefix: u8 = cidr
        .split('/')
        .nth(1)
        .and_then(|p| p.parse().ok())
        .unwrap_or(24);
    NetworkInfo {
        interface_name: interface_name.into(),
        local_ip: local_ip.into(),
        cidr: cidr.into(),
        prefix,
        gateway: gateway.map(str::to_string),
        host_count,
        fingerprint: String::new(),
        display_name: None,
        auto_name: String::new(),
        media: "unknown".into(),
        ssid: None,
        search_domain: None,
        external_ip: None,
        db_id: None,
    }
}

#[test]
fn sanitize_network_normalizes_valid_selection() {
    let info = bare_network("eth1", "192.168.5.77/24", "192.168.5.77", None, 0);
    let out = network::interfaces::sanitize_network(info).unwrap();
    assert_eq!(out.cidr, "192.168.5.0/24");
    assert_eq!(out.prefix, 24);
    assert_eq!(out.host_count, 254);
    assert_eq!(out.gateway.as_deref(), Some("192.168.5.1"));
    assert_eq!(out.interface_name, "eth1");
    assert!(!out.fingerprint.is_empty());
}

#[test]
fn sanitize_network_keeps_valid_gateway() {
    let info = bare_network("eth0", "10.1.2.0/24", "10.1.2.30", Some("10.1.2.254"), 254);
    let out = network::interfaces::sanitize_network(info).unwrap();
    assert_eq!(out.gateway.as_deref(), Some("10.1.2.254"));
}

#[test]
fn sanitize_network_replaces_foreign_gateway() {
    let info = bare_network("eth0", "10.1.2.0/24", "10.1.2.30", Some("192.168.1.1"), 254);
    let out = network::interfaces::sanitize_network(info).unwrap();
    // Gateway outside the subnet is discarded in favor of the convention guess.
    assert_eq!(out.gateway.as_deref(), Some("10.1.2.1"));
}

#[test]
fn sanitize_network_rejects_ip_outside_subnet() {
    let info = bare_network("eth0", "192.168.1.0/24", "10.0.0.5", None, 254);
    let err = network::interfaces::sanitize_network(info).unwrap_err();
    assert!(err.contains("not inside"), "unexpected error: {err}");
}

#[test]
fn sanitize_network_rejects_bad_cidr() {
    let info = bare_network("eth0", "not-a-cidr", "10.0.0.5", None, 0);
    assert!(network::interfaces::sanitize_network(info).is_err());
}

#[test]
fn list_networks_contains_detected_default() {
    match (
        network::interfaces::list_networks(),
        network::interfaces::detect_network(),
    ) {
        (Ok(list), Ok(default)) => {
            assert!(!list.is_empty());
            // detect_network must be the first (best) candidate.
            assert_eq!(list[0].cidr, default.cidr);
            assert_eq!(list[0].local_ip, default.local_ip);
            for n in &list {
                assert!(n.cidr.contains('/'));
                assert!((8..=30).contains(&n.prefix));
            }
        }
        (Err(e), _) | (_, Err(e)) => eprintln!("skipping interface assertions: {e}"),
    }
}

#[test]
fn ping_parse_linux_output() {
    let out = "\
PING 192.168.1.1 (192.168.1.1) 56(84) bytes of data.
64 bytes from 192.168.1.1: icmp_seq=1 ttl=64 time=0.845 ms

--- 192.168.1.1 ping statistics ---
1 packets transmitted, 1 received, 0% packet loss, time 0ms
";
    assert!(network::ping::output_indicates_reply(out));
    assert_eq!(network::ping::parse_ping_time_ms(out), Some(0.845));
}

#[test]
fn ping_parse_macos_output() {
    let out = "\
PING 10.0.0.1 (10.0.0.1): 56 data bytes
64 bytes from 10.0.0.1: icmp_seq=0 ttl=64 time=1.334 ms
";
    assert!(network::ping::output_indicates_reply(out));
    assert_eq!(network::ping::parse_ping_time_ms(out), Some(1.334));
}

#[test]
fn ping_parse_windows_output() {
    let out = "\
Pinging 192.168.1.1 with 32 bytes of data:
Reply from 192.168.1.1: bytes=32 time=3ms TTL=64
";
    assert!(network::ping::output_indicates_reply(out));
    assert_eq!(network::ping::parse_ping_time_ms(out), Some(3.0));
}

#[test]
fn ping_parse_windows_sub_millisecond() {
    let out = "Reply from 127.0.0.1: bytes=32 time<1ms TTL=128\n";
    assert!(network::ping::output_indicates_reply(out));
    assert_eq!(network::ping::parse_ping_time_ms(out), Some(1.0));
}

#[test]
fn ping_parse_localized_output() {
    // German Windows uses "Zeit=" but keeps "TTL=" and the ms unit.
    let out = "Antwort von 192.168.1.1: Bytes=32 Zeit=2ms TTL=64\n";
    assert!(network::ping::output_indicates_reply(out));
    assert_eq!(network::ping::parse_ping_time_ms(out), Some(2.0));
}

#[test]
fn ping_unreachable_is_not_a_reply() {
    // Windows exits 0 here; the missing TTL= marks it as a failure.
    let out = "Reply from 192.168.1.10: Destination host unreachable.\n";
    assert!(!network::ping::output_indicates_reply(out));
}

#[test]
fn ping_parse_rejects_unrelated_ms_text() {
    let out = "1 packets transmitted, 0 received, 100% packet loss, time 0ms\n";
    assert!(!network::ping::output_indicates_reply(out));
    assert_eq!(network::ping::parse_ping_time_ms(out), None);
}

#[tokio::test]
async fn ping_loopback_succeeds() {
    let outcome = network::ping::ping_once(Ipv4Addr::LOCALHOST).await;
    // Loopback answers ICMP where ping is available; otherwise the TCP
    // fallback sees an immediate refusal, which also proves liveness.
    assert!(
        outcome.success,
        "loopback ping failed via {}: {:?}",
        outcome.method, outcome.error
    );
    println!(
        "loopback ping method={} latency={:?}",
        outcome.method, outcome.latency_ms
    );
}

#[test]
fn detect_network_info_or_skip() {
    match network::scan::network_info() {
        Ok(info) => {
            assert!(!info.local_ip.is_empty());
            assert!(info.cidr.contains('/'));
            assert!((8..=30).contains(&info.prefix));
        }
        Err(e) => eprintln!("skipping interface assertion: {e}"),
    }
}

#[test]
fn neighbors_readable_without_panic() {
    let map = network::neighbors::read_neighbors();
    println!("neighbor entries: {}", map.len());
}

#[tokio::test]
async fn local_udp_seed_smoke() {
    match network::scan::network_info() {
        Ok(info) => {
            let ip: Ipv4Addr = info.local_ip.parse().expect("local ip");
            let addr = std::net::SocketAddr::from((ip, 9));
            let sock = tokio::net::UdpSocket::bind("0.0.0.0:0").await;
            assert!(sock.is_ok(), "udp bind should work");
            if let Ok(s) = sock {
                let _ = s.send_to(&[0u8], addr).await;
            }
        }
        Err(e) => eprintln!("no network: {e}"),
    }
}
