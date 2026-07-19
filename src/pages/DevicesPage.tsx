import { useEffect, useRef, useState } from "react";
import {
  pingDevice,
  scanPorts,
  setDeviceNickname,
  wakeOnLan,
} from "../api";
import { useScanSession } from "../scanSession";
import {
  automaticName,
  deviceKey,
  displayName,
  networkLabel,
  type Device,
  type PingOutcome,
  type PortScanResult,
  type PortState,
  type WakeResult,
} from "../types";

export function DevicesPage() {
  const {
    network,
    devices,
    setDevices,
    scanning,
    error,
    setError,
    networkNameDraft,
    setNetworkNameDraft,
    clear,
    saveNetworkName,
  } = useScanSession();
  const [selected, setSelected] = useState<Device | null>(null);
  const [nicknameDraft, setNicknameDraft] = useState("");
  const [renamingIp, setRenamingIp] = useState<string | null>(null);
  const [listRenameDraft, setListRenameDraft] = useState("");
  const listRenameRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    setSelected(null);
    setRenamingIp(null);
  }, [network?.fingerprint]);

  useEffect(() => {
    setNicknameDraft(selected?.nickname ?? "");
  }, [selected?.ip, selected?.nickname]);

  useEffect(() => {
    if (renamingIp) listRenameRef.current?.focus();
  }, [renamingIp]);

  function applyNicknameLocally(device: Device, value: string | null) {
    setDevices((prev) =>
      prev.map((d) => (d.ip === device.ip ? { ...d, nickname: value } : d)),
    );
    setSelected((s) => (s && s.ip === device.ip ? { ...s, nickname: value } : s));
    if (renamingIp === device.ip) {
      setListRenameDraft(value ?? "");
    }
    if (selected?.ip === device.ip) {
      setNicknameDraft(value ?? "");
    }
  }

  async function persistCustomName(device: Device, raw: string) {
    const value = raw.trim() || null;
    await setDeviceNickname(deviceKey(device), value);
    applyNicknameLocally(device, value);
  }

  async function saveNickname() {
    if (!selected) return;
    await persistCustomName(selected, nicknameDraft);
  }

  async function clearNickname() {
    if (!selected) return;
    await persistCustomName(selected, "");
  }

  function startListRename(device: Device) {
    setSelected(device);
    setRenamingIp(device.ip);
    setListRenameDraft(device.nickname ?? "");
  }

  async function commitListRename(device: Device) {
    await persistCustomName(device, listRenameDraft);
    setRenamingIp(null);
  }

  return (
    <div className="devices-page">
      <header className="devices-top">
        <div className="devices-title-row">
          <div>
            <h2 className="devices-heading">
              {network ? networkLabel(network) : "Devices"}
            </h2>
            <p className="muted devices-sub">
              {network
                ? `${network.localIp} · ${network.cidr}`
                : "Scan a network to see results here"}
            </p>
          </div>
          <button
            className="btn ghost"
            type="button"
            onClick={clear}
            disabled={scanning || !network || devices.length === 0}
            title="Clear cached devices for this network"
          >
            Clear
          </button>
        </div>

        {network ? (
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
        ) : null}
      </header>

      {error ? (
        <p className="error">
          {error}{" "}
          <button
            type="button"
            className="text-link"
            onClick={() => setError(null)}
          >
            Dismiss
          </button>
        </p>
      ) : null}

      <div className={`workspace ${selected ? "split" : ""}`}>
        <main className="list-pane">
          <div className="list-head">
            <h3>Results</h3>
            <span className="muted">
              {devices.length
                ? `${devices.length} on this network`
                : "Run a scan from the Scan page"}
            </span>
          </div>

          <ul className="device-list">
            {devices.map((device) => {
              const renaming = renamingIp === device.ip;
              const subtitle = device.nickname
                ? automaticName(device)
                : [
                    device.isLocal ? "This computer" : null,
                    device.isGateway ? "Gateway" : null,
                    device.vendor,
                  ]
                    .filter(Boolean)
                    .join(" · ") || "Unknown vendor";

              return (
                <li key={device.ip}>
                  {renaming ? (
                    <form
                      className={`device-row renaming ${selected?.ip === device.ip ? "active" : ""}`}
                      onSubmit={(e) => {
                        e.preventDefault();
                        void commitListRename(device);
                      }}
                    >
                      <span className={`pulse ${device.online ? "on" : "off"}`} />
                      <input
                        ref={listRenameRef}
                        className="device-rename-input"
                        value={listRenameDraft}
                        onChange={(e) => setListRenameDraft(e.target.value)}
                        placeholder={automaticName(device)}
                        aria-label={`Custom name for ${device.ip}`}
                        onKeyDown={(e) => {
                          if (e.key === "Escape") {
                            e.preventDefault();
                            setRenamingIp(null);
                          }
                        }}
                      />
                      <button className="btn primary small" type="submit">
                        Save
                      </button>
                      <button
                        className="btn ghost small"
                        type="button"
                        onClick={() => setRenamingIp(null)}
                      >
                        Cancel
                      </button>
                    </form>
                  ) : (
                    <div
                      className={`device-row ${selected?.ip === device.ip ? "active" : ""}`}
                    >
                      <button
                        type="button"
                        className="device-row-select"
                        onClick={() => setSelected(device)}
                      >
                        <span className={`pulse ${device.online ? "on" : "off"}`} />
                        <span className="device-main">
                          <span className="device-name">{displayName(device)}</span>
                          <span className="device-sub">{subtitle}</span>
                        </span>
                        <span className="device-ip mono">{device.ip}</span>
                      </button>
                      <button
                        type="button"
                        className="btn ghost small device-rename-btn"
                        onClick={() => startListRename(device)}
                        title="Set a custom name"
                      >
                        Rename
                      </button>
                    </div>
                  )}
                </li>
              );
            })}
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

            <label className="nick-field custom-name-field">
              <span>Custom name</span>
              <div className="nick-row">
                <input
                  value={nicknameDraft}
                  onChange={(e) => setNicknameDraft(e.target.value)}
                  placeholder={automaticName(selected)}
                  aria-label="Custom device name"
                  onKeyDown={(e) => {
                    if (e.key === "Enter") {
                      e.preventDefault();
                      void saveNickname();
                    }
                  }}
                />
                <button
                  className="btn primary small"
                  type="button"
                  onClick={saveNickname}
                >
                  Save
                </button>
                <button
                  className="btn ghost small"
                  type="button"
                  onClick={clearNickname}
                  disabled={!selected.nickname && !nicknameDraft.trim()}
                  title="Restore automatic display name"
                >
                  Clear
                </button>
              </div>
            </label>

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
            <PortScanPanel ip={selected.ip} />
            <WakePanel mac={selected.mac} />
          </aside>
        ) : null}
      </div>
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

