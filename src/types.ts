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

export type PingOutcome = {
  method: "icmp" | "tcp";
  success: boolean;
  latencyMs: number | null;
  error: string | null;
};

export type DnsRecordType = "A" | "AAAA" | "CNAME" | "TXT" | "MX" | "NS";

export type DnsQueryResult = {
  server: string;
  query: string;
  recordType: string;
  success: boolean;
  answers: string[];
  latencyMs: number;
  error: string | null;
};

export type AppPage = "devices" | "dns" | "history" | "settings";

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
