export type NetworkInfo = {
  interfaceName: string;
  localIp: string;
  cidr: string;
  prefix: number;
  gateway: string | null;
  hostCount: number;
};

export type Device = {
  ip: string;
  mac: string | null;
  hostname: string | null;
  vendor: string | null;
  nickname: string | null;
  online: boolean;
  lastSeen: number;
  isGateway: boolean;
  isLocal: boolean;
};

export type ScanProgress = {
  checked: number;
  total: number;
  found: number;
  phase: string;
};

export type ScanResult = {
  network: NetworkInfo;
  devices: Device[];
  cancelled: boolean;
};

export function deviceKey(device: Device): string {
  return device.mac ?? device.ip;
}

export function displayName(device: Device): string {
  return (
    device.nickname ||
    device.hostname ||
    device.vendor ||
    (device.isLocal ? "This computer" : device.isGateway ? "Gateway" : device.ip)
  );
}
