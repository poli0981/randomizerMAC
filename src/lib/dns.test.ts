import { beforeEach, describe, expect, it } from "vitest";
import catalogJson from "../../crates/randommac-core/src/dns/catalog.json";
import {
  customProblems,
  defaultVariant,
  describeAll,
  describeDnsOutcome,
  dohTemplateProblem,
  encryptionOptions,
  endpointsFor,
  findVariant,
  formatMs,
  hasProblems,
  ipKey,
  isHostName,
  isUnusable,
  keepChoice,
  labelForSelection,
  labelText,
  latencyRows,
  modeName,
  newPresetId,
  noteText,
  outcomeText,
  parseIpv4,
  parseIpv6,
  presetFromDraft,
  presetProblems,
  reasonText,
  removePreset,
  requestFor,
  resolvedEncryption,
  selectionForLabel,
  selectionUsable,
  serverProblem,
  sortLatency,
  splitServers,
  summaryText,
  unavailableText,
  upsertPreset,
  type LatencyRow,
  type Selection,
} from "./dns";
import { i18n } from "./i18n/index.svelte";
import type {
  CustomDnsPreset,
  DnsCapabilities,
  DnsCatalog,
  DnsOpResult,
  DnsReport,
  ProbeOutcome,
} from "./types";

const catalog = catalogJson as DnsCatalog;

const caps = (doh: boolean, dot: boolean): DnsCapabilities => ({
  supported: true,
  canChange: true,
  ipv4: { available: true },
  ipv6: { available: true },
  doh: doh ? { available: true } : { available: false, reason: "DOH_REQUIRES_NEWER_WINDOWS" },
  dot: dot ? { available: true } : { available: false, reason: "DOT_UNSUPPORTED_WINDOWS" },
});

const lab: CustomDnsPreset = {
  id: "0123456789abcdef",
  name: "Lab",
  ipv4: ["192.0.2.53"],
  ipv6: [],
  dohTemplate: "https://dns.lab.example/dns-query",
};

beforeEach(() => i18n.setLanguage("en"));

