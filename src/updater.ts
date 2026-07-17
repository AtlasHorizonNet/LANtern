import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

export type UpdateStatus =
  | { state: "idle" }
  | { state: "checking" }
  | { state: "up-to-date" }
  | { state: "available"; version: string; notes: string | null }
  | { state: "downloading"; received: number; total: number | null }
  | { state: "installed" }
  | { state: "error"; message: string };

let pending: Update | null = null;

export async function checkForUpdate(): Promise<UpdateStatus> {
  try {
    const update = await check();
    if (update) {
      pending = update;
      return {
        state: "available",
        version: update.version,
        notes: update.body ?? null,
      };
    }
    pending = null;
    return { state: "up-to-date" };
  } catch (e) {
    return { state: "error", message: String(e) };
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
