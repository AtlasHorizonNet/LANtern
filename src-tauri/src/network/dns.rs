//! DNS helpers used by the scanner (reverse enrichment) and the interactive
//! DNS diagnostics tool (forward/reverse queries against a chosen resolver).

use hickory_resolver::config::{NameServerConfig, ResolverConfig, ResolverOpts};
use hickory_resolver::name_server::TokioConnectionProvider;
use hickory_resolver::proto::rr::RecordType;
use hickory_resolver::proto::xfer::Protocol;
use hickory_resolver::Resolver;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::time::{Duration, Instant};

const QUERY_TIMEOUT: Duration = Duration::from_secs(3);

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DnsQueryResult {
    /// Resolver label shown in the UI (`system` or an IPv4/IPv6 address).
    pub server: String,
    pub query: String,
    pub record_type: String,
    pub success: bool,
    pub answers: Vec<String>,
    pub latency_ms: f64,
    pub error: Option<String>,
}

/// Forward lookup for `host` of the given record type against `server`.
///
/// Pass `server = None` (or `"system"`) to use the OS resolver configuration.
pub async fn dns_lookup(
    host: &str,
    record_type: &str,
    server: Option<&str>,
) -> Result<DnsQueryResult, String> {
    let host = host.trim();
    if host.is_empty() {
        return Err("Hostname is required".into());
    }

    let rtype = parse_record_type(record_type)?;
    let server_label = server_label(server);
    let resolver = build_resolver(server)?;

    let started = Instant::now();
    let outcome = resolver.lookup(host, rtype).await;
    let latency_ms = elapsed_ms(started);

    match outcome {
        Ok(lookup) => {
            let answers: Vec<String> = lookup
                .record_iter()
                .map(|record| format_rdata(record.data()))
                .collect();
            Ok(DnsQueryResult {
                server: server_label,
                query: host.to_string(),
                record_type: record_type_name(rtype).into(),
                success: !answers.is_empty(),
                answers,
                latency_ms,
                error: None,
            })
        }
        Err(err) => Ok(DnsQueryResult {
            server: server_label,
            query: host.to_string(),
            record_type: record_type_name(rtype).into(),
            success: false,
            answers: Vec::new(),
            latency_ms,
            error: Some(err.to_string()),
        }),
    }
}

/// Reverse (PTR) lookup for `ip` against `server`.
pub async fn dns_reverse(ip: &str, server: Option<&str>) -> Result<DnsQueryResult, String> {
    let ip = ip.trim();
    let addr: IpAddr = ip
        .parse()
        .map_err(|e| format!("Invalid IP address {ip}: {e}"))?;

    let server_label = server_label(server);
    let resolver = build_resolver(server)?;

    let started = Instant::now();
    let outcome = resolver.reverse_lookup(addr).await;
    let latency_ms = elapsed_ms(started);

    match outcome {
        Ok(lookup) => {
            let answers: Vec<String> = lookup
                .iter()
                .map(|name| name.to_string().trim_end_matches('.').to_string())
                .filter(|name| !name.is_empty())
                .collect();
            Ok(DnsQueryResult {
                server: server_label,
                query: ip.to_string(),
                record_type: "PTR".into(),
                success: !answers.is_empty(),
                answers,
                latency_ms,
                error: None,
            })
        }
        Err(err) => Ok(DnsQueryResult {
            server: server_label,
            query: ip.to_string(),
            record_type: "PTR".into(),
            success: false,
            answers: Vec::new(),
            latency_ms,
            error: Some(err.to_string()),
        }),
    }
}

fn build_resolver(server: Option<&str>) -> Result<Resolver<TokioConnectionProvider>, String> {
    let mut opts = ResolverOpts::default();
    opts.timeout = QUERY_TIMEOUT;
    opts.attempts = 1;
    opts.cache_size = 0;
    opts.validate = false;

    let provider = TokioConnectionProvider::default();

    match normalize_server(server) {
        None => {
            #[cfg(any(unix, target_os = "windows"))]
            {
                let mut builder = Resolver::builder(provider)
                    .map_err(|e| format!("Failed to read system DNS config: {e}"))?;
                *builder.options_mut() = opts;
                Ok(builder.build())
            }
            #[cfg(not(any(unix, target_os = "windows")))]
            {
                let _ = provider;
                let mut builder =
                    Resolver::builder_with_config(ResolverConfig::default(), provider);
                *builder.options_mut() = opts;
                Ok(builder.build())
            }
        }
        Some(ip) => {
            let socket: SocketAddr = format!("{ip}:53")
                .parse()
                .map_err(|e| format!("Invalid DNS server address {ip}: {e}"))?;
            let mut config = ResolverConfig::new();
            config.add_name_server(NameServerConfig::new(socket, Protocol::Udp));
            config.add_name_server(NameServerConfig::new(socket, Protocol::Tcp));
            let mut builder = Resolver::builder_with_config(config, provider);
            *builder.options_mut() = opts;
            Ok(builder.build())
        }
    }
}

fn normalize_server(server: Option<&str>) -> Option<String> {
    let trimmed = server?.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("system") {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn server_label(server: Option<&str>) -> String {
    normalize_server(server).unwrap_or_else(|| "system".into())
}

fn parse_record_type(value: &str) -> Result<RecordType, String> {
    match value.trim().to_ascii_uppercase().as_str() {
        "A" => Ok(RecordType::A),
        "AAAA" => Ok(RecordType::AAAA),
        "CNAME" => Ok(RecordType::CNAME),
        "TXT" => Ok(RecordType::TXT),
        "MX" => Ok(RecordType::MX),
        "NS" => Ok(RecordType::NS),
        "PTR" => Ok(RecordType::PTR),
        other => Err(format!(
            "Unsupported record type '{other}'. Use A, AAAA, CNAME, TXT, MX, NS, or PTR."
        )),
    }
}

fn record_type_name(rtype: RecordType) -> &'static str {
    match rtype {
        RecordType::A => "A",
        RecordType::AAAA => "AAAA",
        RecordType::CNAME => "CNAME",
        RecordType::TXT => "TXT",
        RecordType::MX => "MX",
        RecordType::NS => "NS",
        RecordType::PTR => "PTR",
        _ => "UNKNOWN",
    }
}

fn format_rdata(data: &hickory_resolver::proto::rr::RData) -> String {
    data.to_string()
}

fn elapsed_ms(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_supported_record_types() {
        assert_eq!(parse_record_type("a").unwrap(), RecordType::A);
        assert_eq!(parse_record_type("AAAA").unwrap(), RecordType::AAAA);
        assert_eq!(parse_record_type("Mx").unwrap(), RecordType::MX);
        assert!(parse_record_type("SOA").is_err());
    }

    #[test]
    fn normalizes_system_server() {
        assert_eq!(normalize_server(None), None);
        assert_eq!(normalize_server(Some("")), None);
        assert_eq!(normalize_server(Some("system")), None);
        assert_eq!(normalize_server(Some("1.1.1.1")), Some("1.1.1.1".into()));
    }
}