describe("addresses", () => {
  it("parses IPv4 like Rust", () => {
    expect(parseIpv4("9.9.9.9")).toEqual([9, 9, 9, 9]);
    expect(parseIpv4("255.255.255.255")).toEqual([255, 255, 255, 255]);
    for (const bad of ["1.2.3", "1.2.3.4.5", "01.2.3.4", "256.1.1.1", "1.2.3.a", "", "1..2.3"]) {
      expect(parseIpv4(bad), bad).toBeNull();
    }
  });

  it("parses IPv6 like Rust", () => {
    expect(parseIpv6("2620:fe::fe")).toEqual([0x2620, 0xfe, 0, 0, 0, 0, 0, 0xfe]);
    expect(parseIpv6("::")).toEqual([0, 0, 0, 0, 0, 0, 0, 0]);
    expect(parseIpv6("::1")).toEqual([0, 0, 0, 0, 0, 0, 0, 1]);
    expect(parseIpv6("1::")).toEqual([1, 0, 0, 0, 0, 0, 0, 0]);
    expect(parseIpv6("1:2:3:4:5:6:7:8")).toEqual([1, 2, 3, 4, 5, 6, 7, 8]);
    expect(parseIpv6("::ffff:1.2.3.4")).toEqual([0, 0, 0, 0, 0, 0xffff, 0x0102, 0x0304]);
    expect(parseIpv6("64:ff9b::192.0.2.1")).toEqual([0x64, 0xff9b, 0, 0, 0, 0, 0xc000, 0x0201]);
    for (const bad of [
      "1:2:3:4:5:6:7",
      "1:2:3:4:5:6:7:8:9",
      "1::2::3",
      ":1",
      "1:::2",
      "12345::",
      "1.2.3.4::",
      "fe80::1%eth0",
      "[::1]",
      "g::1",
      "1:2:3:4:5:6:7::8",
    ]) {
      expect(parseIpv6(bad), bad).toBeNull();
    }
  });

  it("compares addresses by value", () => {
    expect(ipKey("2620:FE::FE")).toBe(ipKey("2620:fe:0:0::fe"));
    expect(ipKey("9.9.9.9")).toBe("4:9.9.9.9");
    expect(ipKey("dns.quad9.net")).toBeNull();
  });

  it("refuses addresses that cannot be DNS servers", () => {
    for (const bad of [
      "0.0.0.0",
      "255.255.255.255",
      "224.0.0.1",
      "239.1.1.1",
      "0.1.2.3",
      "::",
      "ff02::1",
      "fe80::1",
      "febf::1",
      "::ffff:1.2.3.4",
    ]) {
      expect(isUnusable(bad), bad).toBe(true);
    }
    for (const ok of ["9.9.9.9", "127.0.0.1", "::1", "2620:fe::fe", "fec0::1", "not an ip"]) {
      expect(isUnusable(ok), ok).toBe(false);
    }
  });

  it("splits and checks server lists", () => {
    expect(splitServers(" 9.9.9.9, 149.112.112.112;1.1.1.1  8.8.8.8 ,")).toEqual([
      "9.9.9.9",
      "149.112.112.112",
      "1.1.1.1",
      "8.8.8.8",
    ]);
    expect(serverProblem([], 4)).toBeNull();
    expect(serverProblem(["9.9.9.9", "149.112.112.112"], 4)).toBeNull();
    expect(serverProblem(["1.1.1.1", "1.0.0.1", "8.8.8.8"], 4)).toEqual({ code: "tooMany" });
    expect(serverProblem(["dns"], 4)).toEqual({ code: "invalid", value: "dns" });
    expect(serverProblem(["2620:fe::fe"], 4)).toEqual({
      code: "wrongFamily",
      value: "2620:fe::fe",
    });
    expect(serverProblem(["9.9.9.9"], 6)).toEqual({ code: "wrongFamily", value: "9.9.9.9" });
    expect(serverProblem(["224.0.0.1"], 4)).toEqual({ code: "unusable", value: "224.0.0.1" });
    expect(serverProblem(["2620:fe::fe", "2620:FE::FE"], 6)).toEqual({
      code: "duplicate",
      value: "2620:FE::FE",
    });
  });
});

describe("host names and templates", () => {
  it("accepts host names only", () => {
    for (const ok of ["dns.quad9.net", "one.one.one.one", "a-b.example", "localhost", "1.2.3"]) {
      expect(isHostName(ok), ok).toBe(true);
    }
    for (const bad of ["", "9.9.9.9", "::1", "-a.example", "a-.example", "a..b", "a_b.example"]) {
      expect(isHostName(bad), bad).toBe(false);
    }
    expect(isHostName(`${"a".repeat(64)}.example`)).toBe(false);
    expect(isHostName(Array(64).fill("abc").join("."))).toBe(false);
  });

  it("checks DoH templates like Rust", () => {
    expect(dohTemplateProblem("https://dns.quad9.net/dns-query", [])).toBeNull();
    expect(dohTemplateProblem("https://dns.example:8443/dns-query{?dns}", [])).toBeNull();
    expect(dohTemplateProblem("https://9.9.9.9/dns-query", ["9.9.9.9"])).toBeNull();
    expect(dohTemplateProblem("https://[2620:fe::fe]/dns-query", ["2620:FE::FE"])).toBeNull();
    expect(dohTemplateProblem("https://9.9.9.9/dns-query", ["9.9.9.9", "1.1.1.1"])).toBe(
      "ipMismatch",
    );
    expect(dohTemplateProblem("https://9.9.9.9/dns-query", [])).toBe("ipMismatch");
    for (const bad of [
      "http://dns.example/dns-query",
      "https://dns.example",
      "https://dns.example/",
      "https://user@dns.example/dns-query",
      "https://dns.example:0/q",
      "https://dns.example:65536/q",
      "https://dns.example:x/q",
      "https://dns example/q",
      "https://dns.example/q|x",
      "https://dnsé.example/q",
      "https://[2620:fe::fe/q",
      "https://[2620:fe::fe]x/q",
      "https://-bad.example/q",
      `https://dns.example/${"a".repeat(600)}`,
    ]) {
      expect(dohTemplateProblem(bad, []), bad).toBe("bad");
    }
  });
});

