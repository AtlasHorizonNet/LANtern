import { useEffect, useRef, useState, useTransition } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  cancelScan,
  clearDevices,
  getDevices,
  listNetworks,
  pingDevice,
  refreshExternalIp,
  setDeviceNickname,
  setNetworkDisplayName,
  startScan,
} from "../api";
import {
  deviceKey,
  displayName,
  networkLabel,
  type Device,
  type NetworkInfo,
  type PingOutcome,
  type ScanProgress,
} from "../types";

function networkOptionId(n: NetworkInfo): string {
  return n.fingerprint || `${n.interfaceName}|${n.localIp}|${n.cidr}`;
}

export function DevicesPage() {
  const [networks, setNetworks] = useState<NetworkInfo[]>([]);
  const [network, setNetwork] = useState<NetworkInfo | null>(null);
  const [devices, setDevices] = useState<Device[]>([]);
  const [selected, setSelected] = useState<Device | null>(null);
  const [progress, setProgress] = useState<ScanProgress | null>(null);
  const [scanning, setScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [nicknameDraft, setNicknameDraft] = useState("");
  const [networkNameDraft, setNetworkNameDraft] = useState("");
  const [, startTransition] = useTransition();
  const activeFingerprint = useRef<string | null>(null);

  useEffect(() => {
    let unprogress: (() => void) | undefined;
    let undevice: (() => void) | undefined;

    (async () => {
      try {
        const found = await listNetworks();
        setNetworks(found);
        if (found.length) {
          setNetwork(found[0]);
          activeFingerprint.current = found[0].fingerprint;
          setNetworkNameDraft(networkLabel(found[0]));
          const cached = await getDevices(found[0].fingerprint);
          setDevices([...cached].sort(sortDevices));
          // Best-effort WAN IP refresh for the active network.
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
  }, []);

  useEffect(() => {
    setNicknameDraft(selected?.nickname ?? "");
  }, [selected?.ip, selected?.nickname]);

  async function loadDevicesFor(next: NetworkInfo) {
    activeFingerprint.current = next.fingerprint;
    setNetworkNameDraft(networkLabel(next));
    setSelected(null);
    try {
      const cached = await getDevices(next.fingerprint);
      setDevices([...cached].sort(sortDevices));
    } catch {
      setDevices([]);
    }
  }

  async function onScan() {
    if (!network) return;
    setError(null);
    setScanning(true);
    setProgress({
      checked: 0,
      total: network.hostCount ?? 0,
      found: 0,
      phase: "starting",
    });
    setDevices([]);
    setSelected(null);
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
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setScanning(false);
      setProgress(null);
    }
  }

  async function onCancel() {
    await cancelScan();
  }

  async function onSelectNetwork(id: string) {
    const next = networks.find((n) => networkOptionId(n) === id);
    if (!next) return;
    if (next.fingerprint === network?.fingerprint) {
      setNetwork(next);
      return;
    }
    setNetwork(next);
    await loadDevicesFor(next);
  }

  async function onClear() {
    if (!network) return;
    setError(null);
    try {
      await clearDevices(network.fingerprint);
      setDevices([]);
      setSelected(null);
    } catch (e) {
      setError(String(e));
    }
  }

  async function saveNetworkName() {
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
  }

  async function saveNickname() {
    if (!selected) return;
    const key = deviceKey(selected);
    const value = nicknameDraft.trim() || null;
    await setDeviceNickname(key, value);
    setDevices((prev) =>
      prev.map((d) =>
        d.ip === selected.ip ? { ...d, nickname: value } : d,
      ),
    );
    setSelected((s) => (s ? { ...s, nickname: value } : s));
  }

  const onlineCount = devices.filter((d) => d.online).length;
  const pct =
    progress && progress.total > 0
      ? Math.min(100, Math.round((progress.checked / progress.total) * 100))
      : scanning
        ? 5
        : 0;

  return (
    <div className="devices-page">
      <div className="page-actions">
        <button
          className="btn ghost"
          type="button"
          onClick={onClear}
          disabled={scanning || !network || devices.length === 0}
          title="Clear cached devices for this network"
        >
          Clear
        </button>
        {scanning ? (
          <button className="btn ghost" type="button" onClick={onCancel}>
            Cancel
          </button>
        ) : null}
        <button
          className="btn primary"
          type="button"
          onClick={onScan}
          disabled={scanning || !network}
        >
          {scanning ? "Scanning…" : "Scan network"}
        </button>
      </div>

      <section className="netbar" aria-label="Network summary">
        <div className="netstat">
          <span className="netstat-label">Network</span>
          {networks.length > 1 ? (
            <select
              className="netstat-select"
              value={network ? networkOptionId(network) : ""}
              onChange={(e) => onSelectNetwork(e.target.value)}
              disabled={scanning}
              aria-label="Select network to view and scan"
            >
              {networks.map((n) => (
                <option key={networkOptionId(n)} value={networkOptionId(n)}>
                  {networkLabel(n)} — {n.localIp} ({n.cidr})
                </option>
              ))}
            </select>
          ) : (
            <span className="netstat-value">
              {network ? networkLabel(network) : "—"}
            </span>
          )}
        </div>
        <NetStat label="Your IP" value={network?.localIp ?? "—"} mono />
        <NetStat label="Subnet" value={network?.cidr ?? "—"} mono />
        <NetStat label="Gateway" value={network?.gateway ?? "—"} mono />
        <NetStat
          label="External IP"
          value={network?.externalIp ?? "—"}
          mono
        />
        <NetStat
          label="Devices"
          value={devices.length ? `${onlineCount} online` : "—"}
        />
      </section>

      {network ? (
        <div className="network-rename">
          <label className="nick-field network-rename-field">
            <span>Network name</span>
            <div className="nick-row">
              <input
                value={networkNameDraft}
                onChange={(e) => setNetworkNameDraft(e.target.value)}
                placeholder={network.autoName}
                disabled={scanning}
              />
              <button
                className="btn primary small"
                type="button"
                onClick={saveNetworkName}
                disabled={scanning}
              >
                Save
              </button>
            </div>
          </label>
          <p className="muted network-meta">
            {[
              network.media,
              network.ssid ? `SSID ${network.ssid}` : null,
              network.searchDomain ? `domain ${network.searchDomain}` : null,
              network.interfaceName,
            ]
              .filter(Boolean)
              .join(" · ")}
          </p>
        </div>
      ) : null}

      {scanning || progress ? (
        <div className="progress-wrap" role="status">
          <div className="progress-meta">
            <span>{progress?.phase ?? "starting"}</span>
            <span>
              {progress
                ? `${progress.checked} / ${progress.total}`
                : "…"}
            </span>
          </div>
          <div className="progress-track">
            <div className="progress-fill" style={{ width: `${pct}%` }} />
          </div>
        </div>
      ) : null}

      {error ? <p className="error">{error}</p> : null}

      <div className={`workspace ${selected ? "split" : ""}`}>
        <main className="list-pane">
          <div className="list-head">
            <h2>Devices</h2>
            <span className="muted">
              {devices.length
                ? `${devices.length} on this network`
                : "Run a scan to discover hosts"}
            </span>
          </div>

          <ul className="device-list">
            {devices.map((device) => (
              <li key={device.ip}>
                <button
                  type="button"
                  className={`device-row ${selected?.ip === device.ip ? "active" : ""}`}
                  onClick={() => setSelected(device)}
                >
                  <span className={`pulse ${device.online ? "on" : "off"}`} />
                  <span className="device-main">
                    <span className="device-name">{displayName(device)}</span>
                    <span className="device-sub">
                      {[
                        device.isLocal ? "This computer" : null,
                        device.isGateway ? "Gateway" : null,
                        device.vendor,
                      ]
                        .filter(Boolean)
                        .join(" · ") || "Unknown vendor"}
                    </span>
                  </span>
                  <span className="device-ip mono">{device.ip}</span>
                </button>
              </li>
            ))}
          </ul>
        </main>

        {selected ? (
          <aside className="detail-pane" aria-label="Device details">
            <div className="detail-top">
              <h2>{displayName(selected)}</h2>
              <button
                type="button"
                className="btn ghost small"
                onClick={() => setSelected(null)}
              >
                Close
              </button>
            </div>

            <dl className="detail-grid">
              <Detail label="Status" value={selected.online ? "Online" : "Offline"} />
              <Detail label="IP address" value={selected.ip} mono />
              <Detail label="MAC" value={selected.mac ?? "—"} mono />
              <Detail label="Vendor" value={selected.vendor ?? "—"} />
              <Detail label="Hostname" value={selected.hostname ?? "—"} />
              <Detail
                label="Last seen"
                value={
                  selected.lastSeen
                    ? new Date(selected.lastSeen * 1000).toLocaleString()
                    : "—"
                }
              />
            </dl>

            <PingPanel ip={selected.ip} />

            <label className="nick-field">
              <span>Nickname</span>
              <div className="nick-row">
                <input
                  value={nicknameDraft}
                  onChange={(e) => setNicknameDraft(e.target.value)}
                  placeholder="Optional label"
                />
                <button className="btn primary small" type="button" onClick={saveNickname}>
                  Save
                </button>
              </div>
            </label>
          </aside>
        ) : null}
      </div>
    </div>
  );
}

function NetStat({
  label,
  value,
  mono,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div className="netstat">
      <span className="netstat-label">{label}</span>
      <span className={`netstat-value ${mono ? "mono" : ""}`}>{value}</span>
    </div>
  );
}

function Detail({
  label,
  value,
  mono,
}: {
  label: string;
  value: string;
  mono?: boolean;
}) {
  return (
    <div>
      <dt>{label}</dt>
      <dd className={mono ? "mono" : undefined}>{value}</dd>
    </div>
  );
}

const PING_HISTORY_LIMIT = 20;
const PING_INTERVAL_MS = 1000;

function PingPanel({ ip }: { ip: string }) {
  const [running, setRunning] = useState(false);
  const [history, setHistory] = useState<PingOutcome[]>([]);
  const runningRef = useRef(false);

  useEffect(() => {
    setRunning(false);
    setHistory([]);
    return () => {
      runningRef.current = false;
    };
  }, [ip]);

  useEffect(() => {
    runningRef.current = running;
    if (!running) return;

    let timer: ReturnType<typeof setTimeout> | undefined;
    const tick = async () => {
      const outcome = await pingDevice(ip).catch(
        (e): PingOutcome => ({
          method: "icmp",
          success: false,
          latencyMs: null,
          error: String(e),
        }),
      );
      if (!runningRef.current) return;
      setHistory((prev) => [...prev.slice(-(PING_HISTORY_LIMIT - 1)), outcome]);
      timer = setTimeout(tick, PING_INTERVAL_MS);
    };
    tick();

    return () => {
      if (timer) clearTimeout(timer);
    };
  }, [running, ip]);

  const sent = history.length;
  const replies = history.filter((h) => h.success);
  const lossPct = sent ? Math.round(((sent - replies.length) / sent) * 100) : 0;
  const latencies = replies
    .map((h) => h.latencyMs)
    .filter((v): v is number => v !== null);
  const last = history[history.length - 1];
  const avg = latencies.length
    ? latencies.reduce((a, b) => a + b, 0) / latencies.length
    : null;
  const max = latencies.length ? Math.max(...latencies) : null;

  const fmt = (v: number | null | undefined) =>
    v === null || v === undefined ? "—" : `${v < 10 ? v.toFixed(2) : v.toFixed(1)} ms`;

  return (
    <div className="ping-panel">
      <div className="ping-head">
        <h3>Ping</h3>
        <button
          className={`btn small ${running ? "ghost" : "primary"}`}
          type="button"
          onClick={() => setRunning((r) => !r)}
        >
          {running ? "Stop" : "Start"}
        </button>
      </div>

      {sent ? (
        <>
          <div className="ping-stats">
            <span>
              Last:{" "}
              <strong>
                {last?.success ? fmt(last.latencyMs) : "timeout"}
              </strong>
            </span>
            <span>
              Avg: <strong>{fmt(avg)}</strong>
            </span>
            <span>
              Loss: <strong>{lossPct}%</strong>
            </span>
            <span className="muted">
              {replies.length}/{sent} replies
              {last ? ` · ${last.method}` : ""}
            </span>
          </div>
          <div className="ping-history" aria-label="Ping history">
            {history.map((h, i) => (
              <span
                key={i}
                className={`ping-bar ${h.success ? "ok" : "fail"}`}
                style={{
                  height: h.success
                    ? `${Math.max(
                        12,
                        max && h.latencyMs
                          ? Math.min(100, (h.latencyMs / max) * 100)
                          : 40,
                      )}%`
                    : "100%",
                }}
                title={
                  h.success
                    ? `${fmt(h.latencyMs)} (${h.method})`
                    : (h.error ?? "timeout")
                }
              />
            ))}
          </div>
        </>
      ) : (
        <p className="muted ping-hint">
          {running
            ? "Waiting for first reply…"
            : "Measure latency and packet loss for this device."}
        </p>
      )}
    </div>
  );
}

function sortDevices(a: Device, b: Device): number {
  if (a.isLocal !== b.isLocal) return a.isLocal ? -1 : 1;
  if (a.isGateway !== b.isGateway) return a.isGateway ? -1 : 1;
  if (a.online !== b.online) return a.online ? -1 : 1;
  return a.ip.localeCompare(b.ip, undefined, { numeric: true });
}
