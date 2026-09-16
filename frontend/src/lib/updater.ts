import { IS_DEMO } from "./tauri.ts";

export type UpdateStatus =
  | { status: "unavailable" }
  | { status: "up-to-date"; currentVersion: string }
  | {
      status: "available";
      currentVersion: string;
      availableVersion: string;
      notes?: string;
      update: {
        downloadAndInstall: (
          onEvent?: (event: { event: string; data?: { chunkLength?: number; contentLength?: number } }) => void,
        ) => Promise<void>;
      };
    };

export async function getCurrentVersion(): Promise<string> {
  if (IS_DEMO) return "0.0.0-demo";
  const { getVersion } = await import("@tauri-apps/api/app");
  return getVersion();
}

export async function checkForUpdate(): Promise<UpdateStatus> {
  if (IS_DEMO) {
    return { status: "unavailable" };
  }
  const currentVersion = await getCurrentVersion();
  const { check } = await import("@tauri-apps/plugin-updater");
  const update = await check();
  if (!update) {
    return { status: "up-to-date", currentVersion };
  }
  return {
    status: "available",
    currentVersion,
    availableVersion: update.version,
    notes: update.body ?? undefined,
    update,
  };
}

export async function installUpdateAndRelaunch(
  update: {
    downloadAndInstall: (
      onEvent?: (event: { event: string; data?: { chunkLength?: number; contentLength?: number } }) => void,
    ) => Promise<void>;
  },
  onEvent?: (event: { event: string; data?: { chunkLength?: number; contentLength?: number } }) => void,
): Promise<void> {
  await update.downloadAndInstall(onEvent);
  const { relaunch } = await import("@tauri-apps/plugin-process");
  await relaunch();
}