describe("selections", () => {
  const custom = (ipv4: string, ipv6 = "", doh = "", dot = ""): Selection => ({
    kind: "custom",
    ipv4,
    ipv6,
    doh,
    dot,
  });

  it("finds provider variants", () => {
    const quad9 = catalog.providers.find((p) => p.id === "quad9")!;
    expect(defaultVariant(quad9)?.default).toBe(true);
    expect(findVariant(catalog, "quad9")?.variant.id).toBe(defaultVariant(quad9)?.id);
    expect(findVariant(catalog, "quad9", "ecs")?.variant.id).toBe("ecs");
    expect(findVariant(catalog, "quad9", "nope")).toBeNull();
    expect(findVariant(catalog, "nope")).toBeNull();
    expect(findVariant(null, "quad9")).toBeNull();
    expect(defaultVariant({ ...quad9, variants: [] })).toBeUndefined();
  });

  it("gives the endpoints of every kind of selection", () => {
    expect(endpointsFor({ kind: "automatic" }, catalog, [])).toBeNull();
    expect(endpointsFor({ kind: "localResolver" }, catalog, [])).toEqual({
      ipv4: ["127.0.0.1"],
      ipv6: ["::1"],
    });
    const quad9 = endpointsFor(
      { kind: "provider", provider: "quad9", variant: "recommended" },
      catalog,
      [],
    );
    expect(quad9?.ipv4).toContain("9.9.9.9");
    expect(quad9?.doh).toBe("https://dns.quad9.net/dns-query");
    expect(endpointsFor({ kind: "provider", provider: "x", variant: "y" }, catalog, [])).toEqual({
      ipv4: [],
      ipv6: [],
    });
    expect(endpointsFor({ kind: "preset", id: lab.id }, catalog, [lab])).toEqual({
      ipv4: ["192.0.2.53"],
      ipv6: [],
      doh: lab.dohTemplate,
      dot: undefined,
    });
    expect(endpointsFor({ kind: "preset", id: "gone" }, catalog, [lab])).toEqual({
      ipv4: [],
      ipv6: [],
    });
    expect(
      endpointsFor(custom("192.0.2.1, 192.0.2.2", "", " https://d.example/q ", ""), null, []),
    ).toEqual({
      ipv4: ["192.0.2.1", "192.0.2.2"],
      ipv6: [],
      doh: "https://d.example/q",
      dot: undefined,
    });
  });

  it("checks typed-in servers", () => {
    expect(
      hasProblems(
        customProblems({ ipv4: "192.0.2.1", ipv6: "2001:db8::1", doh: "", dot: "dns.example" }),
      ),
    ).toBe(false);
    expect(customProblems({ ipv4: "", ipv6: "", doh: "", dot: "" })).toEqual({ empty: true });
    expect(customProblems({ ipv4: "x", ipv6: "9.9.9.9", doh: "http://x/", dot: "-x" })).toEqual({
      ipv4: { code: "invalid", value: "x" },
      ipv6: { code: "wrongFamily", value: "9.9.9.9" },
      doh: "bad",
      dot: "bad",
    });
    expect(
      customProblems({ ipv4: "192.0.2.1", ipv6: "", doh: "https://9.9.9.9/q", dot: "" }),
    ).toEqual({
      doh: "ipMismatch",
    });
  });

  it("knows when a selection can be applied", () => {
    expect(selectionUsable({ kind: "automatic" }, catalog, [])).toBe(true);
    expect(selectionUsable({ kind: "localResolver" }, catalog, [])).toBe(true);
    expect(
      selectionUsable({ kind: "provider", provider: "google", variant: "standard" }, catalog, []),
    ).toBe(true);
    expect(
      selectionUsable({ kind: "provider", provider: "google", variant: "x" }, catalog, []),
    ).toBe(false);
    expect(selectionUsable({ kind: "preset", id: lab.id }, catalog, [lab])).toBe(true);
    expect(selectionUsable({ kind: "preset", id: "x" }, catalog, [lab])).toBe(false);
    expect(selectionUsable(custom("192.0.2.1"), catalog, [])).toBe(true);
    expect(selectionUsable(custom(""), catalog, [])).toBe(false);
  });

  it("builds the request the backend expects", () => {
    expect(requestFor({ kind: "automatic" }, "doh", true)).toEqual({
      target: { kind: "automatic" },
      encryption: "none",
      allowFallback: false,
    });
    expect(requestFor({ kind: "localResolver" }, "preferred", false).target).toEqual({
      kind: "localResolver",
    });
    expect(
      requestFor({ kind: "provider", provider: "quad9", variant: "ecs" }, "doh", true),
    ).toEqual({
      target: { kind: "provider", provider: "quad9", variant: "ecs" },
      encryption: "doh",
      allowFallback: true,
    });
    expect(requestFor({ kind: "preset", id: lab.id }, "none", false).target).toEqual({
      kind: "preset",
      id: lab.id,
    });
    expect(
      requestFor(custom("192.0.2.1", "2001:db8::53", "", "dns.example"), "dot", false),
    ).toEqual({
      target: {
        kind: "custom",
        ipv4: ["192.0.2.1"],
        ipv6: ["2001:db8::53"],
        doh: undefined,
        dot: "dns.example",
      },
      encryption: "dot",
      allowFallback: false,
    });
  });
});

