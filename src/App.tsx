import { useEffect, useState } from "react";
import { UpdateBanner } from "./components/UpdateBanner";
import { DevicesPage } from "./pages/DevicesPage";
import { DhcpPage } from "./pages/DhcpPage";
import { DnsPage } from "./pages/DnsPage";
import { HistoryPage } from "./pages/HistoryPage";
import { ScanPage } from "./pages/ScanPage";
import { SettingsPage } from "./pages/SettingsPage";
import { ScanSessionProvider } from "./scanSession";
import type { AppPage } from "./types";
import {
  checkForUpdate,
  downloadAndInstall,
  getVersionInfo,
  loadInstalledVersion,
  restartApp,
  type UpdateStatus,
  type VersionInfo,
} from "./updater";
import "./App.css";

const NAV_ITEMS: { id: AppPage; label: string }[] = [
  { id: "scan", label: "Scan" },
  { id: "devices", label: "Devices" },
  { id: "dns", label: "DNS" },
  { id: "dhcp", label: "DHCP" },
  { id: "history", label: "History" },
  { id: "settings", label: "Settings" },
];

function App() {
  const [page, setPage] = useState<AppPage>("scan");
  const [update, setUpdate] = useState<UpdateStatus>({ state: "idle" });
  const [version, setVersion] = useState<VersionInfo>(getVersionInfo());

  useEffect(() => {
    void loadInstalledVersion().then((installed) => {
      if (installed) setVersion((v) => ({ ...v, installed }));
    });

    // Non-blocking update check on launch; only surface available/error
    // in the global banner, but always refresh About metadata.
    checkForUpdate().then((result) => {
      setVersion(result.version);
      if (result.status.state === "available") {
        setUpdate(result.status);
      }
    });
  }, []);

  async function onInstallUpdate() {
    setUpdate({ state: "downloading", received: 0, total: null });
    const result = await downloadAndInstall((received, total) => {
      setUpdate({ state: "downloading", received, total });
    });
    setUpdate(result);
  }

  const showGlobalBanner =
    page !== "settings" &&
    (update.state === "available" ||
      update.state === "downloading" ||
      update.state === "installed" ||
      update.state === "error");

  return (
    <ScanSessionProvider>
      <div className={`app page-${page}`}>
        <div className="atmosphere" aria-hidden />

        <header className="top">
          <div className="brand-block">
            <img
              className="brand-logo"
              src="/logo.svg"
              alt=""
              width={36}
              height={36}
            />
            <div>
              <p className="brand">LANtern</p>
              <p className="tagline">
                Light up every device on your local network.
              </p>
            </div>
          </div>

          <nav className="app-nav" aria-label="Main">
            {NAV_ITEMS.map((item) => (
              <button
                key={item.id}
                type="button"
                className={`nav-link ${page === item.id ? "active" : ""}`}
                aria-current={page === item.id ? "page" : undefined}
                onClick={() => setPage(item.id)}
              >
                {item.label}
                {item.id === "settings" && update.state === "available" ? (
                  <span className="nav-dot" aria-label="Update available" />
                ) : null}
              </button>
            ))}
          </nav>
        </header>

        {showGlobalBanner ? (
          <UpdateBanner
            status={update}
            onInstall={onInstallUpdate}
            onRestart={restartApp}
            onDismiss={() => setUpdate({ state: "idle" })}
          />
        ) : null}

        <div className="page-body">
          {page === "scan" ? (
            <ScanPage onViewDevices={() => setPage("devices")} />
          ) : null}
          {page === "devices" ? <DevicesPage /> : null}
          {page === "dns" ? <DnsPage /> : null}
          {page === "dhcp" ? <DhcpPage /> : null}
          {page === "history" ? <HistoryPage /> : null}
          {page === "settings" ? (
            <SettingsPage
              update={update}
              setUpdate={setUpdate}
              version={version}
              setVersion={setVersion}
            />
          ) : null}
        </div>
      </div>
    </ScanSessionProvider>
  );
}

export default App;
