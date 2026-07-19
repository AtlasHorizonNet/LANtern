import type { ReactNode } from "react";
import type { UpdateStatus } from "../updater";

export function UpdateBanner({
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
