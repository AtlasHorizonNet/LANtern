import { invoke } from "@tauri-apps/api/core";
import type { Device, NetworkInfo, ScanResult } from "./types";

export function getNetworkInfo(): Promise<NetworkInfo> {
  return invoke("get_network_info");
}

export function startScan(): Promise<ScanResult> {
  return invoke("start_scan");
}

export function cancelScan(): Promise<void> {
  return invoke("cancel_scan");
}

export function getDevices(): Promise<Device[]> {
  return invoke("get_devices");
}

export function setDeviceNickname(
  key: string,
  nickname: string | null,
): Promise<void> {
  return invoke("set_device_nickname", { key, nickname });
}
