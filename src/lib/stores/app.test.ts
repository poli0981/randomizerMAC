import { afterEach, describe, expect, it, vi } from "vitest";
import { emit } from "@tauri-apps/api/event";
import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import type { Adapter, AppInfo, Settings } from "../types";

function adapter(id: string, extra: Partial<Adapter> = {}): Adapter {
  return {
    id,
    name: id,
    description: id,
    interfaceName: id,
    kind: "ethernet",
    isVirtual: false,
    state: "up",
    connected: false,
    currentMac: "30:56:0F:09:8F:E4",
    permanentMac: "30:56:0F:09:8F:E4",
    overrideValue: null,
    supportsChange: "yes",
    methods: ["mock"],
    nmConnection: null,
    driver: null,
    ipv4: [],
    ipv6: [],
    vendor: null,
    hardwareVendor: null,
    originalMac: null,
    changed: false,
    restorable: false,
    ...extra,
  };
}

const info = { version: "2.0.1", os: "linux", readOnly: false, mock: true } as AppInfo;

function settings(extra: Partial<Settings> = {}): Settings {
  return {
    schemaVersion: 2,
    language: "en",
    theme: "system",
    startWithSystem: false,
    startMinimized: false,
    closeToTray: true,
    showNotifications: true,
    confirmBeforeApply: false,
    autoCheckUpdates: false,
    generationMode: { kind: "random" },
    macFormat: { separator: "hyphen", uppercase: false },
    showVirtualAdapters: false,
    automation: {
      changeOnStartup: false,
      startupAdapterIds: [],
      intervalEnabled: false,
      intervalMinutes: 60,
      intervalAdapterIds: [],
      restoreOnExit: false,
    },
    linux: { method: "auto" },
    history: { maxEntries: 500, retentionDays: 90 },
    legacy: {},
    ...extra,
  };
}

type Command = (args: Record<string, unknown>) => unknown;

/** A fake backend: answers the commands the store uses and records every call. */
function backend(overrides: Record<string, Command> = {}) {
  const state = {
    settings: settings(),
    adapters: [adapter("virt", { isVirtual: true, connected: true }), adapter("eth")],
    calls: [] as string[],
  };
  const commands: Record<string, Command> = {
    app_info: () => info,
    settings_get: () => state.settings,
    settings_update: (args) => (state.settings = args.settings as Settings),
    list_adapters: () => state.adapters,
    history_list: () => [],
    blacklist_get: () => [],
    list_vendors: () => [{ id: "intel", name: "Intel", imitable: true, ouiCount: 1 }],
    ...overrides,
  };
  mockIPC(
    (cmd, args) => {
      state.calls.push(cmd);
      return commands[cmd]?.((args ?? {}) as Record<string, unknown>) ?? null;
    },
    { shouldMockEvents: true },
  );
  return state;
}

/** A fresh store (and i18n) for every test: the module keeps a singleton. */
async function load() {
  vi.resetModules();
  const { app } = await import("./app.svelte");
  const { i18n } = await import("../i18n/index.svelte");
  return { app, i18n };
}

const tick = () => new Promise((r) => setTimeout(r, 0));

afterEach(() => {
  clearMocks();
  vi.useRealTimers();
  vi.restoreAllMocks();
});

describe("init", () => {
  it("has safe defaults before anything is loaded", async () => {
    backend();
    const { app } = await load();
    expect(app.format).toEqual({ separator: "colon", uppercase: true });
    expect(app.readOnly).toBe(false);
    expect(app.selected).toBeNull();
    app.adapters = [adapter("v", { isVirtual: true }), adapter("p")];
    expect(app.visibleAdapters.map((a) => a.id)).toEqual(["p"]);
    app.info = { ...info, readOnly: true };
    expect(app.readOnly).toBe(true);
  });

  it("loads everything and selects the first connected physical adapter", async () => {
    backend();
    const { app } = await load();
    await app.init();
    expect(app.ready).toBe(true);
    expect(app.fatal).toBeNull();
    expect(app.info).toEqual(info);
    expect(app.vendors).toHaveLength(1);
    expect(app.selectedId).toBe("eth");
    expect(app.format).toEqual({ separator: "hyphen", uppercase: false });
    expect(app.readOnly).toBe(false);
  });

  it("keeps the saved selection and applies the language", async () => {
    const be = backend();
    be.settings = settings({ selectedAdapterId: "virt", language: "vi" });
    const { app, i18n } = await load();
    await app.init();
    expect(app.selectedId).toBe("virt");
    expect(app.selected?.id).toBe("virt");
    expect(i18n.locale).toBe("vi");
  });

  it("reports a fatal error when the backend does not answer", async () => {
    backend({
      app_info: () => {
        throw { code: "INTERNAL", message: "backend down" };
      },
    });
    const { app } = await load();
    await app.init();
    expect(app.fatal).toBe("backend down");
    expect(app.ready).toBe(false);
  });
});