describe("encryption", () => {
  const quad9 = {
    ipv4: ["9.9.9.9"],
    ipv6: [],
    doh: "https://dns.quad9.net/dns-query",
    dot: "dns.quad9.net",
  };

  it("explains why a protocol cannot be chosen", () => {
    const windows = encryptionOptions(caps(true, false), quad9);
    expect(windows.map((o) => [o.choice, o.unavailable])).toEqual([
      ["preferred", null],
      ["none", null],
      ["doh", null],
      ["dot", { kind: "reason", code: "DOT_UNSUPPORTED_WINDOWS" }],
    ]);
    const plain = encryptionOptions(caps(true, true), { ipv4: ["192.0.2.1"], ipv6: [] });
    expect(plain[2]!.unavailable).toEqual({ kind: "noEndpoint", mode: "doh" });
    expect(plain[3]!.unavailable).toEqual({ kind: "noEndpoint", mode: "dot" });
    const auto = encryptionOptions(caps(true, true), null);
    expect(auto.filter((o) => o.unavailable).map((o) => o.choice)).toEqual([
      "preferred",
      "doh",
      "dot",
    ]);
    const unknown = encryptionOptions(undefined, quad9);
    expect(unknown[2]!.unavailable).toEqual({ kind: "reason", code: "UNSUPPORTED_PLATFORM" });
  });

  it("resolves the preferred choice like the backend", () => {
    expect(resolvedEncryption("preferred", caps(true, true), quad9)).toBe("doh");
    expect(resolvedEncryption("preferred", caps(false, true), quad9)).toBe("dot");
    expect(resolvedEncryption("preferred", caps(false, false), quad9)).toBe("none");
    expect(
      resolvedEncryption("preferred", caps(true, true), { ipv4: ["192.0.2.1"], ipv6: [] }),
    ).toBe("none");
    expect(resolvedEncryption("dot", caps(true, true), quad9)).toBe("dot");
    expect(resolvedEncryption("doh", caps(true, true), null)).toBe("none");
    expect(resolvedEncryption("none", caps(true, true), quad9)).toBe("none");
  });

  it("keeps a choice while it is possible", () => {
    const options = encryptionOptions(caps(true, false), quad9);
    expect(keepChoice("doh", options)).toBe("doh");
    expect(keepChoice("dot", options)).toBe("preferred");
    expect(keepChoice("doh", encryptionOptions(caps(true, true), null))).toBe("none");
  });

  it("puts reasons into words", () => {
    expect(unavailableText({ kind: "reason", code: "DOT_UNSUPPORTED_WINDOWS" })).toBe(
      i18n.t("dns.reason.DOT_UNSUPPORTED_WINDOWS"),
    );
    expect(unavailableText({ kind: "noEndpoint", mode: "dot" })).toContain("DNS over TLS");
    expect(unavailableText({ kind: "needsServers" })).toBe(i18n.t("dns.encryption.needsServers"));
    expect(reasonText("SOMETHING_NEW")).toBe(i18n.t("dns.reason.UNSUPPORTED_PLATFORM"));
    expect(reasonText(undefined)).toBe(i18n.t("dns.reason.UNSUPPORTED_PLATFORM"));
    expect(modeName("doh")).toBe("DNS over HTTPS");
    expect(modeName("none")).toBe(i18n.t("common.none"));
  });
});

