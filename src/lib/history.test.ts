import { beforeEach, describe, expect, it } from "vitest";
import catalogJson from "../../crates/randommac-core/src/dns/catalog.json";
import { changeText, entryKind, filterHistory, isSuccess, type HistoryFilters } from "./history";
import { i18n } from "./i18n/index.svelte";
import type { DnsCatalog, HistoryEntry } from "./types";

const catalog = catalogJson as DnsCatalog;
const format = { separator: "colon", uppercase: true } as const;

const mac: HistoryEntry = {
  id: "m1",
  timestamp: "2026-11-18T10:00:00Z",
  adapterId: "eth",
  adapterName: "Ethernet",
  action: "apply",
  source: "ui",
  status: "applied",
  previousMac: "30:56:0F:09:8F:E4",
  newMac: "02:11:22:33:44:55",
};

const dns: HistoryEntry = {
  id: "d1",
  timestamp: "2026-11-18T11:00:00Z",
  adapterId: "wifi",
  adapterName: "Wi-Fi",
  action: "apply",
  source: "tray",
  status: "rolledBack",
  kind: "dns",
  dns: {
    previous: { ipv4: { mode: "automatic" }, ipv6: { mode: "automatic" }, encryption: "none" },
    requested: {
      ipv4: { mode: "static", servers: ["9.9.9.9", "149.112.112.112"] },
      ipv6: { mode: "automatic" },
      encryption: "doh",
    },
    label: { kind: "provider", provider: "quad9", variant: "recommended" },
  },
};

const none: HistoryFilters = { query: "", adapterId: "", status: "", kind: "" };
const ids = (list: HistoryEntry[]) => list.map((h) => h.id);

beforeEach(() => i18n.setLanguage("en"));

describe("history", () => {
  it("treats entries without a kind as MAC changes", () => {
    expect(entryKind(mac)).toBe("mac");
    expect(entryKind(dns)).toBe("dns");
    expect(isSuccess("unverified")).toBe(true);
    expect(isSuccess("rolledBack")).toBe(false);
  });

  it("filters by kind, adapter and result", () => {
    const all = [mac, dns];
    expect(ids(filterHistory(all, none))).toEqual(["m1", "d1"]);
    expect(ids(filterHistory(all, { ...none, kind: "dns" }))).toEqual(["d1"]);
    expect(ids(filterHistory(all, { ...none, kind: "mac" }))).toEqual(["m1"]);
    expect(ids(filterHistory(all, { ...none, adapterId: "eth" }))).toEqual(["m1"]);
    expect(ids(filterHistory(all, { ...none, status: "success" }))).toEqual(["m1"]);
    expect(ids(filterHistory(all, { ...none, status: "failed" }))).toEqual(["d1"]);
  });

  it("searches adapters, addresses, servers and providers", () => {
    const all = [mac, dns];
    const find = (query: string) => ids(filterHistory(all, { ...none, query }, catalog));
    expect(find("ethernet")).toEqual(["m1"]);
    expect(find("02:11")).toEqual(["m1"]);
    expect(find("021122")).toEqual(["m1"]);
    expect(find("149.112")).toEqual(["d1"]);
    expect(find("quad9")).toEqual(["d1"]);
    expect(find("doh")).toEqual(["d1"]);
    expect(find("nothing here")).toEqual([]);
    expect(find(":::")).toEqual([]);
    expect(ids(filterHistory([{ ...dns, dns: undefined }], { ...none, query: "quad9" }))).toEqual(
      [],
    );
  });

  it("shows the change of both kinds", () => {
    expect(changeText(mac, format)).toBe("30:56:0F:09:8F:E4 → 02:11:22:33:44:55");
    expect(
      changeText({ ...mac, newMac: undefined, requestedMac: "02:AA:BB:CC:DD:EE" }, format),
    ).toBe("30:56:0F:09:8F:E4 → 02:AA:BB:CC:DD:EE");
    expect(changeText(dns, format)).toBe(
      "Automatic → IPv4 9.9.9.9, 149.112.112.112 · IPv6 automatic · DoH",
    );
    expect(changeText({ ...dns, dns: undefined }, format)).toBe("— → —");
  });
});