function PortScanPanel({ ip }: { ip: string }) {
  const [portsSpec, setPortsSpec] = useState("");
  const [scanning, setScanning] = useState(false);
  const [result, setResult] = useState<PortScanResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState<"all" | PortState>("all");

  useEffect(() => {
    setResult(null);
    setError(null);
    setScanning(false);
    setFilter("all");
  }, [ip]);

  async function runScan() {
    setScanning(true);
    setError(null);
    try {
      const next = await scanPorts(ip, portsSpec.trim());
      setResult(next);
      setFilter("all");
    } catch (e) {
      setResult(null);
      setError(String(e));
    } finally {
      setScanning(false);
    }
  }

  const visible = result
    ? filter === "all"
      ? result.ports
      : result.ports.filter((p) => p.state === filter)
    : [];

  return (
    <div className="tool-panel">
      <div className="ping-head">
        <h3>Port scan</h3>
        <button
          className="btn primary small"
          type="button"
          onClick={runScan}
          disabled={scanning}
        >
          {scanning ? "Scanning…" : "Scan"}
        </button>
      </div>

      <label className="tool-field">
        <span>Ports</span>
        <input
          value={portsSpec}
          onChange={(e) => setPortsSpec(e.target.value)}
          placeholder="Defaults (22, 80, 443…) or 22,80,8000-8010"
          disabled={scanning}
        />
      </label>

      {error ? <p className="error tool-msg">{error}</p> : null}

      {result ? (
        <>
          <div className="ping-stats">
            <span>
              Open: <strong>{result.openCount}</strong>
            </span>
            <span>
              Scanned: <strong>{result.scanned}</strong>
            </span>
            <span className="muted">{result.durationMs.toFixed(0)} ms</span>
          </div>
          <div className="port-filter" role="group" aria-label="Filter ports">
            {(["all", "open", "closed", "filtered"] as const).map((key) => (
              <button
                key={key}
                type="button"
                className={`btn ghost small ${filter === key ? "active-filter" : ""}`}
                onClick={() => setFilter(key)}
              >
                {key === "all" ? "All" : key}
              </button>
            ))}
          </div>
          <ul className="port-list" aria-label="Port scan results">
            {visible.map((p) => (
              <li key={p.port} className={`port-row state-${p.state}`}>
                <span className="mono port-num">{p.port}</span>
                <span className="port-svc">{p.service ?? "—"}</span>
                <span className={`port-state state-${p.state}`}>{p.state}</span>
                <span className="muted mono port-lat">
                  {p.latencyMs != null
                    ? `${p.latencyMs < 10 ? p.latencyMs.toFixed(1) : p.latencyMs.toFixed(0)} ms`
                    : "—"}
                </span>
              </li>
            ))}
            {visible.length === 0 ? (
              <li className="muted port-empty">No ports match this filter.</li>
            ) : null}
          </ul>
        </>
      ) : (
        <p className="muted ping-hint">
          TCP connect scan of common ports by default. Enter a list or range for
          a custom scan.
        </p>
      )}
    </div>
  );
}

function WakePanel({ mac }: { mac: string | null }) {
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<WakeResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    setResult(null);
    setError(null);
    setBusy(false);
  }, [mac]);

  async function sendWake() {
    if (!mac) return;
    setBusy(true);
    setError(null);
    setResult(null);
    try {
      const next = await wakeOnLan(mac);
      setResult(next);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="tool-panel">
      <div className="ping-head">
        <h3>Wake-on-LAN</h3>
        <button
          className="btn primary small"
          type="button"
          onClick={sendWake}
          disabled={!mac || busy}
          title={mac ? `Send magic packet to ${mac}` : "MAC address required"}
        >
          {busy ? "Sending…" : "Wake"}
        </button>
      </div>

      {!mac ? (
        <p className="muted ping-hint">
          Needs a MAC address from the ARP/neighbor table. Re-scan while the
          host is online to learn it.
        </p>
      ) : (
        <p className="muted ping-hint">
          Broadcasts a magic packet for{" "}
          <span className="mono">{mac}</span>. The target NIC must support WoL
          and have it enabled in firmware/OS power settings.
        </p>
      )}

      {error ? <p className="error tool-msg">{error}</p> : null}
      {result ? (
        <p className={`tool-msg ${result.success ? "ok-msg" : "error"}`}>
          {result.message}
        </p>
      ) : null}
    </div>
  );
}