describe("text", () => {
  it("summarizes configurations", () => {
    expect(summaryText(undefined)).toBe("—");
    expect(
      summaryText({ ipv4: { mode: "automatic" }, ipv6: { mode: "automatic" }, encryption: "none" }),
    ).toBe("Automatic");
    expect(
      summaryText({
        ipv4: { mode: "static", servers: ["9.9.9.9", "149.112.112.112"] },
        ipv6: { mode: "automatic" },
        encryption: "doh",
      }),
    ).toBe("IPv4 9.9.9.9, 149.112.112.112 · IPv6 automatic · DoH");
    expect(
      summaryText({
        ipv4: { mode: "automatic" },
        ipv6: { mode: "static", servers: ["::1"] },
        encryption: "dot",
      }),
    ).toBe("IPv4 automatic · IPv6 ::1 · DoT");
  });

  it("names configurations", () => {
    expect(labelText({ kind: "automatic" }, catalog)).toBe("Automatic");
    expect(labelText({ kind: "localResolver" }, catalog)).toBe("Local resolver");
    expect(labelText({ kind: "provider", provider: "quad9", variant: "ecs" }, catalog)).toMatch(
      /^Quad9.* · /,
    );
    expect(labelText({ kind: "provider", provider: "gone", variant: "x" }, catalog)).toBe("gone");
    expect(labelText({ kind: "preset", name: "Lab" }, catalog)).toBe("Preset “Lab”");
    expect(labelText({ kind: "custom" }, catalog)).toBe("Custom servers");
    expect(labelText(undefined, null)).toBe("Custom servers");
  });

  it("names selections the way the history will", () => {
    expect(labelForSelection({ kind: "provider", provider: "quad9", variant: "ecs" }, [])).toEqual({
      kind: "provider",
      provider: "quad9",
      variant: "ecs",
    });
    expect(labelForSelection({ kind: "preset", id: lab.id }, [lab])).toEqual({
      kind: "preset",
      name: "Lab",
    });
    expect(labelForSelection({ kind: "preset", id: "gone" }, [lab])).toEqual({ kind: "custom" });
    expect(
      labelForSelection({ kind: "custom", ipv4: "192.0.2.1", ipv6: "", doh: "", dot: "" }, []),
    ).toEqual({ kind: "custom" });
    expect(labelForSelection({ kind: "localResolver" }, [])).toEqual({ kind: "localResolver" });
    expect(labelForSelection({ kind: "automatic" }, [])).toEqual({ kind: "automatic" });
  });

  it("maps labels back to selections", () => {
    expect(selectionForLabel({ kind: "provider", provider: "quad9", variant: "ecs" }, [])).toEqual({
      kind: "provider",
      provider: "quad9",
      variant: "ecs",
    });
    expect(selectionForLabel({ kind: "localResolver" }, [])).toEqual({ kind: "localResolver" });
    expect(selectionForLabel({ kind: "automatic" }, [])).toEqual({ kind: "automatic" });
    expect(selectionForLabel({ kind: "preset", name: "Lab" }, [lab])).toEqual({
      kind: "preset",
      id: lab.id,
    });
    expect(selectionForLabel({ kind: "preset", name: "Other" }, [lab])).toBeNull();
    expect(selectionForLabel({ kind: "custom" }, [])).toBeNull();
    expect(selectionForLabel(undefined, [])).toBeNull();
  });
});

