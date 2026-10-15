// Typed wrappers around the Tauri commands defined in src-tauri/src/commands.rs.
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  Adapter,
  ApiError,
  AppInfo,
  BlacklistEntry,
  GenerationMode,
  HistoryEntry,
  LogEntry,
  MacInfo,
  OpResult,
  Page,
  Settings,
  Stage,
  UpdateInfo,
  V1ImportReport,
  Vendor,
} from "./types";

/** Normalizes anything thrown by `invoke` into an ApiError. */
export function toApiError(e: unknown): ApiError {
  if (e && typeof e === "object" && "code" in e && "message" in e) {
    return e as ApiError;
  }
  return { code: "INTERNAL", message: String(e) };
}

async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    throw toApiError(e);
  }
}

export const api = {
  appInfo: () => call<AppInfo>("app_info"),
  uiReady: () => call<void>("ui_ready"),
  relaunchElevated: () => call<void>("relaunch_elevated"),
  quit: () => call<void>("quit_app"),
  openUrl: (url: string) => call<void>("open_url", { url }),
  openLocation: (kind: "data" | "logs" | "config") => call<void>("open_location", { kind }),
  diagnostics: () => call<string>("diagnostics"),

  listAdapters: () => call<Adapter[]>("list_adapters"),
  setAdapterEnabled: (adapterId: string, enabled: boolean) =>
    call<void>("set_adapter_enabled", { adapterId, enabled }),
  generateMac: (adapterId: string | null, mode: GenerationMode | null) =>
    call<MacInfo>("generate_mac", { adapterId, mode }),
  inspectMac: (input: string, adapterId: string | null) =>
    call<MacInfo>("inspect_mac", { input, adapterId }),
  listVendors: () => call<Vendor[]>("list_vendors"),
  applyMac: (adapterId: string, mac: string, ignoreBlacklist = false) =>
    call<OpResult>("apply_mac", { adapterId, mac, ignoreBlacklist }),
  restoreMac: (adapterId: string) => call<OpResult>("restore_mac", { adapterId }),
  restoreAll: () => call<OpResult[]>("restore_all"),

  historyList: () => call<HistoryEntry[]>("history_list"),
  historyClear: () => call<void>("history_clear"),
  historyExport: (format: "csv" | "json") => call<string | null>("history_export", { format }),

  blacklistGet: () => call<BlacklistEntry[]>("blacklist_get"),
  blacklistAdd: (pattern: string, adapterId: string | null, note: string | null) =>
    call<BlacklistEntry>("blacklist_add", { pattern, adapterId, note }),
  blacklistRemove: (id: string) => call<boolean>("blacklist_remove", { id }),
  blacklistImport: () => call<number | null>("blacklist_import"),
  blacklistExport: () => call<string | null>("blacklist_export"),

  settingsGet: () => call<Settings>("settings_get"),
  settingsUpdate: (settings: Settings) => call<Settings>("settings_update", { settings }),
  settingsReset: () => call<Settings>("settings_reset"),
  settingsExport: () => call<string | null>("settings_export"),
  settingsImport: () => call<Settings | null>("settings_import"),
  bundleExport: () => call<string | null>("bundle_export"),
  v1Import: () => call<V1ImportReport | null>("v1_import"),

  logsGet: (after: number | null) => call<LogEntry[]>("logs_get", { after }),
  logsClear: () => call<void>("logs_clear"),
  logsExport: () => call<string | null>("logs_export"),

  updateCheck: () => call<UpdateInfo>("update_check"),
  updateInstall: () => call<void>("update_install"),
};

export interface ProgressEvent {
  adapterId: string;
  stage: Stage;
}

export interface BusyEvent {
  busy: boolean;
  adapterId: string;
}

export const events = {
  onProgress: (cb: (e: ProgressEvent) => void): Promise<UnlistenFn> =>
    listen<ProgressEvent>("op:progress", (e) => cb(e.payload)),
  onBusy: (cb: (e: BusyEvent) => void): Promise<UnlistenFn> =>
    listen<BusyEvent>("app:busy", (e) => cb(e.payload)),
  onAdaptersChanged: (cb: () => void) => listen("adapters:changed", () => cb()),
  onHistoryChanged: (cb: () => void) => listen("history:changed", () => cb()),
  onBlacklistChanged: (cb: () => void) => listen("blacklist:changed", () => cb()),
  onSettingsChanged: (cb: (s: Settings) => void) =>
    listen<Settings>("settings:changed", (e) => cb(e.payload)),
  onNavigate: (cb: (p: Page) => void) => listen<Page>("app:navigate", (e) => cb(e.payload)),
  onUpdateAvailable: (cb: (u: UpdateInfo) => void) =>
    listen<UpdateInfo>("update:available", (e) => cb(e.payload)),
  onUpdateProgress: (cb: (p: { downloaded: number; total: number | null }) => void) =>
    listen<{ downloaded: number; total: number | null }>("update:progress", (e) => cb(e.payload)),
  onUpdateInstalling: (cb: () => void) => listen("update:installing", () => cb()),
};
