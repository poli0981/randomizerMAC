import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { clearMocks } from "@tauri-apps/api/mocks";
import { api, events } from "./api";
import { installDevMock } from "./devmock";
import { isLocallyAdministered, isMulticast } from "./mac";
import type { DnsRequest } from "./types";

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

  it("changes, restores and measures DNS servers", async () => {
    const [eth, wifi, usb] = await api.listAdapters();
    const request = (target: DnsRequest["target"]): DnsRequest => ({
      target,
      encryption: "preferred",
      allowFallback: false,
    });
    const catalog = await api.dnsCatalog();
    expect(catalog.providers.map((p) => p.id)).toContain("quad9");
    const wifiView = await api.dnsView(wifi!.id);
    expect(wifiView.matched).toEqual({
      kind: "provider",
      provider: "quad9",
      variant: "recommended",
    });
    expect(wifiView.restorable).toBe(true);
    expect((await api.dnsView(usb!.id)).capabilities.canChange).toBe(false);

    const applied = await api.dnsApply(
      eth!.id,
      request({ kind: "provider", provider: "cloudflare" }),
    );
    expect(applied.success).toBe(true);
    expect(applied.entry?.kind).toBe("dns");
    const ethView = await api.dnsView(eth!.id);
    expect(ethView.status?.dohTemplate).toBe("https://cloudflare-dns.com/dns-query");
    expect(ethView.status?.effectiveIpv4).toEqual(["1.1.1.1", "1.0.0.1"]);
    expect(ethView.restorable).toBe(true);

    await api.dnsApply(eth!.id, request({ kind: "custom", ipv4: ["192.0.2.53"], ipv6: [] }));
    await api.dnsApply(eth!.id, request({ kind: "localResolver" }));
    await api.dnsApply(eth!.id, {
      ...request({ kind: "preset", id: "0a1b2c3d4e5f6071" }),
      encryption: "none",
    });
    expect((await api.dnsView(eth!.id)).matched).toEqual({ kind: "preset", name: "Home router" });
    const restored = await api.dnsRestore(eth!.id);
    expect(restored.report?.observed?.ipv4).toEqual({ mode: "automatic" });
    expect((await api.dnsRestore(eth!.id)).report?.detailCode).toBe("NOTHING_TO_RESTORE");

    const all = await api.dnsApplyAll(request({ kind: "automatic" }));
    expect(all.map((r) => r.adapterName)).toEqual(["Ethernet", "Wi-Fi"]);

    await expect(api.dnsApply(usb!.id, request({ kind: "automatic" }))).rejects.toMatchObject({
      code: "UNSUPPORTED",
    });
    await expect(
      api.dnsApply(eth!.id, request({ kind: "provider", provider: "nope" })),
    ).rejects.toMatchObject({ code: "INVALID_DNS" });
    await expect(
      api.dnsApply(eth!.id, request({ kind: "preset", id: "nope" })),
    ).rejects.toMatchObject({
      code: "UNKNOWN_DNS_PRESET",
    });
    await expect(api.dnsView("nope")).rejects.toMatchObject({ code: "ADAPTER_NOT_FOUND" });
    await expect(api.dnsRestore("nope")).rejects.toMatchObject({ code: "ADAPTER_NOT_FOUND" });

    const latency = await api.dnsLatencyTest(["9.9.9.9", "10.0.0.1"]);
    expect(latency[0]!.outcome.kind).toBe("answered");
    expect(latency[1]!.outcome).toEqual({ kind: "timeout" });
  });

  it("answers other commands with nothing", async () => {
    await expect(api.restoreAll()).resolves.toBeNull();
    await expect(api.diagnostics()).resolves.toBeNull();
  });
});