describe("adapters", () => {
  it("falls back to any physical adapter, then to the first one", async () => {
    const be = backend();
    const { app } = await load();
    be.adapters = [adapter("v", { isVirtual: true }), adapter("p")];
    await app.refreshAdapters();
    expect(app.selectedId).toBe("p");

    app.selectedId = "gone";
    be.adapters = [adapter("v1", { isVirtual: true }), adapter("v2", { isVirtual: true })];
    await app.refreshAdapters();
    expect(app.selectedId).toBe("v1");

    be.adapters = [];
    await app.refreshAdapters();
    expect(app.selectedId).toBeNull();
    expect(app.selected).toBeNull();
  });

  it("hides virtual adapters unless shown or selected", async () => {
    const be = backend();
    be.adapters = [adapter("v", { isVirtual: true }), adapter("p")];
    const { app } = await load();
    await app.init();
    expect(app.visibleAdapters.map((a) => a.id)).toEqual(["p"]);
    app.selectedId = "v";
    expect(app.visibleAdapters.map((a) => a.id)).toEqual(["v", "p"]);
    app.selectedId = "p";
    app.settings = { ...app.settings!, showVirtualAdapters: true };
    expect(app.visibleAdapters).toHaveLength(2);
  });

  it("keeps the error and stops loading when listing fails", async () => {
    backend({
      list_adapters: () => {
        throw { code: "PERMISSION_DENIED", message: "no" };
      },
    });
    const { app } = await load();
    await app.refreshAdapters();
    expect(app.adaptersError).toEqual({ code: "PERMISSION_DENIED", message: "no" });
    expect(app.adaptersLoading).toBe(false);
  });

  it("shows a toast when history or the blacklist cannot be read", async () => {
    vi.spyOn(console, "warn").mockImplementation(() => {});
    const fail = () => {
      throw { code: "IO_ERROR", message: "disk" };
    };
    backend({ history_list: fail, blacklist_get: fail });
    const { app, i18n } = await load();
    i18n.setLanguage("en");
    await app.refreshHistory();
    await app.refreshBlacklist();
    expect(app.toasts.map((t) => t.message)).toEqual([
      i18n.error("IO_ERROR"),
      i18n.error("IO_ERROR"),
    ]);
  });
});

describe("saveSettings", () => {
  it("does nothing before the settings are loaded", async () => {
    backend();
    const { app } = await load();
    await expect(app.saveSettings((s) => (s.theme = "dark"))).resolves.toBe(false);
  });

  it("queues saves so quick edits never overwrite each other", async () => {
    const be = backend();
    const { app } = await load();
    await app.init();
    const first = app.saveSettings((s) => (s.theme = "dark"));
    const second = app.saveSettings((s) => (s.closeToTray = false));
    // Both edits show at once, before anything is saved.
    expect(app.settings?.theme).toBe("dark");
    expect(app.settings?.closeToTray).toBe(false);
    await expect(Promise.all([first, second])).resolves.toEqual([true, true]);
    expect(be.settings.theme).toBe("dark");
    expect(be.settings.closeToTray).toBe(false);
    expect(app.settings).toEqual(be.settings);
  });

  it("reloads the saved copy when a save fails", async () => {
    vi.spyOn(console, "warn").mockImplementation(() => {});
    backend({
      settings_update: () => {
        throw { code: "IO_ERROR", message: "disk full" };
      },
    });
    const { app } = await load();
    await app.init();
    await expect(app.saveSettings((s) => (s.theme = "light"))).resolves.toBe(false);
    expect(app.settings?.theme).toBe("system");
    expect(app.toasts).toHaveLength(1);
    expect(app.toasts[0]!.kind).toBe("error");
  });

  it("ignores its own echo while saves are pending", async () => {
    let release: (() => void) | undefined;
    const be = backend({
      settings_update: (args) =>
        new Promise((resolve) => {
          release = () => resolve((be.settings = args.settings as Settings));
        }),
    });
    const { app } = await load();
    await app.init();
    const saving = app.saveSettings((s) => (s.theme = "dark"));
    await tick();
    await emit("settings:changed", settings({ theme: "light" }));
    expect(app.settings?.theme).toBe("dark");
    release!();
    await saving;
    expect(app.settings?.theme).toBe("dark");

    await emit("settings:changed", settings({ theme: "light", language: "vi" }));
    expect(app.settings?.theme).toBe("light");
  });

  it("saves a new selection only when it changed", async () => {
    const be = backend();
    be.settings = settings({ selectedAdapterId: "eth" });
    const { app } = await load();
    await app.init();
    app.select("eth");
    await tick();
    expect(be.calls).not.toContain("settings_update");
    app.select("virt");
    expect(app.selectedId).toBe("virt");
    await vi.waitFor(() => expect(be.settings.selectedAdapterId).toBe("virt"));
  });
});

