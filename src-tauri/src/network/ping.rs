//! Single-shot device ping: system `ping` binary first (unprivileged on
//! Linux, macOS, and Windows), timed TCP connect as a fallback.
//!
//! The frontend calls `ping_device` repeatedly to build a session history,
//! so each invocation sends exactly one probe.

use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Ports tried by the TCP fallback, mirroring the scanner's probe set.
const FALLBACK_PORTS: &[u16] = &[80, 443, 22, 445, 139, 8080, 8443, 53];
const PING_TIMEOUT: Duration = Duration::from_secs(2);
const TCP_CONNECT_TIMEOUT: Duration = Duration::from_millis(750);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PingOutcome {
    /// "icmp" when the system ping binary produced the answer, "tcp" for the
    /// connect-timing fallback.
    pub method: String,
    pub success: bool,
    /// Round-trip time when known. An ICMP reply that could not be parsed for
    /// a time still counts as success with an unknown latency.
    pub latency_ms: Option<f64>,
    pub error: Option<String>,
}

pub async fn ping_once(ip: Ipv4Addr) -> PingOutcome {
    match icmp_ping(ip).await {
        Ok(outcome) => outcome,
        // The ping binary being unavailable or failing to run is not the same
        // as the host being down: measure a TCP connect instead.
        Err(_) => tcp_ping(ip).await,
    }
}

/// Run the platform ping binary for a single echo request.
///
/// `Err` means the binary could not be executed at all; `Ok` carries the
/// probe result (which may still be an unsuccessful probe).
async fn icmp_ping(ip: Ipv4Addr) -> Result<PingOutcome, String> {
    let output = ping_command(ip)
        .output()
        .await
        .map_err(|e| format!("failed to run ping: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    // "TTL=" only appears in genuine echo replies, which matters on Windows
    // where "Destination host unreachable" still exits with code 0.
    let replied = output_indicates_reply(&stdout);
    let success = replied && (cfg!(windows) || output.status.success());

    Ok(PingOutcome {
        method: "icmp".into(),
        success,
        latency_ms: if success {
            parse_ping_time_ms(&stdout)
        } else {
            None
        },
        error: if success {
            None
        } else {
            Some("no ICMP reply".into())
        },
    })
}

fn ping_command(ip: Ipv4Addr) -> tokio::process::Command {
    let mut cmd = tokio::process::Command::new("ping");

    // One echo request with a per-platform reply timeout:
    // Linux -W takes seconds, macOS -W and Windows -w take milliseconds.
    #[cfg(target_os = "linux")]
    cmd.args(["-n", "-c", "1", "-W"])
        .arg(PING_TIMEOUT.as_secs().max(1).to_string());
    #[cfg(target_os = "macos")]
    cmd.args(["-n", "-c", "1", "-W"])
        .arg(PING_TIMEOUT.as_millis().to_string());
    #[cfg(windows)]
    cmd.args(["-n", "1", "-w"])
        .arg(PING_TIMEOUT.as_millis().to_string());
    #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
    cmd.args(["-c", "1"]);

    cmd.arg(ip.to_string());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

/// Time a TCP connect to common ports. A completed connection — or an
/// immediate refusal — proves the host is up and gives a latency figure.
async fn tcp_ping(ip: Ipv4Addr) -> PingOutcome {
    for &port in FALLBACK_PORTS {
        let addr = SocketAddr::from((ip, port));
        let started = Instant::now();
        match timeout(TCP_CONNECT_TIMEOUT, TcpStream::connect(addr)).await {
            Ok(Ok(_)) => {
                return PingOutcome {
                    method: "tcp".into(),
                    success: true,
                    latency_ms: Some(elapsed_ms(started)),
                    error: None,
                };
            }
            Ok(Err(e)) if e.kind() == std::io::ErrorKind::ConnectionRefused => {
                return PingOutcome {
                    method: "tcp".into(),
                    success: true,
                    latency_ms: Some(elapsed_ms(started)),
                    error: None,
                };
            }
            _ => {}
        }
    }
    PingOutcome {
        method: "tcp".into(),
        success: false,
        latency_ms: None,
        error: Some("no response on common TCP ports".into()),
    }
}

fn elapsed_ms(started: Instant) -> f64 {
    (started.elapsed().as_secs_f64() * 1000.0 * 100.0).round() / 100.0
}

/// True when the output contains an actual echo reply ("ttl=" appears in
/// reply lines on every platform and locale, but never in gateway
/// "unreachable" chatter).
pub fn output_indicates_reply(output: &str) -> bool {
    output.to_ascii_lowercase().contains("ttl=")
}

/// Extract the reported round-trip time in milliseconds.
///
/// Handles `time=0.045 ms` (Linux/macOS), `time=3ms` / `time<1ms` (Windows),
/// and localized variants such as `Zeit=3ms` by looking for any `=`/`<`
/// separated number directly before an `ms` unit.
pub fn parse_ping_time_ms(output: &str) -> Option<f64> {
    let lower = output.to_ascii_lowercase();
    let bytes = lower.as_bytes();

    for (idx, _) in lower.match_indices("ms") {
        // Walk backwards over an optional space, then digits and dots.
        let mut end = idx;
        if end > 0 && bytes[end - 1] == b' ' {
            end -= 1;
        }
        let mut start = end;
        while start > 0 && (bytes[start - 1].is_ascii_digit() || bytes[start - 1] == b'.') {
            start -= 1;
        }
        if start == end {
            continue;
        }
        // Require an explicit =/< separator so we don't misread words that
        // merely end in "ms" or unrelated numbers.
        if start == 0 || (bytes[start - 1] != b'=' && bytes[start - 1] != b'<') {
            continue;
        }
        if let Ok(value) = lower[start..end].parse::<f64>() {
            return Some(value);
        }
    }
    None
}
