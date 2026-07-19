import { useEffect, useState } from "react";
import { getScanRun, listScanRuns } from "../api";
import {
  displayName,
  type Device,
  type ScanRunDetail,
  type ScanRunSummary,
} from "../types";

export function HistoryPage() {
  const [runs, setRuns] = useState<ScanRunSummary[]>([]);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [detail, setDetail] = useState<ScanRunDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      setLoading(true);
      setError(null);
      try {
        const listed = await listScanRuns(100, 0);
        if (cancelled) return;
        setRuns(listed);
        if (listed.length) {
          setSelectedId(listed[0].id);
        }
      } catch (e) {
        if (!cancelled) setError(String(e));
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (selectedId == null) {
      setDetail(null);
      return;
    }
    let cancelled = false;
    (async () => {
      try {
        const next = await getScanRun(selectedId);
        if (!cancelled) setDetail(next);
      } catch (e) {
        if (!cancelled) setError(String(e));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [selectedId]);

  return (
    <div className="tool-page history-page">
      <header className="tool-intro">
        <h2>History</h2>
        <p className="muted">
          Past scan runs stored locally on this device. History never leaves
          your machine.
        </p>
      </header>

      {error ? <p className="error">{error}</p> : null}

      {loading ? (
        <p className="muted">Loading scan history…</p>
      ) : !runs.length ? (
        <div className="coming-soon" role="status">
          <p className="coming-soon-title">No scans yet</p>
          <p className="muted">
            Complete a scan on the Devices page to start building history.
          </p>
        </div>
      ) : (
        <div className={`workspace ${detail ? "split" : ""}`}>
          <main className="list-pane">
            <div className="list-head">
              <h2>Scan runs</h2>
              <span className="muted">{runs.length} saved</span>
            </div>
            <ul className="device-list">
              {runs.map((run) => (
                <li key={run.id}>
                  <button
                    type="button"
                    className={`device-row ${selectedId === run.id ? "active" : ""}`}
                    onClick={() => setSelectedId(run.id)}
                  >
                    <span
                      className={`pulse ${run.cancelled ? "off" : "on"}`}
                    />
                    <span className="device-main">
                      <span className="device-name">{run.networkName}</span>
                      <span className="device-sub">
                        {new Date(run.finishedAt * 1000).toLocaleString()}
                        {run.cancelled ? " · cancelled" : ""}
                        {run.externalIp ? ` · wan ${run.externalIp}` : ""}
                      </span>
                    </span>
                    <span className="device-ip mono">
                      {run.onlineCount}/{run.deviceCount}
                    </span>
                  </button>
                </li>
              ))}
            </ul>
          </main>

          {detail ? (
            <aside className="detail-pane" aria-label="Scan run details">
              <div className="detail-top">
                <h2>{detail.run.networkName}</h2>
              </div>
              <dl className="detail-grid">
                <Detail
                  label="Finished"
                  value={new Date(detail.run.finishedAt * 1000).toLocaleString()}
                />
                <Detail label="Subnet" value={detail.run.cidr} mono />
                <Detail label="Interface" value={detail.run.interfaceName} />
                <Detail label="Local IP" value={detail.run.localIp} mono />
                <Detail
                  label="Gateway"
                  value={detail.run.gateway ?? "—"}
                  mono
                />
                <Detail
                  label="External IP"
                  value={detail.run.externalIp ?? "—"}
                  mono
                />
                <Detail
                  label="Devices"
                  value={`${detail.run.onlineCount} online / ${detail.run.deviceCount} total`}
                />
              </dl>

              <div className="history-devices">
                <h3>Devices in this run</h3>
                <ul className="history-device-list">
                  {detail.devices.map((device) => (
                    <HistoryDeviceRow key={device.ip} device={device} />
                  ))}
                </ul>
              </div>
            </aside>
          ) : null}
        </div>
      )}
    </div>
  );
}

function HistoryDeviceRow({ device }: { device: Device }) {
  return (
    <li className="history-device-row">
      <span className={`pulse ${device.online ? "on" : "off"}`} />
      <span className="device-main">
        <span className="device-name">{displayName(device)}</span>
        <span className="device-sub">
          {[device.vendor, device.hostname].filter(Boolean).join(" · ") || "—"}
        </span>
      </span>
      <span className="mono device-ip">{device.ip}</span>
    </li>
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
