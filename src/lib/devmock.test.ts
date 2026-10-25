import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { clearMocks } from "@tauri-apps/api/mocks";
import { api, events } from "./api";
import { installDevMock } from "./devmock";
import { isLocallyAdministered, isMulticast } from "./mac";

beforeEach(() => installDevMock());
afterEach(() => clearMocks());

describe("dev mock backend", () => {
  it("describes a demo installation", async () => {
    const info = await api.appInfo();
    expect(info.mock).toBe(true);
    expect(info.readOnly).toBe(false);
    expect(info.capabilities.canChange).toBe(true);
  });

  it("serves adapters, history, blacklist and vendors", async () => {
    const adapters = await api.listAdapters();
    expect(adapters.map((a) => a.kind)).toEqual(["ethernet", "wireless", "ethernet"]);
    const history = await api.historyList();
    expect(history.every((h) => adapters.some((a) => a.id === h.adapterId))).toBe(true);
    expect(await api.blacklistGet()).toHaveLength(1);
    expect((await api.listVendors()).map((v) => v.id)).toContain("intel");
  });

  it("keeps settings changes", async () => {
    const settings = await api.settingsGet();
    expect(settings.selectedAdapterId).toBe((await api.listAdapters())[1]!.id);
    const saved = await api.settingsUpdate({ ...settings, theme: "dark" });
    expect(saved.theme).toBe("dark");
    expect((await api.settingsGet()).theme).toBe("dark");
    await api.settingsUpdate(settings);
  });

  it("generates locally administered unicast addresses", async () => {
    for (let i = 0; i < 20; i++) {
      const { mac } = await api.generateMac(null, null);
      expect(isLocallyAdministered(mac!)).toBe(true);
      expect(isMulticast(mac!)).toBe(false);
    }
    expect((await api.inspectMac("x", null)).valid).toBe(true);
  });

  it("returns logs only once", async () => {
    const logs = await api.logsGet(null);
    expect(logs.map((l) => l.level)).toEqual(["info", "warn"]);
    expect(await api.logsGet(logs.at(-1)!.seq)).toEqual([]);
  });

  it("reports no update and accepts listeners", async () => {
    const update = await api.updateCheck();
    expect(update.available).toBe(false);
    const unlisten = await events.onBusy(() => {});
    expect(typeof unlisten).toBe("function");
  });

  it("answers other commands with nothing", async () => {
    await expect(api.restoreAll()).resolves.toBeNull();
    await expect(api.diagnostics()).resolves.toBeNull();
  });
});