describe("latency test", () => {
  const answered = (medianUs: number): ProbeOutcome => ({
    kind: "answered",
    stats: { medianUs, minUs: medianUs - 100, maxUs: medianUs + 100 },
    answered: 3,
    attempts: 3,
    rcode: 0,
  });

  it("lists the default variant's servers of every provider", () => {
    const v4 = latencyRows(catalog, false);
    const both = latencyRows(catalog, true);
    expect(v4.length).toBeGreaterThanOrEqual(catalog.providers.length);
    expect(both.length).toBeGreaterThan(v4.length);
    expect(v4.every((r) => parseIpv4(r.addr))).toBe(true);
    expect(new Set(both.map((r) => r.key)).size).toBe(both.length);
    expect(latencyRows(null, true)).toEqual([]);
    expect(latencyRows({ providers: [{ ...catalog.providers[0]!, variants: [] }] }, true)).toEqual(
      [],
    );
  });

  it("sorts the fastest answers first", () => {
    const row = (name: string, outcome: ProbeOutcome | null): LatencyRow => ({
      key: name,
      providerId: name,
      variantId: "v",
      name,
      addr: "192.0.2.1",
      outcome,
    });
    const sorted = sortLatency([
      row("pending", null),
      row("slow", answered(50_000)),
      row("gone", { kind: "unreachable", message: "x" }),
      row("fast", answered(9_000)),
      row("refused", { kind: "refused", rcode: 5 }),
      row("stopped", { kind: "cancelled" }),
      row("silent", { kind: "timeout" }),
      row("also fast", answered(9_000)),
    ]);
    expect(sorted.map((r) => r.name)).toEqual([
      "also fast",
      "fast",
      "slow",
      "refused",
      "silent",
      "gone",
      "stopped",
      "pending",
    ]);
  });

  it("describes outcomes in the UI language", () => {
    expect(formatMs(12_345, "en")).toBe("12.3 ms");
    expect(formatMs(12_345, "vi")).toBe("12,3 ms");
    expect(outcomeText(answered(8_000), "en")).toBe("8.0 ms");
    expect(outcomeText({ kind: "refused", rcode: 5 }, "en")).toBe("Refused (code 5)");
    expect(outcomeText({ kind: "timeout" }, "en")).toBe(i18n.t("dns.latency.timeout"));
    expect(outcomeText({ kind: "unreachable", message: "x" }, "en")).toBe(
      i18n.t("dns.latency.unreachable"),
    );
    expect(outcomeText({ kind: "cancelled" }, "en")).toBe(i18n.t("dns.latency.cancelled"));
    expect(outcomeText(null, "en")).toBe(i18n.t("dns.latency.pending"));
  });
});

