import { UpdateBanner } from "../components/UpdateBanner";
import {
  checkForUpdate,
  downloadAndInstall,
  restartApp,
  type UpdateStatus,
} from "../updater";

export function SettingsPage({
  update,
  setUpdate,
}: {
  update: UpdateStatus;
  setUpdate: (status: UpdateStatus) => void;
}) {
  async function onCheckUpdate() {
    setUpdate({ state: "checking" });
    setUpdate(await checkForUpdate());
  }

  async function onInstallUpdate() {
    setUpdate({ state: "downloading", received: 0, total: null });
    const result = await downloadAndInstall((received, total) => {
      setUpdate({ state: "downloading", received, total });
    });
    setUpdate(result);
  }

  return (
    <div className="tool-page">
      <header className="tool-intro">
        <h2>Settings</h2>
        <p className="muted">
          App maintenance and preferences. More options will land here over time.
        </p>
      </header>

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
