import { networkOptionId, useScanSession } from "../scanSession";
import { networkLabel } from "../types";

export function ScanPage({ onViewDevices }: { onViewDevices: () => void }) {
  const {
    networks,
    network,
    devices,
    progress,
    scanning,
    error,
    onlineCount,
    progressPct,
    selectNetwork,
    runScan,
    cancel,
  } = useScanSession();

  const canViewResults = devices.length > 0 && !scanning;

  async function onScan() {
    const outcome = await runScan();
    if (outcome === "ok") {
      onViewDevices();
    }
  }

  return (
    <div className="tool-page scan-page">
      <header className="tool-intro">
        <h2>Scan</h2>
        <p className="muted">
          Choose a network and run discovery. Results show up on the Devices
          page.
        </p>
      </header>

      <div className="page-actions scan-actions">
        {scanning ? (
          <button className="btn ghost" type="button" onClick={cancel}>
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
        {canViewResults ? (
          <button className="btn ghost" type="button" onClick={onViewDevices}>
            View devices
          </button>
        ) : null}
      </div>

      <section className="netbar" aria-label="Network to scan">
        <div className="netstat">
          <span className="netstat-label">Network</span>
          {networks.length > 1 ? (
            <select
              className="netstat-select"
              value={network ? networkOptionId(network) : ""}
              onChange={(e) => selectNetwork(e.target.value)}
              disabled={scanning}
              aria-label="Select network to scan"
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
          label="Hosts"
          value={
            network
              ? `${network.hostCount.toLocaleString()} max`
              : "—"
          }
        />
      </section>

      {network ? (
        <p className="muted network-meta scan-meta">
          {[
            network.media,
            network.ssid ? `SSID ${network.ssid}` : null,
            network.searchDomain ? `domain ${network.searchDomain}` : null,
            network.interfaceName,
          ]
            .filter(Boolean)
            .join(" · ")}
        </p>
      ) : null}

      {scanning || progress ? (
        <div className="progress-wrap" role="status">
          <div className="progress-meta">
            <span>{progress?.phase ?? "starting"}</span>
            <span>
              {progress ? `${progress.checked} / ${progress.total}` : "…"}
            </span>
          </div>
          <div className="progress-track">
            <div className="progress-fill" style={{ width: `${progressPct}%` }} />
          </div>
        </div>
      ) : null}

      {error ? <p className="error">{error}</p> : null}

      {!scanning && !progress && devices.length ? (
        <p className="muted scan-summary">
          Last scan found {devices.length} device
          {devices.length === 1 ? "" : "s"}
          {onlineCount ? ` (${onlineCount} online)` : ""}.{" "}
          <button type="button" className="text-link" onClick={onViewDevices}>
            Open Devices
          </button>
        </p>
      ) : null}

      {!scanning && !devices.length && !error ? (
        <p className="muted scan-summary">
          Ready to scan. Results will appear here as a summary and on Devices.
        </p>
      ) : null}
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
