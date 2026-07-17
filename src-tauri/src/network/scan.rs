use crate::network::dns;
use crate::network::interfaces::{self, hosts_to_scan};
use crate::network::neighbors;
use crate::network::oui;
use crate::network::{Device, NetworkInfo, ScanProgress, ScanResult};
use std::collections::{HashMap, HashSet};
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter};
use tokio::net::TcpStream;
use tokio::sync::Semaphore;
use tokio::time::timeout;

const PROBE_PORTS: &[u16] = &[80, 443, 22, 445, 139, 8080, 8443, 53];
const CONNECT_TIMEOUT: Duration = Duration::from_millis(350);
const UDP_SEED_PORT: u16 = 9;
const MAX_IN_FLIGHT: usize = 128;

pub async fn run_scan(
    app: AppHandle,
    cancel: Arc<AtomicBool>,
    nicknames: HashMap<String, String>,
    previous: HashMap<String, Device>,
) -> Result<ScanResult, String> {
    cancel.store(false, Ordering::SeqCst);
    oui::warm_cache();

    let network = interfaces::detect_network()?;
    let hosts = hosts_to_scan(&network)?;
    let total = hosts.len() as u32;
    let local_ip: Ipv4Addr = network
        .local_ip
        .parse()
        .map_err(|e| format!("bad local ip: {e}"))?;
    let gateway_ip = network
        .gateway
        .as_ref()
        .and_then(|g| g.parse::<Ipv4Addr>().ok());

    emit_progress(
        &app,
        ScanProgress {
            checked: 0,
            total,
            found: 0,
            phase: "probing".into(),
        },
    );

    // Phase 1: ARP-seed via UDP + TCP connect probes
    let live = probe_hosts(&app, &hosts, total, cancel.clone()).await?;

    if cancel.load(Ordering::SeqCst) {
        return Ok(ScanResult {
            network,
            devices: Vec::new(),
            cancelled: true,
        });
    }

    // Always include ourselves
    let mut live_set: HashSet<Ipv4Addr> = live.into_iter().collect();
    live_set.insert(local_ip);
    if let Some(gw) = gateway_ip {
        // Gateway may not answer TCP; still try to include if neighbor table knows it later
        let _ = gw;
    }

    emit_progress(
        &app,
        ScanProgress {
            checked: total,
            total,
            found: live_set.len() as u32,
            phase: "neighbors".into(),
        },
    );

    // Brief settle so OS ARP table fills from UDP seeds
    tokio::time::sleep(Duration::from_millis(400)).await;
    let neighbor_map = neighbors::read_neighbors();

    // Include any ARP-known hosts on this subnet even if TCP failed (phones, IoT, etc.)
    let cidr: ipnetwork::Ipv4Network = network
        .cidr
        .parse()
        .map_err(|e| format!("bad cidr: {e}"))?;
    for (ip_str, _) in &neighbor_map {
        if let Ok(ip) = ip_str.parse::<Ipv4Addr>() {
            if cidr.contains(ip) {
                live_set.insert(ip);
            }
        }
    }
    if let Some(gw) = gateway_ip {
        if neighbor_map.contains_key(&gw.to_string()) || live_set.contains(&gw) {
            live_set.insert(gw);
        } else {
            // Still list gateway as a candidate — many routers respond to ARP after seed
            live_set.insert(gw);
        }
    }

    emit_progress(
        &app,
        ScanProgress {
            checked: total,
            total,
            found: live_set.len() as u32,
            phase: "enriching".into(),
        },
    );

    let now = chrono::Utc::now().timestamp();
    let mut devices = Vec::new();

    let mut sorted: Vec<Ipv4Addr> = live_set.into_iter().collect();
    sorted.sort();

    for ip in sorted {
        if cancel.load(Ordering::SeqCst) {
            break;
        }

        let ip_str = ip.to_string();
        let mac = neighbor_map.get(&ip_str).cloned().or_else(|| {
            // Local machine MAC from interface is harder; leave blank if unknown
            None
        });
        let vendor = mac.as_ref().and_then(|m| oui::lookup_vendor(m));
        let hostname = tokio::task::spawn_blocking(move || {
            dns::reverse_lookup_timed(ip, Duration::from_millis(500))
        })
        .await
        .ok()
        .flatten();

        let key = mac.clone().unwrap_or_else(|| ip_str.clone());
        let nickname = nicknames
            .get(&key)
            .cloned()
            .or_else(|| nicknames.get(&ip_str).cloned());

        let device = Device {
            ip: ip_str.clone(),
            mac,
            hostname,
            vendor,
            nickname,
            online: true,
            last_seen: now,
            is_gateway: gateway_ip == Some(ip),
            is_local: ip == local_ip,
        };

        let _ = app.emit("device-found", &device);
        devices.push(device);
    }

    // Merge previously known devices that went offline
    let online_ips: HashSet<String> = devices.iter().map(|d| d.ip.clone()).collect();
    for (ip, prev) in previous {
        if !online_ips.contains(&ip) {
            let mut offline = prev;
            offline.online = false;
            // refresh nickname if updated
            let key = offline
                .mac
                .clone()
                .unwrap_or_else(|| offline.ip.clone());
            if let Some(n) = nicknames.get(&key).or_else(|| nicknames.get(&offline.ip)) {
                offline.nickname = Some(n.clone());
            }
            devices.push(offline);
        }
    }

    devices.sort_by(|a, b| {
        // local first, gateway second, online before offline, then IP
        b.is_local
            .cmp(&a.is_local)
            .then(b.is_gateway.cmp(&a.is_gateway))
            .then(b.online.cmp(&a.online))
            .then_with(|| {
                let ai: Ipv4Addr = a.ip.parse().unwrap_or(Ipv4Addr::UNSPECIFIED);
                let bi: Ipv4Addr = b.ip.parse().unwrap_or(Ipv4Addr::UNSPECIFIED);
                ai.cmp(&bi)
            })
    });

    let cancelled = cancel.load(Ordering::SeqCst);
    emit_progress(
        &app,
        ScanProgress {
            checked: total,
            total,
            found: devices.iter().filter(|d| d.online).count() as u32,
            phase: if cancelled { "cancelled" } else { "done" }.into(),
        },
    );

    Ok(ScanResult {
        network,
        devices,
        cancelled,
    })
}

