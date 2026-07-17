import { invoke } from "@tauri-apps/api/core";
import type { Device, NetworkInfo, PingOutcome, ScanResult } from "./types";

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

export function getDevices(): Promise<Device[]> {
  return invoke("get_devices");
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
