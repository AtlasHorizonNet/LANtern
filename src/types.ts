export type NetworkInfo = {
  interfaceName: string;
  localIp: string;
  cidr: string;
  prefix: number;
  gateway: string | null;
  hostCount: number;
  fingerprint: string;
  displayName: string | null;
  autoName: string;
  media: string;
  ssid: string | null;
  searchDomain: string | null;
  externalIp: string | null;
  dbId: number | null;
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

export type ScanRunSummary = {
  id: number;
  networkId: number;
  networkName: string;
  startedAt: number;
  finishedAt: number;
  interfaceName: string;
  localIp: string;
  cidr: string;
  gateway: string | null;
  externalIp: string | null;
  deviceCount: number;
  onlineCount: number;
  cancelled: boolean;
};

export type ScanRunDetail = {
  run: ScanRunSummary;
  devices: Device[];
};

export type DhcpOffer = {
  serverIp: string | null;
  offeredIp: string;
  leaseSeconds: number | null;
  subnetMask: string | null;
  gateway: string | null;
  dnsServers: string[];
  domain: string | null;
  latencyMs: number;
  messageType: string;
};

export type DhcpDiscoverResult = {
  success: boolean;
  privilegeNote: string;
  timeoutMs: number;
  offers: DhcpOffer[];
  error: string | null;
};

export type PortState = "open" | "closed" | "filtered";

export type PortResult = {
  port: number;
  state: PortState;
  service: string | null;
  latencyMs: number | null;
};

export type PortScanResult = {
  ip: string;
  scanned: number;
  openCount: number;
  durationMs: number;
  ports: PortResult[];
};

export type WakeResult = {
  success: boolean;
  mac: string;
  broadcast: string;
  message: string;
};

export type AppPage =
  | "scan"
  | "devices"
  | "dns"
  | "dhcp"
  | "history"
  | "settings";

export function deviceKey(device: Device): string {
  return device.mac ?? device.ip;
}

/** Automatic label when no custom name is set. */
export function automaticName(device: Device): string {
  return (
    device.hostname ||
    device.vendor ||
    (device.isLocal ? "This computer" : device.isGateway ? "Gateway" : device.ip)
  );
}

export function displayName(device: Device): string {
  return device.nickname || automaticName(device);
}

export function networkLabel(network: NetworkInfo): string {
  return network.displayName || network.autoName || network.interfaceName;
}