async fn probe_hosts(
    app: &AppHandle,
    hosts: &[Ipv4Addr],
    total: u32,
    cancel: Arc<AtomicBool>,
) -> Result<Vec<Ipv4Addr>, String> {
    let sem = Arc::new(Semaphore::new(MAX_IN_FLIGHT));
    let checked = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let mut handles = Vec::with_capacity(hosts.len());

    for &ip in hosts {
        if cancel.load(Ordering::SeqCst) {
            break;
        }
        let sem = sem.clone();
        let cancel = cancel.clone();
        let checked = checked.clone();
        let app = app.clone();
        handles.push(tokio::spawn(async move {
            let _permit = match sem.acquire().await {
                Ok(p) => p,
                Err(_) => return None,
            };
            if cancel.load(Ordering::SeqCst) {
                return None;
            }

            // UDP seed forces ARP resolution on most OSes
            seed_arp(ip).await;
            let alive = tcp_probe(ip).await;

            let n = checked.fetch_add(1, Ordering::Relaxed) + 1;
            if n % 8 == 0 || n == total {
                emit_progress(
                    &app,
                    ScanProgress {
                        checked: n,
                        total,
                        found: 0,
                        phase: "probing".into(),
                    },
                );
            }

            if alive {
                Some(ip)
            } else {
                None
            }
        }));
    }

    let mut live = Vec::new();
    for h in handles {
        if let Ok(Some(ip)) = h.await {
            live.push(ip);
        }
    }
    Ok(live)
}

async fn seed_arp(ip: Ipv4Addr) {
    use tokio::net::UdpSocket;
    if let Ok(sock) = UdpSocket::bind("0.0.0.0:0").await {
        let _ = sock.send_to(&[0u8], (ip, UDP_SEED_PORT)).await;
    }
}

async fn tcp_probe(ip: Ipv4Addr) -> bool {
    for &port in PROBE_PORTS {
        let addr = SocketAddr::from((ip, port));
        match timeout(CONNECT_TIMEOUT, TcpStream::connect(addr)).await {
            Ok(Ok(_)) => return true,
            // Connection refused still means host is alive
            Ok(Err(e)) => {
                let kind = e.kind();
                if kind == std::io::ErrorKind::ConnectionRefused {
                    return true;
                }
            }
            Err(_) => {}
        }
    }
    false
}

fn emit_progress(app: &AppHandle, progress: ScanProgress) {
    let _ = app.emit("scan-progress", progress);
}

pub fn network_info() -> Result<NetworkInfo, String> {
    interfaces::detect_network()
}
