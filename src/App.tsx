import { useEffect, useState, useTransition } from "react";
import { listen } from "@tauri-apps/api/event";
import {
  cancelScan,
  getDevices,
  getNetworkInfo,
  setDeviceNickname,
  startScan,
} from "./api";
import {
  deviceKey,
  displayName,
  type Device,
  type NetworkInfo,
  type ScanProgress,
} from "./types";
import "./App.css";

function App() {
  const [network, setNetwork] = useState<NetworkInfo | null>(null);
  const [devices, setDevices] = useState<Device[]>([]);
  const [selected, setSelected] = useState<Device | null>(null);
  const [progress, setProgress] = useState<ScanProgress | null>(null);
  const [scanning, setScanning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [nicknameDraft, setNicknameDraft] = useState("");
  const [, startTransition] = useTransition();

  useEffect(() => {
    let unprogress: (() => void) | undefined;
    let undevice: (() => void) | undefined;

    (async () => {
      try {
        const info = await getNetworkInfo();
        setNetwork(info);
      } catch (e) {
        setError(String(e));
      }

      try {
        const cached = await getDevices();
        if (cached.length) setDevices(cached);
      } catch {
        /* first run */
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

  async function onScan() {
    setError(null);
    setScanning(true);
    setProgress({ checked: 0, total: network?.hostCount ?? 0, found: 0, phase: "starting" });
    setDevices([]);
    setSelected(null);
    try {
      const result = await startScan();
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
          <p className="brand">LANtern</p>
          <p className="tagline">Light up every device on your local network.</p>
        </div>

        <div className="actions">
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

      <section className="netbar" aria-label="Network summary">
        <NetStat label="Interface" value={network?.interfaceName ?? "—"} />
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

function sortDevices(a: Device, b: Device): number {
  if (a.isLocal !== b.isLocal) return a.isLocal ? -1 : 1;
  if (a.isGateway !== b.isGateway) return a.isGateway ? -1 : 1;
  if (a.online !== b.online) return a.online ? -1 : 1;
  return a.ip.localeCompare(b.ip, undefined, { numeric: true });
}

export default App;