describe("presets", () => {
  const draft = {
    name: " Office ",
    ipv4: "192.0.2.10",
    ipv6: "",
    doh: "",
    dot: " dns.office.example ",
  };

  it("validates names, servers and the limit", () => {
    expect(presetProblems(draft, [lab])).toEqual({});
    expect(presetProblems({ ...draft, name: "  " }, []).name).toBe("missing");
    expect(presetProblems({ ...draft, name: "x".repeat(41) }, []).name).toBe("tooLong");
    expect(presetProblems({ ...draft, name: "lab" }, [lab]).name).toBe("taken");
    expect(presetProblems({ ...draft, name: "lab" }, [lab], lab.id).name).toBeUndefined();
    const full = Array.from({ length: 20 }, (_, i) => ({ ...lab, id: String(i), name: `P${i}` }));
    expect(presetProblems(draft, full).limit).toBe(true);
    expect(presetProblems(draft, full, "3").limit).toBeUndefined();
    expect(presetProblems({ ...draft, ipv4: "" }, []).empty).toBe(true);
  });

  it("creates, replaces and removes presets", () => {
    const id = newPresetId();
    expect(id).toMatch(/^[0-9a-f]{16}$/);
    expect(newPresetId()).not.toBe(id);
    const office = presetFromDraft(draft, id);
    expect(office).toEqual({
      id,
      name: "Office",
      ipv4: ["192.0.2.10"],
      ipv6: [],
      dotHost: "dns.office.example",
    });
    expect(
      presetFromDraft({ ...draft, doh: "https://dns.office.example/q", dot: "" }, id).dohTemplate,
    ).toBe("https://dns.office.example/q");
    const list = upsertPreset([lab], office);
    expect(list.map((p) => p.name)).toEqual(["Lab", "Office"]);
    const renamed = upsertPreset(list, { ...office, name: "HQ" });
    expect(renamed.map((p) => p.name)).toEqual(["Lab", "HQ"]);
    expect(removePreset(renamed, lab.id).map((p) => p.name)).toEqual(["HQ"]);
  });
});

describe("results", () => {
  const report = (extra: Partial<DnsReport>): DnsReport => ({
    method: "mock",
    status: "applied",
    effective: [],
    cacheFlushed: true,
    ...extra,
  });
  const result = (r: DnsReport | null, extra: Partial<DnsOpResult> = {}): DnsOpResult => ({
    adapterId: "a1",
    adapterName: "Wi-Fi",
    entry: null,
    report: r,
    error: null,
    success: r?.status === "applied",
    ...extra,
  });

  it("describes every outcome", () => {
    expect(describeDnsOutcome(result(report({})), false)).toEqual({
      kind: "success",
      message: "Wi-Fi uses the new DNS settings.",
      notes: [],
    });
    expect(describeDnsOutcome(result(report({})), true).message).toBe(
      "Wi-Fi is back to its original DNS settings.",
    );
    const skipped = describeDnsOutcome(
      result(report({ warnings: ["DNS_IPV6_SKIPPED", "ODD"] })),
      false,
    );
    expect(skipped.kind).toBe("warning");
    expect(skipped.notes).toEqual([i18n.t("dns.code.DNS_IPV6_SKIPPED"), "ODD"]);
    expect(
      describeDnsOutcome(result(report({ detailCode: "NOTHING_TO_RESTORE" })), true).kind,
    ).toBe("info");
    expect(
      describeDnsOutcome(
        result(report({ status: "unverified", detailCode: "DNS_NOT_ACTIVE" })),
        false,
      ).message,
    ).toBe(i18n.t("dns.result.notActive"));
    expect(describeDnsOutcome(result(report({ status: "unverified" })), false).message).toBe(
      i18n.t("dns.result.unverified"),
    );
    expect(describeDnsOutcome(result(report({ status: "rolledBack" })), false)).toMatchObject({
      kind: "error",
      message: i18n.t("dns.result.rolledBack"),
    });
    expect(
      describeDnsOutcome(result(report({ status: "failed", detailCode: "DNS_NOT_APPLIED" })), false)
        .kind,
    ).toBe("error");
    expect(
      describeDnsOutcome(result(null, { error: { code: "NOT_ELEVATED", message: "x" } }), false)
        .message,
    ).toBe(i18n.t("error.NOT_ELEVATED"));
    expect(noteText("DNS_CACHE_NOT_FLUSHED")).toBe(i18n.t("dns.code.DNS_CACHE_NOT_FLUSHED"));
  });

  it("sums up a change on every adapter", () => {
    const ok = result(report({}));
    const bad = result(report({ status: "rolledBack" }));
    expect(describeAll([ok, ok])).toEqual({ kind: "success", message: "2 of 2 adapters changed." });
    expect(describeAll([ok, bad]).kind).toBe("warning");
    expect(describeAll([bad]).kind).toBe("error");
  });
});
