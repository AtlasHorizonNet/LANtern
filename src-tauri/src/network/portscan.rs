//! Concurrent TCP connect scan against a single host.

use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::sync::Semaphore;
use tokio::time::timeout;

const CONNECT_TIMEOUT: Duration = Duration::from_millis(400);
const MAX_IN_FLIGHT: usize = 64;
const MAX_PORTS: usize = 1024;

/// Sensible default “top services” port set for device diagnostics.
pub const DEFAULT_PORTS: &[u16] = &[
    21, 22, 23, 25, 53, 80, 110, 111, 135, 139, 143, 443, 445, 993, 995, 1433, 1521, 3306, 3389,
    5432, 5900, 6379, 8080, 8443, 9100,
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PortState {
    Open,
    Closed,
    Filtered,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortResult {
    pub port: u16,
    pub state: PortState,
    pub service: Option<String>,
    pub latency_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PortScanResult {
    pub ip: String,
    pub scanned: u32,
    pub open_count: u32,
    pub duration_ms: f64,
    pub ports: Vec<PortResult>,
}

/// Parse a ports expression: `80`, `22,80,443`, `8000-8010`, or mixed.
pub fn parse_ports_spec(spec: &str) -> Result<Vec<u16>, String> {
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return Ok(DEFAULT_PORTS.to_vec());
    }

    let mut ports = Vec::new();
    for part in trimmed.split([',', ' ', ';']) {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some((start, end)) = part.split_once('-') {
            let start: u16 = start
                .trim()
                .parse()
                .map_err(|_| format!("Invalid port range start in '{part}'"))?;
            let end: u16 = end
                .trim()
                .parse()
                .map_err(|_| format!("Invalid port range end in '{part}'"))?;
            if start == 0 || end == 0 {
                return Err("Port 0 is not allowed".into());
            }
            if end < start {
                return Err(format!("Invalid port range '{part}' (end < start)"));
            }
            if (end as u32) - (start as u32) + 1 > 512 {
                return Err(format!(
                    "Port range '{part}' is too large (max 512 ports per range)"
                ));
            }
            for p in start..=end {
                ports.push(p);
            }
        } else {
            let p: u16 = part.parse().map_err(|_| format!("Invalid port '{part}'"))?;
            if p == 0 {
                return Err("Port 0 is not allowed".into());
            }
            ports.push(p);
        }
    }

    ports.sort_unstable();
    ports.dedup();
    if ports.is_empty() {
        return Err("No ports to scan".into());
    }
    if ports.len() > MAX_PORTS {
        return Err(format!(
            "Too many ports ({}). Maximum is {MAX_PORTS}.",
            ports.len()
        ));
    }
    Ok(ports)
}

pub async fn scan_ports(ip: Ipv4Addr, ports: Vec<u16>) -> PortScanResult {
    let started = Instant::now();
    let sem = Arc::new(Semaphore::new(MAX_IN_FLIGHT));
    let mut handles = Vec::with_capacity(ports.len());

    for port in ports {
        let sem = sem.clone();
        handles.push(tokio::spawn(async move {
            let _permit = sem.acquire().await.ok();
            probe_port(ip, port).await
        }));
    }

    let mut results = Vec::with_capacity(handles.len());
    for handle in handles {
        if let Ok(result) = handle.await {
            results.push(result);
        }
    }
    results.sort_by_key(|r| r.port);

    let open_count = results
        .iter()
        .filter(|r| r.state == PortState::Open)
        .count() as u32;

    PortScanResult {
        ip: ip.to_string(),
        scanned: results.len() as u32,
        open_count,
        duration_ms: started.elapsed().as_secs_f64() * 1000.0,
        ports: results,
    }
}

async fn probe_port(ip: Ipv4Addr, port: u16) -> PortResult {
    let addr = SocketAddr::from((ip, port));
    let started = Instant::now();
    match timeout(CONNECT_TIMEOUT, TcpStream::connect(addr)).await {
        Ok(Ok(_stream)) => PortResult {
            port,
            state: PortState::Open,
            service: well_known_service(port).map(str::to_string),
            latency_ms: Some(started.elapsed().as_secs_f64() * 1000.0),
        },
        Ok(Err(err)) => {
            let state = match err.kind() {
                std::io::ErrorKind::ConnectionRefused => PortState::Closed,
                _ => PortState::Filtered,
            };
            PortResult {
                port,
                state,
                service: well_known_service(port).map(str::to_string),
                latency_ms: None,
            }
        }
        Err(_) => PortResult {
            port,
            state: PortState::Filtered,
            service: well_known_service(port).map(str::to_string),
            latency_ms: None,
        },
    }
}

fn well_known_service(port: u16) -> Option<&'static str> {
    Some(match port {
        21 => "ftp",
        22 => "ssh",
        23 => "telnet",
        25 => "smtp",
        53 => "dns",
        80 => "http",
        110 => "pop3",
        111 => "rpcbind",
        135 => "msrpc",
        139 => "netbios-ssn",
        143 => "imap",
        443 => "https",
        445 => "smb",
        993 => "imaps",
        995 => "pop3s",
        1433 => "mssql",
        1521 => "oracle",
        3306 => "mysql",
        3389 => "rdp",
        5432 => "postgres",
        5900 => "vnc",
        6379 => "redis",
        8080 => "http-alt",
        8443 => "https-alt",
        9100 => "jetdirect",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_lists_and_ranges() {
        assert_eq!(parse_ports_spec("80").unwrap(), vec![80]);
        assert_eq!(parse_ports_spec("22,80,443").unwrap(), vec![22, 80, 443]);
        assert_eq!(
            parse_ports_spec("8000-8002").unwrap(),
            vec![8000, 8001, 8002]
        );
        assert_eq!(
            parse_ports_spec("22, 8000-8001, 443").unwrap(),
            vec![22, 443, 8000, 8001]
        );
    }

    #[test]
    fn empty_spec_uses_defaults() {
        let ports = parse_ports_spec("").unwrap();
        assert!(ports.contains(&22));
        assert!(ports.contains(&80));
        assert!(ports.contains(&443));
    }

    #[test]
    fn rejects_invalid_ranges() {
        assert!(parse_ports_spec("10-1").is_err());
        assert!(parse_ports_spec("abc").is_err());
        assert!(parse_ports_spec("0").is_err());
    }

    #[tokio::test]
    async fn loopback_open_and_closed() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let _ = listener.accept().await;
            tokio::time::sleep(Duration::from_millis(200)).await;
        });

        let result = scan_ports(Ipv4Addr::LOCALHOST, vec![port, 1]).await;
        assert_eq!(result.scanned, 2);
        let open = result.ports.iter().find(|p| p.port == port).unwrap();
        assert_eq!(open.state, PortState::Open);
        let closedish = result.ports.iter().find(|p| p.port == 1).unwrap();
        assert!(matches!(
            closedish.state,
            PortState::Closed | PortState::Filtered
        ));
    }
}
