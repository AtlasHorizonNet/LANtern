import { useEffect, useState, type FormEvent } from "react";
import { dnsLookup, dnsReverse, listNetworks } from "../api";
import type { DnsQueryResult, DnsRecordType } from "../types";

const RECORD_TYPES: DnsRecordType[] = ["A", "AAAA", "CNAME", "TXT", "MX", "NS"];

const PUBLIC_RESOLVERS = [
  { id: "cloudflare", label: "1.1.1.1", address: "1.1.1.1" },
  { id: "google", label: "8.8.8.8", address: "8.8.8.8" },
] as const;

type Mode = "forward" | "reverse";

export function DnsPage() {
  const [mode, setMode] = useState<Mode>("forward");
  const [query, setQuery] = useState("example.com");
  const [recordType, setRecordType] = useState<DnsRecordType>("A");
  const [useSystem, setUseSystem] = useState(true);
  const [useGateway, setUseGateway] = useState(true);
  const [gateway, setGateway] = useState<string | null>(null);
  const [publicIds, setPublicIds] = useState<string[]>(["cloudflare"]);
  const [customServer, setCustomServer] = useState("");
  const [running, setRunning] = useState(false);
  const [results, setResults] = useState<DnsQueryResult[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    listNetworks()
      .then((networks) => {
        const gw = networks.find((n) => n.gateway)?.gateway ?? null;
        setGateway(gw);
      })
      .catch(() => {
        setGateway(null);
      });
  }, []);

  function togglePublic(id: string) {
    setPublicIds((prev) =>
      prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id],
    );
  }

  async function onQuery(e: FormEvent) {
    e.preventDefault();
    setError(null);

    const target = query.trim();
    if (!target) {
      setError(mode === "forward" ? "Enter a hostname to look up." : "Enter an IP address.");
      return;
    }

    const servers: { label: string; address: string | null }[] = [];
    if (useSystem) servers.push({ label: "system", address: null });
    if (useGateway && gateway) servers.push({ label: gateway, address: gateway });
    for (const resolver of PUBLIC_RESOLVERS) {
      if (publicIds.includes(resolver.id)) {
        servers.push({ label: resolver.address, address: resolver.address });
      }
    }
    const custom = customServer.trim();
    if (custom) servers.push({ label: custom, address: custom });

    if (!servers.length) {
      setError("Select at least one resolver.");
      return;
    }

    // De-dupe by address label so gateway/custom don't double-hit the same IP.
    const unique = new Map<string, { label: string; address: string | null }>();
    for (const server of servers) {
      unique.set(server.label, server);
    }

    setRunning(true);
    setResults([]);
    try {
      const settled = await Promise.all(
        [...unique.values()].map(async (server) => {
          if (mode === "reverse") {
            return dnsReverse(target, server.address);
          }
          return dnsLookup(target, recordType, server.address);
        }),
      );
      setResults(settled);
    } catch (err) {
      setError(String(err));
    } finally {
      setRunning(false);
    }
  }

  return (
    <div className="tool-page">
      <header className="tool-intro">
        <h2>DNS</h2>
        <p className="muted">
          Query hostnames and reverse lookups against the system resolver, your
          gateway, public DNS, or a custom server — and compare the answers.
        </p>
      </header>

      <form className="tool-form" onSubmit={onQuery}>
        <div className="tool-mode" role="tablist" aria-label="Lookup mode">
          <button
            type="button"
            role="tab"
            aria-selected={mode === "forward"}
            className={`tool-mode-btn ${mode === "forward" ? "active" : ""}`}
            onClick={() => setMode("forward")}
          >
            Forward
          </button>
          <button
            type="button"
            role="tab"
            aria-selected={mode === "reverse"}
            className={`tool-mode-btn ${mode === "reverse" ? "active" : ""}`}
            onClick={() => setMode("reverse")}
          >
            Reverse
          </button>
        </div>

        <div className="tool-fields">
          <label className="tool-field">
            <span>{mode === "forward" ? "Hostname" : "IP address"}</span>
            <input
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              placeholder={mode === "forward" ? "example.com" : "1.1.1.1"}
              autoComplete="off"
              spellCheck={false}
            />
          </label>

          {mode === "forward" ? (
            <label className="tool-field">
              <span>Record type</span>
              <select
                className="netstat-select"
                value={recordType}
                onChange={(e) => setRecordType(e.target.value as DnsRecordType)}
              >
                {RECORD_TYPES.map((type) => (
                  <option key={type} value={type}>
                    {type}
                  </option>
                ))}
              </select>
            </label>
          ) : null}
        </div>

        <fieldset className="tool-resolvers">
          <legend>Resolvers</legend>
          <label className="check-row">
            <input
              type="checkbox"
              checked={useSystem}
              onChange={(e) => setUseSystem(e.target.checked)}
            />
            <span>System resolver</span>
          </label>
          <label className="check-row">
            <input
              type="checkbox"
              checked={useGateway}
              onChange={(e) => setUseGateway(e.target.checked)}
              disabled={!gateway}
            />
            <span>
              Gateway{gateway ? ` (${gateway})` : " (none detected)"}
            </span>
          </label>
          {PUBLIC_RESOLVERS.map((resolver) => (
            <label key={resolver.id} className="check-row">
              <input
                type="checkbox"
                checked={publicIds.includes(resolver.id)}
                onChange={() => togglePublic(resolver.id)}
              />
              <span>{resolver.label}</span>
            </label>
          ))}
          <label className="tool-field">
            <span>Custom DNS server</span>
            <input
              value={customServer}
              onChange={(e) => setCustomServer(e.target.value)}
              placeholder="Optional IP, e.g. 192.168.1.1"
              autoComplete="off"
              spellCheck={false}
            />
          </label>
        </fieldset>

        <div className="tool-submit">
          <button className="btn primary" type="submit" disabled={running}>
            {running ? "Querying…" : "Run lookup"}
          </button>
        </div>
      </form>

      {error ? <p className="error">{error}</p> : null}

      {results.length ? (
        <div className="dns-results" aria-live="polite">
          {results.map((result) => (
            <article
              key={`${result.server}-${result.recordType}-${result.query}`}
              className={`dns-result ${result.success ? "ok" : "fail"}`}
            >
              <div className="dns-result-head">
                <h3 className="mono">{result.server}</h3>
                <span className={`dns-badge ${result.success ? "ok" : "fail"}`}>
                  {result.success ? "Success" : "Failed"}
                </span>
                <span className="muted mono">
                  {result.latencyMs < 10
                    ? result.latencyMs.toFixed(2)
                    : result.latencyMs.toFixed(1)}{" "}
                  ms
                </span>
              </div>
              <p className="dns-query muted">
                {result.recordType} · {result.query}
              </p>
              {result.success ? (
                <ul className="dns-answers">
                  {result.answers.map((answer, index) => (
                    <li key={`${answer}-${index}`} className="mono">
                      {answer}
                    </li>
                  ))}
                </ul>
              ) : (
                <p className="error dns-error">
                  {result.error ?? "No answer"}
                </p>
              )}
            </article>
          ))}
        </div>
      ) : null}
    </div>
  );
}
