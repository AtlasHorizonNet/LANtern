import { getVersion } from "@tauri-apps/api/app";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

const LATEST_JSON_URL =
  "https://github.com/AtlasHorizonNet/LANtern/releases/latest/download/latest.json";

export type UpdateStatus =
  | { state: "idle" }
  | { state: "checking" }
  | { state: "up-to-date" }
  | { state: "available"; version: string; notes: string | null }
  | { state: "downloading"; received: number; total: number | null }
  | { state: "installed" }
  | { state: "error"; message: string };

/** Version metadata shown in Settings → About. */
export type VersionInfo = {
  installed: string | null;
  /** Latest release version from GitHub / the updater endpoint. */
  github: string | null;
  /** Epoch ms of the most recent update check (success or failure). */
  lastCheckedAt: number | null;
};

export type UpdateCheckResult = {
  status: UpdateStatus;
  version: VersionInfo;
};

let pending: Update | null = null;
let versionInfo: VersionInfo = {
  installed: null,
  github: null,
  lastCheckedAt: null,
};

export function getVersionInfo(): VersionInfo {
  return versionInfo;
}

export async function loadInstalledVersion(): Promise<string | null> {
  try {
    const installed = await getVersion();
    versionInfo = { ...versionInfo, installed };
    return installed;
  } catch {
    return versionInfo.installed;
  }
}

async function fetchGithubLatestVersion(): Promise<string | null> {
  try {
    const res = await fetch(LATEST_JSON_URL, { cache: "no-store" });
    if (!res.ok) return null;
    const data: unknown = await res.json();
    if (
      data &&
      typeof data === "object" &&
      "version" in data &&
      typeof (data as { version: unknown }).version === "string"
    ) {
      return (data as { version: string }).version;
    }
    return null;
  } catch {
    return null;
  }
}

export async function checkForUpdate(): Promise<UpdateCheckResult> {
  const lastCheckedAt = Date.now();
  if (!versionInfo.installed) {
    await loadInstalledVersion();
  }

  try {
    const update = await check();
    if (update) {
      pending = update;
      versionInfo = {
        ...versionInfo,
        github: update.version,
        lastCheckedAt,
      };
      return {
        status: {
          state: "available",
          version: update.version,
          notes: update.body ?? null,
        },
        version: versionInfo,
      };
    }

    pending = null;
    const github =
      (await fetchGithubLatestVersion()) ?? versionInfo.installed;
    versionInfo = {
      ...versionInfo,
      github,
      lastCheckedAt,
    };
    return {
      status: { state: "up-to-date" },
      version: versionInfo,
    };
  } catch (e) {
    // Still try to surface the published version for About when possible.
    const github =
      versionInfo.github ?? (await fetchGithubLatestVersion());
    versionInfo = {
      ...versionInfo,
      github,
      lastCheckedAt,
    };
    return {
      status: { state: "error", message: String(e) },
      version: versionInfo,
    };
  }
}

export async function downloadAndInstall(
  onProgress: (received: number, total: number | null) => void,
): Promise<UpdateStatus> {
  if (!pending) {
    return { state: "error", message: "No update pending" };
  }
  try {
    let received = 0;
    let total: number | null = null;
    await pending.downloadAndInstall((event) => {
      switch (event.event) {
        case "Started":
          total = event.data.contentLength ?? null;
          onProgress(0, total);
          break;
        case "Progress":
          received += event.data.chunkLength;
          onProgress(received, total);
          break;
        case "Finished":
          onProgress(received, total);
          break;
      }
    });
    return { state: "installed" };
  } catch (e) {
    return { state: "error", message: String(e) };
  }
}

export async function restartApp(): Promise<void> {
  await relaunch();
}
