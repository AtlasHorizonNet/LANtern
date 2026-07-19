import { useEffect, useState } from "react";
import { dhcpDiscover, dhcpPrivilegeNote, listNetworks } from "../api";
import type { DhcpDiscoverResult, NetworkInfo } from "../types";
import { networkLabel } from "../types";

export function DhcpPage() {
  const [networks, setNetworks] = useState<NetworkInfo[]>([]);
  const [network, setNetwork] = useState<NetworkInfo | null>(null);
  const [privilegeNote, setPrivilegeNote] = useState("");
  const [running, setRunning] = useState(false);
  const [result, setResult] = useState<DhcpDiscoverResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    listNetworks()
      .then((found) => {
        setNetworks(found);
        if (found.length) setNetwork(found[0]);
      })
      .catch((e) => setError(String(e)));
    dhcpPrivilegeNote()
      .then(setPrivilegeNote)
      .catch(() => {
        /* ignore */
      });
  }, []);

  async function onDiscover() {
    if (!network) return;
    setRunning(true);
    setError(null);
    setResult(null);
    try {
      const next = await dhcpDiscover(network.localIp, 4000);
      setResult(next);
      if (!next.success && next.error) {
        setError(next.error);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setRunning(false);
    }
  }

  return (
    <div className="tool-page">
      <header className="tool-intro">
        <h2>DHCP</h2>
        <p className="muted">
          Discover-only probe: send DHCPDISCOVER and report OFFER details.
          LANtern never sends DHCPREQUEST, so your active lease is not taken.
        </p>
      </header>

      {privilegeNote ? (
        <p className="privilege-note" role="note">
          {privilegeNote}
        </p>
      ) : null}

      <form
        className="tool-form"
        onSubmit={(e) => {
          e.preventDefault();
          onDiscover();
        }}
      >
        <label className="tool-field">
          <span>Interface / network</span>
          <select
            className="netstat-select"
            value={network?.fingerprint ?? ""}
            onChange={(e) => {
              const next = networks.find((n) => n.fingerprint === e.target.value);
              if (next) setNetwork(next);
            }}
            disabled={running || networks.length === 0}
          >
            {networks.map((n) => (
              <option key={n.fingerprint} value={n.fingerprint}>
                {networkLabel(n)} — {n.localIp}
              </option>
            ))}
          </select>
        </label>

        <div className="tool-submit">
          <button
            className="btn primary"
            type="submit"
            disabled={running || !network}
          >
            {running ? "Discovering…" : "Run DHCP discover"}
          </button>
        </div>
      </form>

      {error ? <p className="error">{error}</p> : null}

      {result?.offers.length ? (
        <div className="dns-results" aria-live="polite">
          {result.offers.map((offer, index) => (
            <article key={`${offer.offeredIp}-${index}`} className="dns-result ok">
              <div className="dns-result-head">
                <h3 className="mono">{offer.offeredIp}</h3>
                <span className="dns-badge ok">{offer.messageType}</span>
                <span className="muted mono">
                  {offer.latencyMs < 10
                    ? offer.latencyMs.toFixed(2)
                    : offer.latencyMs.toFixed(1)}{" "}
                  ms
                </span>
              </div>
              <ul className="dhcp-offer-details">
                <li>
                  <span className="muted">Server</span>{" "}
                  <span className="mono">{offer.serverIp ?? "—"}</span>
                </li>
                <li>
                  <span className="muted">Lease</span>{" "}
                  <span className="mono">
                    {offer.leaseSeconds != null
                      ? `${offer.leaseSeconds}s`
                      : "—"}
                  </span>
                </li>
                <li>
                  <span className="muted">Gateway</span>{" "}
                  <span className="mono">{offer.gateway ?? "—"}</span>
                </li>
                <li>
                  <span className="muted">Subnet</span>{" "}
                  <span className="mono">{offer.subnetMask ?? "—"}</span>
                </li>
                <li>
                  <span className="muted">DNS</span>{" "}
                  <span className="mono">
                    {offer.dnsServers.length
                      ? offer.dnsServers.join(", ")
                      : "—"}
                  </span>
                </li>
                <li>
                  <span className="muted">Domain</span>{" "}
                  <span className="mono">{offer.domain ?? "—"}</span>
                </li>
              </ul>
            </article>
          ))}
        </div>
      ) : null}
    </div>
  );
}
