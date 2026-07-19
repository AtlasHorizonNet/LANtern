import {
  useEffect,
  useState,
  type Dispatch,
  type SetStateAction,
} from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { UpdateBanner } from "../components/UpdateBanner";
import {
  checkForUpdate,
  downloadAndInstall,
  loadInstalledVersion,
  restartApp,
  type UpdateStatus,
  type VersionInfo,
} from "../updater";

const SOURCE_URL = "https://github.com/AtlasHorizonNet/LANtern";

export function SettingsPage({
  update,
  setUpdate,
  version,
  setVersion,
}: {
  update: UpdateStatus;
  setUpdate: (status: UpdateStatus) => void;
  version: VersionInfo;
  setVersion: Dispatch<SetStateAction<VersionInfo>>;
}) {
  const [installedLoading, setInstalledLoading] = useState(!version.installed);

  useEffect(() => {
    let cancelled = false;
    loadInstalledVersion().then((installed) => {
      if (cancelled || !installed) return;
      setInstalledLoading(false);
      setVersion((prev) => ({ ...prev, installed }));
    });
    return () => {
      cancelled = true;
    };
  }, [setVersion]);

  async function onCheckUpdate() {
    setUpdate({ state: "checking" });
    const result = await checkForUpdate();
    setVersion(result.version);
    setUpdate(result.status);
  }

  async function onInstallUpdate() {
    setUpdate({ state: "downloading", received: 0, total: null });
    const result = await downloadAndInstall((received, total) => {
      setUpdate({ state: "downloading", received, total });
    });
    setUpdate(result);
  }

  async function onOpenSource() {
    try {
      await openUrl(SOURCE_URL);
    } catch {
      // Fallback when opener is unavailable (e.g. plain web preview).
      window.open(SOURCE_URL, "_blank", "noopener,noreferrer");
    }
  }

  return (
    <div className="tool-page">
      <header className="tool-intro">
        <h2>Settings</h2>
        <p className="muted">
          App maintenance and preferences. More options will land here over time.
        </p>
      </header>

      <section className="settings-section" aria-labelledby="about-heading">
        <div className="settings-section-head">
          <div>
            <h3 id="about-heading">About</h3>
            <p className="muted">
              Installed build versus the latest GitHub release.
            </p>
          </div>
        </div>

        <dl className="about-grid">
          <div>
            <dt>Installed version</dt>
            <dd className="mono">
              {installedLoading && !version.installed
                ? "…"
                : formatVersion(version.installed)}
            </dd>
          </div>
          <div>
            <dt>Latest on GitHub</dt>
            <dd className="mono">{formatVersion(version.github)}</dd>
          </div>
          <div>
            <dt>Last checked</dt>
            <dd>{formatLastChecked(version.lastCheckedAt)}</dd>
          </div>
          <div>
            <dt>Source code</dt>
            <dd>
              <button
                type="button"
                className="text-link"
                onClick={onOpenSource}
              >
                GitHub
              </button>
            </dd>
          </div>
        </dl>
      </section>

      <section className="settings-section" aria-labelledby="updates-heading">
        <div className="settings-section-head">
          <div>
            <h3 id="updates-heading">Updates</h3>
            <p className="muted">
              Check GitHub for a newer LANtern build and install it when available.
            </p>
          </div>
          <button
            className="btn ghost"
            type="button"
            onClick={onCheckUpdate}
            disabled={
              update.state === "checking" || update.state === "downloading"
            }
          >
            {update.state === "checking" ? "Checking…" : "Check for updates"}
          </button>
        </div>

        {update.state === "idle" ? (
          <p className="muted settings-idle">
            No update status yet. Run a check anytime, or wait for the quiet
            launch check to finish.
          </p>
        ) : update.state === "checking" ? (
          <p className="muted" role="status">
            Checking for updates…
          </p>
        ) : (
          <UpdateBanner
            status={update}
            onInstall={onInstallUpdate}
            onRestart={restartApp}
            onDismiss={() => setUpdate({ state: "idle" })}
          />
        )}
      </section>
    </div>
  );
}

function formatVersion(version: string | null): string {
  if (!version) return "—";
  return version.startsWith("v") ? version : `v${version}`;
}

function formatLastChecked(at: number | null): string {
  if (at == null) return "Not checked yet";
  return new Date(at).toLocaleString();
}
