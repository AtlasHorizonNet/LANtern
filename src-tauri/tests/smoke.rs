//! Smoke tests for network helpers (no GUI required).

use scanapp_lib::network;

#[test]
fn oui_looks_up_known_xerox_prefix() {
    // 00:00:01 is Xerox in the bundled manuf extract
    let vendor = network::oui::lookup_vendor("00:00:01:aa:bb:cc");
    assert!(vendor.is_some(), "expected OUI hit for 00:00:01");
}

#[test]
fn normalize_network_info_or_skip() {
    // Cloud/CI environments may lack a usable LAN interface.
    match network::scan::network_info() {
        Ok(info) => {
            assert!(!info.local_ip.is_empty());
            assert!(info.cidr.contains('/'));
            assert!(info.prefix >= 8 && info.prefix <= 30);
            println!("detected {}", info.cidr);
        }
        Err(e) => {
            eprintln!("skipping interface assertion: {e}");
        }
    }
}

#[test]
fn neighbors_readable() {
    let map = network::neighbors::read_neighbors();
    // Just ensure it doesn't panic; empty is fine in containers.
    println!("neighbor entries: {}", map.len());
}


#[tokio::test]
async fn local_ip_tcp_probe_smoke() {
    match scanapp_lib::network::scan::network_info() {
        Ok(info) => {
            let ip: std::net::Ipv4Addr = info.local_ip.parse().expect("local ip");
            let addr = std::net::SocketAddr::from((ip, 9));
            // Binding/connecting to self may vary; just ensure socket APIs work.
            let sock = tokio::net::UdpSocket::bind("0.0.0.0:0").await;
            assert!(sock.is_ok(), "udp bind should work");
            if let Ok(s) = sock {
                let _ = s.send_to(&[0u8], addr).await;
            }
            let neighbors = scanapp_lib::network::neighbors::read_neighbors();
            println!("local {} neighbors {}", info.local_ip, neighbors.len());
        }
        Err(e) => eprintln!("no network: {e}"),
    }
}
