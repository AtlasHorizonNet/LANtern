import { invoke } from "@tauri-apps/api/core";
import type {
  Device,
  DnsQueryResult,
  NetworkInfo,
  PingOutcome,
  ScanResult,
  ScanRunDetail,
  ScanRunSummary,
} from "./types";

export function getNetworkInfo(): Promise<NetworkInfo> {
  return invoke("get_network_info");
}

export function listNetworks(): Promise<NetworkInfo[]> {
  return invoke("list_networks");
}

export function startScan(network?: NetworkInfo | null): Promise<ScanResult> {
  return invoke("start_scan", { network: network ?? null });
}

export function cancelScan(): Promise<void> {
  return invoke("cancel_scan");
}

export function getDevices(fingerprint: string): Promise<Device[]> {
  return invoke("get_devices", { fingerprint });
}

export function clearDevices(fingerprint: string): Promise<void> {
  return invoke("clear_devices", { fingerprint });
}

export function pingDevice(ip: string): Promise<PingOutcome> {
  return invoke("ping_device", { ip });
}

export function setDeviceNickname(
  key: string,
  nickname: string | null,
): Promise<void> {
  return invoke("set_device_nickname", { key, nickname });
}

export function setNetworkDisplayName(
  fingerprint: string,
  displayName: string | null,
): Promise<NetworkInfo> {
  return invoke("set_network_display_name", { fingerprint, displayName });
}

export function refreshExternalIp(
  fingerprint: string,
): Promise<string | null> {
  return invoke("refresh_external_ip", { fingerprint });
}

export function listScanRuns(
  limit = 50,
  offset = 0,
): Promise<ScanRunSummary[]> {
  return invoke("list_scan_runs", { limit, offset });
}

export function getScanRun(id: number): Promise<ScanRunDetail | null> {
  return invoke("get_scan_run", { id });
}

export function dnsLookup(
  host: string,
  recordType: string,
  server?: string | null,
): Promise<DnsQueryResult> {
  return invoke("dns_lookup", {
    host,
    recordType,
    server: server ?? null,
  });
}

export function dnsReverse(
  ip: string,
  server?: string | null,
): Promise<DnsQueryResult> {
  return invoke("dns_reverse", { ip, server: server ?? null });
}
