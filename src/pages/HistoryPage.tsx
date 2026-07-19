export function HistoryPage() {
  return (
    <div className="tool-page">
      <header className="tool-intro">
        <h2>History</h2>
        <p className="muted">
          Browse past scan runs — when they happened, which network was scanned,
          and which devices were found.
        </p>
      </header>

      <div className="coming-soon" role="status">
        <p className="coming-soon-title">Coming soon</p>
        <p className="muted">
          Scan history will appear here once local run storage is ready. For now,
          use the Devices page for the current session.
        </p>
      </div>
    </div>
  );
}
