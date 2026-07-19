import { useEffect, useRef, useState } from "react";
import { pingDevice, setDeviceNickname } from "../api";
import { useScanSession } from "../scanSession";
import {
  deviceKey,
  displayName,
  networkLabel,
  type Device,
  type PingOutcome,
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

  useEffect(() => {
    setSelected(null);
  }, [network?.fingerprint]);

  useEffect(() => {
    setNicknameDraft(selected?.nickname ?? "");
  }, [selected?.ip, selected?.nickname]);

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
                <button
                  className="btn primary small"
                  type="button"
                  onClick={saveNickname}
                >
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
