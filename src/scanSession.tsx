import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  useTransition,
  type Dispatch,
  type ReactNode,
  type SetStateAction,
} from "react";
import { listen } from "@tauri-apps/api/event";
import {
  cancelScan,
  clearDevices,
  getDevices,
  listNetworks,
  refreshExternalIp,
  setNetworkDisplayName,
  startScan,
} from "./api";
import {
  networkLabel,
  type Device,
  type NetworkInfo,
  type ScanProgress,
} from "./types";

export function networkOptionId(n: NetworkInfo): string {
  return n.fingerprint || `${n.interfaceName}|${n.localIp}|${n.cidr}`;
}

export function sortDevices(a: Device, b: Device): number {
  if (a.isLocal !== b.isLocal) return a.isLocal ? -1 : 1;
  if (a.isGateway !== b.isGateway) return a.isGateway ? -1 : 1;
  if (a.online !== b.online) return a.online ? -1 : 1;
  return a.ip.localeCompare(b.ip, undefined, { numeric: true });
}

type ScanSession = {
  networks: NetworkInfo[];
  network: NetworkInfo | null;
  devices: Device[];
  setDevices: Dispatch<SetStateAction<Device[]>>;
  progress: ScanProgress | null;
  scanning: boolean;
  error: string | null;
  setError: (error: string | null) => void;
  networkNameDraft: string;
  setNetworkNameDraft: (value: string) => void;
  onlineCount: number;
  progressPct: number;
  selectNetwork: (id: string) => Promise<void>;
  runScan: () => Promise<"ok" | "cancelled" | "error">;
  cancel: () => Promise<void>;
  clear: () => Promise<void>;
  saveNetworkName: () => Promise<void>;
};

const ScanSessionContext = createContext<ScanSession | null>(null);

export function ScanSessionProvider({ children }: { children: ReactNode }) {
  const [networks, setNetworks] = useState<NetworkInfo[]>([]);
  const [network, setNetwork] = useState<NetworkInfo | null>(null);
  const [devices, setDevices] = useState<Device[]>([]);
  const [progress, setProgress] = useState<ScanProgress | null>(null);
  const [scanning, setScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [networkNameDraft, setNetworkNameDraft] = useState("");
  const [, startTransition] = useTransition();
  const activeFingerprint = useRef<string | null>(null);

  const loadDevicesFor = useCallback(async (next: NetworkInfo) => {
    activeFingerprint.current = next.fingerprint;
    setNetworkNameDraft(networkLabel(next));
    try {
      const cached = await getDevices(next.fingerprint);
      setDevices([...cached].sort(sortDevices));
    } catch {
      setDevices([]);
    }
  }, []);

  useEffect(() => {
    let unprogress: (() => void) | undefined;
    let undevice: (() => void) | undefined;

    (async () => {
      try {
        const found = await listNetworks();
        setNetworks(found);
        if (found.length) {
          setNetwork(found[0]);
          await loadDevicesFor(found[0]);
          refreshExternalIp(found[0].fingerprint)
            .then((ip) => {
              if (!ip) return;
              setNetwork((n) =>
                n && n.fingerprint === found[0].fingerprint
                  ? { ...n, externalIp: ip }
                  : n,
              );
              setNetworks((list) =>
                list.map((n) =>
                  n.fingerprint === found[0].fingerprint
                    ? { ...n, externalIp: ip }
                    : n,
                ),
              );
            })
            .catch(() => {
              /* offline / blocked */
            });
        } else {
          setError("No suitable IPv4 network interface found");
        }
      } catch (e) {
        setError(String(e));
      }

      unprogress = await listen<ScanProgress>("scan-progress", (event) => {
        setProgress(event.payload);
      });

      undevice = await listen<Device>("device-found", (event) => {
        startTransition(() => {
          setDevices((prev) => {
            const key = event.payload.ip;
            const next = prev.filter((d) => d.ip !== key);
            next.push(event.payload);
            next.sort(sortDevices);
            return next;
          });
        });
      });
    })();

    return () => {
      unprogress?.();
      undevice?.();
    };
  }, [loadDevicesFor]);

  const selectNetwork = useCallback(
    async (id: string) => {
      const next = networks.find((n) => networkOptionId(n) === id);
      if (!next) return;
      if (next.fingerprint === network?.fingerprint) {
        setNetwork(next);
        return;
      }
      setNetwork(next);
      await loadDevicesFor(next);
    },
    [loadDevicesFor, network?.fingerprint, networks],
  );

  const runScan = useCallback(async () => {
    if (!network) return "error";
    setError(null);
    setScanning(true);
    setProgress({
      checked: 0,
      total: network.hostCount ?? 0,
      found: 0,
      phase: "starting",
    });
    setDevices([]);
    try {
      const result = await startScan(network);
      setNetwork(result.network);
      setNetworks((list) => {
        const others = list.filter(
          (n) => n.fingerprint !== result.network.fingerprint,
        );
        return [result.network, ...others];
      });
      setNetworkNameDraft(networkLabel(result.network));
      activeFingerprint.current = result.network.fingerprint;
      setDevices([...result.devices].sort(sortDevices));
      if (result.cancelled) {
        setError("Scan cancelled");
        return "cancelled";
      }
      return "ok";
    } catch (e) {
      setError(String(e));
      return "error";
    } finally {
      setScanning(false);
      setProgress(null);
    }
  }, [network]);

  const cancel = useCallback(async () => {
    await cancelScan();
  }, []);

  const clear = useCallback(async () => {
    if (!network) return;
    setError(null);
    try {
      await clearDevices(network.fingerprint);
      setDevices([]);
    } catch (e) {
      setError(String(e));
    }
  }, [network]);

  const saveNetworkName = useCallback(async () => {
    if (!network) return;
    try {
      const updated = await setNetworkDisplayName(
        network.fingerprint,
        networkNameDraft.trim() || null,
      );
      setNetwork(updated);
      setNetworks((list) =>
        list.map((n) =>
          n.fingerprint === updated.fingerprint ? updated : n,
        ),
      );
      setNetworkNameDraft(networkLabel(updated));
    } catch (e) {
      setError(String(e));
    }
  }, [network, networkNameDraft]);

  const onlineCount = devices.filter((d) => d.online).length;
  const progressPct =
    progress && progress.total > 0
      ? Math.min(100, Math.round((progress.checked / progress.total) * 100))
      : scanning
        ? 5
        : 0;

  const value = useMemo<ScanSession>(
    () => ({
      networks,
      network,
      devices,
      setDevices,
      progress,
      scanning,
      error,
      setError,
      networkNameDraft,
      setNetworkNameDraft,
      onlineCount,
      progressPct,
      selectNetwork,
      runScan,
      cancel,
      clear,
      saveNetworkName,
    }),
    [
      networks,
      network,
      devices,
      progress,
      scanning,
      error,
      networkNameDraft,
      onlineCount,
      progressPct,
      selectNetwork,
      runScan,
      cancel,
      clear,
      saveNetworkName,
    ],
  );

  return (
    <ScanSessionContext.Provider value={value}>
      {children}
    </ScanSessionContext.Provider>
  );
}

export function useScanSession(): ScanSession {
  const ctx = useContext(ScanSessionContext);
  if (!ctx) {
    throw new Error("useScanSession must be used within ScanSessionProvider");
  }
  return ctx;
}