describe("events", () => {
  it("follows progress, busy state, navigation and updates", async () => {
    backend();
    const { app } = await load();
    await app.init();

    await emit("op:progress", { adapterId: "eth", stage: "writing" });
    expect(app.busy).toBeNull();
    await emit("app:busy", { busy: true, adapterId: "eth" });
    expect(app.busy).toEqual({ adapterId: "eth", stage: null });
    await emit("op:progress", { adapterId: "other", stage: "writing" });
    expect(app.busy?.stage).toBeNull();
    await emit("op:progress", { adapterId: "eth", stage: "verifying" });
    expect(app.busy?.stage).toBe("verifying");
    await emit("app:busy", { busy: false, adapterId: "eth" });
    expect(app.busy).toBeNull();

    await emit("app:navigate", "history");
    expect(app.page).toBe("history");
    app.navigate("settings");
    expect(app.page).toBe("settings");

    await emit("update:available", { version: "9.9.9", available: true });
    expect(app.update?.version).toBe("9.9.9");
    await emit("update:progress", { downloaded: 5, total: 10 });
    expect(app.updateProgress).toEqual({ downloaded: 5, total: 10 });
    await emit("update:installing");
    expect(app.updateInstalling).toBe(true);
  });

  it("reloads data when the backend reports changes", async () => {
    const be = backend();
    const { app } = await load();
    await app.init();
    be.calls.length = 0;
    be.adapters = [adapter("new", { connected: true })];
    await emit("adapters:changed");
    await emit("history:changed");
    await emit("blacklist:changed");
    await vi.waitFor(() => expect(app.adapters.map((a) => a.id)).toEqual(["new"]));
    expect(app.adaptersLoading).toBe(false);
    expect(be.calls).toEqual(
      expect.arrayContaining(["list_adapters", "history_list", "blacklist_get"]),
    );
  });
});

describe("toasts", () => {
  it("keeps the newest four and dismisses them after a while", async () => {
    backend();
    const { app } = await load();
    vi.useFakeTimers();
    for (const kind of ["info", "success", "warning", "error", "info"] as const) {
      app.toast(kind, kind);
    }
    expect(app.toasts.map((t) => t.message)).toEqual(["success", "warning", "error", "info"]);
    vi.advanceTimersByTime(4500);
    expect(app.toasts.map((t) => t.message)).toEqual(["warning", "error"]);
    vi.advanceTimersByTime(2000);
    expect(app.toasts.map((t) => t.message)).toEqual(["error"]);
    vi.advanceTimersByTime(1500);
    expect(app.toasts).toEqual([]);
  });

  it("translates errors", async () => {
    const warn = vi.spyOn(console, "warn").mockImplementation(() => {});
    backend();
    const { app, i18n } = await load();
    i18n.setLanguage("en");
    app.toastError({ code: "NOT_ELEVATED", message: "x" });
    app.toastError(new Error("odd"));
    expect(app.toasts.map((t) => t.message)).toEqual([
      i18n.t("error.NOT_ELEVATED"),
      i18n.t("error.INTERNAL"),
    ]);
    expect(warn).toHaveBeenCalledTimes(2);
    app.dismiss(app.toasts[0]!.id);
    expect(app.toasts).toHaveLength(1);
  });
});
