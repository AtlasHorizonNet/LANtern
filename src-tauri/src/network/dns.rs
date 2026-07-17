use std::net::Ipv4Addr;
use std::time::Duration;

/// Reverse DNS with a short timeout. Returns None when lookup fails or times out.
pub fn reverse_lookup(ip: Ipv4Addr) -> Option<String> {
    let handle =
        std::thread::spawn(move || dns_lookup::lookup_addr(&std::net::IpAddr::V4(ip)).ok());

    match handle.join() {
        Ok(Some(name)) => {
            let trimmed = name.trim_end_matches('.').to_string();
            if trimmed.is_empty() || trimmed == ip.to_string() {
                None
            } else {
                Some(trimmed)
            }
        }
        _ => None,
    }
}

/// Best-effort timed reverse lookup used from async context via spawn_blocking.
pub fn reverse_lookup_timed(ip: Ipv4Addr, timeout: Duration) -> Option<String> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(reverse_lookup(ip));
    });
    rx.recv_timeout(timeout).ok().flatten()
}
