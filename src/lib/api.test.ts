import { afterEach, describe, expect, it, vi } from "vitest";
import { emit } from "@tauri-apps/api/event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { api, events, toApiError } from "./api";
import type { Settings } from "./types";

afterEach(() => clearMocks());

function record() {
  const calls: { cmd: string; args: unknown }[] = [];
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    return null;
  });
  return calls;
}

describe("toApiError", () => {
  it("keeps errors from the backend", () => {
    const e = { code: "NOT_ELEVATED", message: "run as administrator" };
    expect(toApiError(e)).toBe(e);
  });

  it("wraps anything else", () => {
    expect(toApiError(new Error("boom"))).toEqual({ code: "INTERNAL", message: "Error: boom" });
    expect(toApiError("plain")).toEqual({ code: "INTERNAL", message: "plain" });
    expect(toApiError(null)).toEqual({ code: "INTERNAL", message: "null" });
    expect(toApiError({ code: "X" })).toEqual({ code: "INTERNAL", message: "[object Object]" });
  });
});

describe("api", () => {
  // Every wrapper with the command and arguments src-tauri/src/commands.rs expects.
  const settings = { schemaVersion: 2 } as Settings;
  const cases: [string, () => Promise<unknown>, string, unknown][] = [
    ["appInfo", () => api.appInfo(), "app_info", {}],
    ["uiReady", () => api.uiReady(), "ui_ready", {}],
    ["relaunchElevated", () => api.relaunchElevated(), "relaunch_elevated", {}],
    ["quit", () => api.quit(), "quit_app", {}],
    ["openUrl", () => api.openUrl("https://x"), "open_url", { url: "https://x" }],
    ["openLocation", () => api.openLocation("logs"), "open_location", { kind: "logs" }],
    ["diagnostics", () => api.diagnostics(), "diagnostics", {}],
    ["listAdapters", () => api.listAdapters(), "list_adapters", {}],
    [
      "setAdapterEnabled",
      () => api.setAdapterEnabled("a1", false),
      "set_adapter_enabled",
      { adapterId: "a1", enabled: false },
    ],
    [
      "generateMac",
      () => api.generateMac("a1", { kind: "random" }),
      "generate_mac",
      { adapterId: "a1", mode: { kind: "random" } },
    ],
    [
      "inspectMac",
      () => api.inspectMac("02:11:22:33:44:55", null),
      "inspect_mac",
      { input: "02:11:22:33:44:55", adapterId: null },
    ],
    ["listVendors", () => api.listVendors(), "list_vendors", {}],
    [
      "applyMac",
      () => api.applyMac("a1", "02:11:22:33:44:55"),
      "apply_mac",
      { adapterId: "a1", mac: "02:11:22:33:44:55", ignoreBlacklist: false },
    ],
    [
      "applyMac (ignore blacklist)",
      () => api.applyMac("a1", "02:11:22:33:44:55", true),
      "apply_mac",
      { adapterId: "a1", mac: "02:11:22:33:44:55", ignoreBlacklist: true },
    ],
    ["restoreMac", () => api.restoreMac("a1"), "restore_mac", { adapterId: "a1" }],
    ["restoreAll", () => api.restoreAll(), "restore_all", {}],
    ["historyList", () => api.historyList(), "history_list", {}],
    ["historyClear", () => api.historyClear(), "history_clear", {}],
    ["historyExport", () => api.historyExport("csv"), "history_export", { format: "csv" }],
    ["blacklistGet", () => api.blacklistGet(), "blacklist_get", {}],
    [
      "blacklistAdd",
      () => api.blacklistAdd("02:00:5E:*", null, "lab"),
      "blacklist_add",
      { pattern: "02:00:5E:*", adapterId: null, note: "lab" },
    ],
    ["blacklistRemove", () => api.blacklistRemove("b1"), "blacklist_remove", { id: "b1" }],
    ["blacklistImport", () => api.blacklistImport(), "blacklist_import", {}],
    ["blacklistExport", () => api.blacklistExport(), "blacklist_export", {}],
    ["settingsGet", () => api.settingsGet(), "settings_get", {}],
    ["settingsUpdate", () => api.settingsUpdate(settings), "settings_update", { settings }],
    ["settingsReset", () => api.settingsReset(), "settings_reset", {}],
    ["settingsExport", () => api.settingsExport(), "settings_export", {}],
    ["settingsImport", () => api.settingsImport(), "settings_import", {}],
    ["bundleExport", () => api.bundleExport(), "bundle_export", {}],
    ["v1Import", () => api.v1Import(), "v1_import", {}],
    ["logsGet", () => api.logsGet(7), "logs_get", { after: 7 }],
    ["logsClear", () => api.logsClear(), "logs_clear", {}],
    ["logsExport", () => api.logsExport(), "logs_export", {}],
    ["updateCheck", () => api.updateCheck(), "update_check", {}],
    ["updateInstall", () => api.updateInstall(), "update_install", {}],
  ];

  it.each(cases)("%s invokes its command", async (_name, run, cmd, args) => {
    const calls = record();
    await run();
    expect(calls).toHaveLength(1);
    expect(calls[0]!.cmd).toBe(cmd);
    expect(calls[0]!.args ?? {}).toEqual(args);
  });

  it("wraps every command", () => {
    const wrapped = new Set(cases.map(([name]) => name.split(" ")[0]));
    expect([...wrapped].sort()).toEqual(Object.keys(api).sort());
  });

  it("returns the command result", async () => {
    mockIPC((cmd) => (cmd === "diagnostics" ? "report" : null));
    await expect(api.diagnostics()).resolves.toBe("report");
  });

  it("rejects with an ApiError", async () => {
    mockIPC(() => {
      throw { code: "ADAPTER_NOT_FOUND", message: "no such adapter" };
    });
    await expect(api.restoreMac("x")).rejects.toEqual({
      code: "ADAPTER_NOT_FOUND",
      message: "no such adapter",
    });

    mockIPC(() => {
      throw new Error("ipc down");
    });
    await expect(api.listAdapters()).rejects.toEqual({
      code: "INTERNAL",
      message: "Error: ipc down",
    });
  });
});

describe("events", () => {
  const payloads: [keyof typeof events, string, unknown][] = [
    ["onProgress", "op:progress", { adapterId: "a1", stage: "writing" }],
    ["onBusy", "app:busy", { busy: true, adapterId: "a1" }],
    ["onSettingsChanged", "settings:changed", { schemaVersion: 2 }],
    ["onNavigate", "app:navigate", "history"],
    ["onUpdateAvailable", "update:available", { version: "9.9.9" }],
    ["onUpdateProgress", "update:progress", { downloaded: 10, total: 100 }],
  ];

  it.each(payloads)("%s passes the %s payload", async (name, event, payload) => {
    mockIPC(() => null, { shouldMockEvents: true });
    const cb = vi.fn();
    const subscribe = events[name] as (cb: (p: unknown) => void) => Promise<() => void>;
    const unlisten = await subscribe(cb);
    await emit(event, payload);
    expect(cb).toHaveBeenCalledWith(payload);
    unlisten();
  });

  it.each([
    ["onAdaptersChanged", "adapters:changed"],
    ["onHistoryChanged", "history:changed"],
    ["onBlacklistChanged", "blacklist:changed"],
    ["onUpdateInstalling", "update:installing"],
  ] as const)("%s fires on %s", async (name, event) => {
    mockIPC(() => null, { shouldMockEvents: true });
    const cb = vi.fn();
    await events[name](cb);
    await emit(event);
    expect(cb).toHaveBeenCalledOnce();
  });
});
