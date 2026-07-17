import { useEffect, useRef, useState, useTransition, type ReactNode } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  cancelScan,
  getDevices,
  listNetworks,
  pingDevice,
  setDeviceNickname,
  startScan,
} from "./api";
import {
  deviceKey,
  displayName,
  type Device,
  type NetworkInfo,
  type PingOutcome,
  type ScanProgress,
} from "./types";
import {
  checkForUpdate,
  downloadAndInstall,
  restartApp,
  type UpdateStatus,
} from "./updater";
import "./App.css";

function networkId(n: NetworkInfo): string {
  return `${n.interfaceName}|${n.localIp}|${n.cidr}`;
}

function App() {
  const [networks, setNetworks] = useState<NetworkInfo[]>([]);
  const [network, setNetwork] = useState<NetworkInfo | null>(null);
  const [devices, setDevices] = useState<Device[]>([]);
  const [selected, setSelected] = useState<Device | null>(null);
  const [progress, setProgress] = useState<ScanProgress | null>(null);
  const [scanning, setScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [nicknameDraft, setNicknameDraft] = useState("");
  const [update, setUpdate] = useState<UpdateStatus>({ state: "idle" });
  const [, startTransition] = useTransition();

  useEffect(() => {
    let unprogress: (() => void) | undefined;
    let undevice: (() => void) | undefined;

    (async () => {
      try {
        const found = await listNetworks();
        setNetworks(found);
        if (found.length) setNetwork(found[0]);
        else setError("No suitable IPv4 network interface found");
      } catch (e) {
        setError(String(e));
      }

      try {
        const cached = await getDevices();
        if (cached.length) setDevices(cached);
      } catch {
        /* first run */
      }

      // Non-blocking update check on launch; failures are silent.
      checkForUpdate().then((status) => {
        if (status.state === "available") setUpdate(status);
      });

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

  async function onScan() {
    setError(null);
    setScanning(true);
    setProgress({ checked: 0, total: network?.hostCount ?? 0, found: 0, phase: "starting" });
    setDevices([]);
    setSelected(null);
    try {
      const result = await startScan(network);
      setNetwork(result.network);
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

  function onSelectNetwork(id: string) {
    const next = networks.find((n) => networkId(n) === id);
    if (next) setNetwork(next);
  }

  async function onCheckUpdate() {
    setUpdate({ state: "checking" });
    setUpdate(await checkForUpdate());
  }

  async function onInstallUpdate() {
    setUpdate({ state: "downloading", received: 0, total: null });
    const result = await downloadAndInstall((received, total) => {
      setUpdate({ state: "downloading", received, total });
    });
    setUpdate(result);
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
    <div className="app">
      <div className="atmosphere" aria-hidden />

      <header className="top">
        <div className="brand-block">
          <img className="brand-logo" src="/logo.svg" alt="" width={36} height={36} />
          <div>
            <p className="brand">LANtern</p>
            <p className="tagline">Light up every device on your local network.</p>
          </div>
        </div>

        <div className="actions">
          <button
            className="btn ghost"
            type="button"
            onClick={onCheckUpdate}
            disabled={update.state === "checking" || update.state === "downloading"}
            title="Check GitHub for a newer version"
          >
            {update.state === "checking" ? "Checking…" : "Check for updates"}
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
            disabled={scanning}
          >
            {scanning ? "Scanning…" : "Scan network"}
          </button>
        </div>
      </header>

      {update.state !== "idle" && update.state !== "checking" ? (
        <UpdateBanner
          status={update}
          onInstall={onInstallUpdate}
          onRestart={restartApp}
          onDismiss={() => setUpdate({ state: "idle" })}
        />
      ) : null}

      <section className="netbar" aria-label="Network summary">
        <div className="netstat">
          <span className="netstat-label">Interface</span>
          {networks.length > 1 ? (
            <select
              className="netstat-select"
              value={network ? networkId(network) : ""}
              onChange={(e) => onSelectNetwork(e.target.value)}
              disabled={scanning}
              aria-label="Select network interface to scan"
            >
              {networks.map((n) => (
                <option key={networkId(n)} value={networkId(n)}>
                  {n.interfaceName} — {n.localIp} ({n.cidr})
                </option>
              ))}
            </select>
          ) : (
            <span className="netstat-value">
              {network?.interfaceName ?? "—"}
            </span>
          )}
        </div>
        <NetStat label="Your IP" value={network?.localIp ?? "—"} mono />
        <NetStat label="Subnet" value={network?.cidr ?? "—"} mono />
        <NetStat label="Gateway" value={network?.gateway ?? "—"} mono />
        <NetStat
          label="Devices"
          value={devices.length ? `${onlineCount} online` : "—"}
        />
      </section>

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
                ? `${devices.length} known`
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

  // Stop and clear when the selected device changes or the panel unmounts.
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

function UpdateBanner({
  status,
  onInstall,
  onRestart,
  onDismiss,
}: {
  status: UpdateStatus;
  onInstall: () => void;
  onRestart: () => void;
  onDismiss: () => void;
}) {
  let body: ReactNode = null;
  let tone = "info";

  switch (status.state) {
    case "up-to-date":
      body = <span>LANtern is up to date.</span>;
      break;
    case "available":
      body = (
        <>
          <span>
            Version <strong>{status.version}</strong> is available.
          </span>
          <button className="btn primary small" type="button" onClick={onInstall}>
            Download &amp; install
          </button>
        </>
      );
      break;
    case "downloading": {
      const pct =
        status.total && status.total > 0
          ? Math.min(100, Math.round((status.received / status.total) * 100))
          : null;
      body = (
        <span>
          Downloading update…{pct !== null ? ` ${pct}%` : ""}
        </span>
      );
      break;
    }
    case "installed":
      body = (
        <>
          <span>Update installed. Restart to finish.</span>
          <button className="btn primary small" type="button" onClick={onRestart}>
            Restart now
          </button>
        </>
      );
      break;
    case "error":
      tone = "warn";
      body = <span>Update failed: {status.message}</span>;
      break;
    default:
      return null;
  }

  return (
    <div className={`update-banner ${tone}`} role="status">
      {body}
      {status.state !== "downloading" ? (
        <button
          className="btn ghost small"
          type="button"
          onClick={onDismiss}
          aria-label="Dismiss update notice"
        >
          Dismiss
        </button>
      ) : null}
    </div>
  );
}

function sortDevices(a: Device, b: Device): number {
  if (a.isLocal !== b.isLocal) return a.isLocal ? -1 : 1;
  if (a.isGateway !== b.isGateway) return a.isGateway ? -1 : 1;
  if (a.online !== b.online) return a.online ? -1 : 1;
  return a.ip.localeCompare(b.ip, undefined, { numeric: true });
}

export default App;
