// DNS page logic, kept out of the Svelte files so that it is tested: address and template checks
// that mirror crates/randommac-core/src/dns/model.rs, the endpoints and request of a selection,
// which encryption is available and why not, summaries, the latency table and the presets.
import { en } from "./i18n/en";
import { i18n, type MessageKey } from "./i18n/index.svelte";
import type { ToastKind } from "./stores/app.svelte";
import type {
  CustomDnsPreset,
  DnsCapabilities,
  DnsCatalog,
  DnsEndpoints,
  DnsLabel,
  DnsOpResult,
  DnsProvider,
  DnsRequest,
  DnsSummary,
  DnsVariant,
  EncryptionChoice,
  EncryptionMode,
  FamilyDns,
  ProbeOutcome,
} from "./types";

export const MAX_SERVERS_PER_FAMILY = 2;
export const MAX_PRESETS = 20;
export const MAX_PRESET_NAME = 40;
const MAX_TEMPLATE_LEN = 512;
const MAX_HOST_LEN = 253;
export const LOCAL_RESOLVER: DnsEndpoints = { ipv4: ["127.0.0.1"], ipv6: ["::1"] };

// --- Addresses -------------------------------------------------------------------------------

/** The four octets of an IPv4 address in dotted form (no leading zeros, like Rust's parser). */
export function parseIpv4(text: string): number[] | null {
  const parts = text.split(".");
  if (parts.length !== 4) return null;
  const octets: number[] = [];
  for (const part of parts) {
    if (!/^\d{1,3}$/.test(part) || (part.length > 1 && part.startsWith("0"))) return null;
    const n = Number(part);
    if (n > 255) return null;
    octets.push(n);
  }
  return octets;
}

function hexGroups(part: string, last: boolean): number[] | null {
  if (part === "") return [];
  const items = part.split(":");
  const groups: number[] = [];
  for (const [i, item] of items.entries()) {
    if (last && i === items.length - 1 && item.includes(".")) {
      const v4 = parseIpv4(item);
      if (!v4) return null;
      groups.push((v4[0]! << 8) | v4[1]!, (v4[2]! << 8) | v4[3]!);
    } else if (/^[0-9a-f]{1,4}$/i.test(item)) {
      groups.push(Number.parseInt(item, 16));
    } else {
      return null;
    }
  }
  return groups;
}

/** The eight 16-bit groups of an IPv6 address (with `::` and a trailing IPv4 part, no zone). */
export function parseIpv6(text: string): number[] | null {
  if (!/^[0-9a-f:.]+$/i.test(text)) return null;
  const halves = text.split("::");
  if (halves.length > 2) return null;
  if (halves.length === 1) {
    const all = hexGroups(text, true);
    return all?.length === 8 ? all : null;
  }
  const head = hexGroups(halves[0]!, false);
  const tail = hexGroups(halves[1]!, true);
  if (!head || !tail) return null;
  const missing = 8 - head.length - tail.length;
  if (missing < 1) return null;
  return [...head, ...new Array<number>(missing).fill(0), ...tail];
}

export type Family = 4 | 6;

/** A canonical form for comparing addresses ("2620:FE::FE" equals "2620:fe:0::fe"). */
export function ipKey(text: string): string | null {
  const v4 = parseIpv4(text);
  if (v4) return `4:${v4.join(".")}`;
  const v6 = parseIpv6(text);
  return v6 ? `6:${v6.join(":")}` : null;
}

/** True for addresses that cannot be DNS servers (as `check_v4` / `check_v6` in Rust). */
export function isUnusable(text: string): boolean {
  const v4 = parseIpv4(text);
  if (v4) {
    const [a] = v4 as [number];
    const all = (n: number) => v4.every((o) => o === n);
    return all(0) || all(255) || (a >= 224 && a <= 239) || a === 0;
  }
  const v6 = parseIpv6(text);
  if (!v6) return false;
  const [g0] = v6 as [number];
  const unspecified = v6.every((g) => g === 0);
  const multicast = g0 >= 0xff00;
  const linkLocal = (g0 & 0xffc0) === 0xfe80;
  const mapped = v6.slice(0, 5).every((g) => g === 0) && v6[5] === 0xffff;
  return unspecified || multicast || linkLocal || mapped;
}

/** The entries of a server field: separated by commas, semicolons or white space. */
export function splitServers(text: string): string[] {
  return text
    .split(/[\s,;]+/)
    .map((s) => s.trim())
    .filter((s) => s !== "");
}

export type ServerProblem =
  | { code: "invalid" | "unusable" | "duplicate" | "wrongFamily"; value: string }
  | { code: "tooMany" };

/** The first problem of a server list for one address family, or null. */
export function serverProblem(list: string[], family: Family): ServerProblem | null {
  if (list.length > MAX_SERVERS_PER_FAMILY) return { code: "tooMany" };
  const seen = new Set<string>();
  for (const value of list) {
    const own = family === 4 ? parseIpv4(value) : parseIpv6(value);
    if (!own) {
      const other = family === 4 ? parseIpv6(value) : parseIpv4(value);
      return { code: other ? "wrongFamily" : "invalid", value };
    }
    if (isUnusable(value)) return { code: "unusable", value };
    const key = ipKey(value)!;
    if (seen.has(key)) return { code: "duplicate", value };
    seen.add(key);
  }
  return null;
}

// --- Host names and DoH templates --------------------------------------------------------------

/** A DNS host name: labels of letters, digits and hyphens, not an IP address (as in Rust). */
export function isHostName(host: string): boolean {
  if (host === "" || host.length > MAX_HOST_LEN || ipKey(host)) return false;
  return host
    .split(".")
    .every(
      (label) =>
        label.length > 0 &&
        label.length <= 63 &&
        !label.startsWith("-") &&
        !label.endsWith("-") &&
        /^[a-z0-9-]+$/i.test(label),
    );
}

function splitAuthority(authority: string): { host: string; port: string | null } | null {
  if (authority.startsWith("[")) {
    const end = authority.indexOf("]");
    if (end < 0) return null;
    const after = authority.slice(end + 1);
    if (after !== "" && !after.startsWith(":")) return null;
    return { host: authority.slice(1, end), port: after === "" ? null : after.slice(1) };
  }
  const colon = authority.indexOf(":");
  return colon < 0
    ? { host: authority, port: null }
    : { host: authority.slice(0, colon), port: authority.slice(colon + 1) };
}

/**
 * Checks a DNS-over-HTTPS template: `https://host[:port]/path`, where an IP address as the host
 * must be the address of every server (Windows refuses a template for another address).
 */
export function dohTemplateProblem(
  template: string,
  servers: string[],
): "bad" | "ipMismatch" | null {
  // Printable ASCII only, without the characters URLs may not contain.
  if (template.length > MAX_TEMPLATE_LEN || /[^\x21-\x7e]|["<>\\^`|]/.test(template)) return "bad";
  if (!template.startsWith("https://")) return "bad";
  const rest = template.slice("https://".length);
  const slash = rest.indexOf("/");
  if (slash < 0) return "bad";
  const authority = rest.slice(0, slash);
  if (rest.length - slash < 2 || authority.includes("@")) return "bad";
  const parts = splitAuthority(authority);
  if (!parts) return "bad";
  if (parts.port !== null) {
    const port = /^\d{1,5}$/.test(parts.port) ? Number(parts.port) : 0;
    if (port < 1 || port > 65535) return "bad";
  }
  const hostKey = ipKey(parts.host);
  if (hostKey) {
    const keys = servers.map(ipKey);
    return keys.length > 0 && keys.every((k) => k === hostKey) ? null : "ipMismatch";
  }
  return isHostName(parts.host) ? null : "bad";
}

// --- Selections ----------------------------------------------------------------------------------

/** What the DNS page has selected; the custom fields are kept as typed. */
export type Selection =
  | { kind: "automatic" }
  | { kind: "localResolver" }
  | { kind: "provider"; provider: string; variant: string }
  | { kind: "preset"; id: string }
  | { kind: "custom"; ipv4: string; ipv6: string; doh: string; dot: string };

export function defaultVariant(provider: DnsProvider): DnsVariant | undefined {
  return provider.variants.find((v) => v.default) ?? provider.variants[0];
}

export function findVariant(
  catalog: DnsCatalog | null,
  providerId: string,
  variantId?: string,
): { provider: DnsProvider; variant: DnsVariant } | null {
  const provider = catalog?.providers.find((p) => p.id === providerId);
  if (!provider) return null;
  const variant = variantId
    ? provider.variants.find((v) => v.id === variantId)
    : defaultVariant(provider);
  return variant ? { provider, variant } : null;
}

function presetEndpoints(p: CustomDnsPreset): DnsEndpoints {
  return { ipv4: p.ipv4, ipv6: p.ipv6, doh: p.dohTemplate, dot: p.dotHost };
}

function optional(text: string): string | undefined {
  const value = text.trim();
  return value === "" ? undefined : value;
}

/** The servers and encryption endpoints of a selection; null for "automatic". */
export function endpointsFor(
  sel: Selection,
  catalog: DnsCatalog | null,
  presets: CustomDnsPreset[],
): DnsEndpoints | null {
  switch (sel.kind) {
    case "automatic":
      return null;
    case "localResolver":
      return LOCAL_RESOLVER;
    case "provider": {
      const found = findVariant(catalog, sel.provider, sel.variant);
      if (!found) return { ipv4: [], ipv6: [] };
      const { ipv4, ipv6, doh, dot } = found.variant;
      return { ipv4, ipv6, doh, dot };
    }
    case "preset": {
      const preset = presets.find((p) => p.id === sel.id);
      return preset ? presetEndpoints(preset) : { ipv4: [], ipv6: [] };
    }
    case "custom":
      return {
        ipv4: splitServers(sel.ipv4),
        ipv6: splitServers(sel.ipv6),
        doh: optional(sel.doh),
        dot: optional(sel.dot),
      };
  }
}

export interface CustomProblems {
  ipv4?: ServerProblem;
  ipv6?: ServerProblem;
  doh?: "bad" | "ipMismatch";
  dot?: "bad";
  empty?: true;
}

/** Problems of typed-in servers, template and host name; empty when they can be applied. */
export function customProblems(fields: {
  ipv4: string;
  ipv6: string;
  doh: string;
  dot: string;
}): CustomProblems {
  const problems: CustomProblems = {};
  const ipv4 = splitServers(fields.ipv4);
  const ipv6 = splitServers(fields.ipv6);
  const p4 = serverProblem(ipv4, 4);
  const p6 = serverProblem(ipv6, 6);
  if (p4) problems.ipv4 = p4;
  if (p6) problems.ipv6 = p6;
  if (ipv4.length === 0 && ipv6.length === 0) problems.empty = true;
  const doh = optional(fields.doh);
  if (doh) {
    const p = dohTemplateProblem(doh, [...ipv4, ...ipv6]);
    if (p) problems.doh = p;
  }
  const dot = optional(fields.dot);
  if (dot && !isHostName(dot)) problems.dot = "bad";
  return problems;
}

export function hasProblems(p: CustomProblems): boolean {
  return Object.keys(p).length > 0;
}

/** Whether the selection can be applied as it is. */
export function selectionUsable(
  sel: Selection,
  catalog: DnsCatalog | null,
  presets: CustomDnsPreset[],
): boolean {
  switch (sel.kind) {
    case "custom":
      return !hasProblems(customProblems(sel));
    case "provider":
      return findVariant(catalog, sel.provider, sel.variant) !== null;
    case "preset":
      return presets.some((p) => p.id === sel.id);
    default:
      return true;
  }
}

/** The request the backend gets for a selection. */
export function requestFor(
  sel: Selection,
  encryption: EncryptionChoice,
  allowFallback: boolean,
): DnsRequest {
  switch (sel.kind) {
    case "automatic":
      return { target: { kind: "automatic" }, encryption: "none", allowFallback: false };
    case "localResolver":
      return { target: { kind: "localResolver" }, encryption, allowFallback };
    case "provider":
      return {
        target: { kind: "provider", provider: sel.provider, variant: sel.variant },
        encryption,
        allowFallback,
      };
    case "preset":
      return { target: { kind: "preset", id: sel.id }, encryption, allowFallback };
    case "custom": {
      const e = endpointsFor(sel, null, [])!;
      return { target: { kind: "custom", ...e }, encryption, allowFallback };
    }
  }
}

// --- Encryption ----------------------------------------------------------------------------------

export type Unavailable =
  | { kind: "reason"; code: string }
  | { kind: "noEndpoint"; mode: "doh" | "dot" }
  | { kind: "needsServers" };

export interface EncryptionOption {
  choice: EncryptionChoice;
  /** Why it cannot be chosen; null when it can. */
  unavailable: Unavailable | null;
}

/** The encryption choices for a selection on an adapter, each with why it is unavailable. */
export function encryptionOptions(
  caps: DnsCapabilities | undefined,
  endpoints: DnsEndpoints | null,
): EncryptionOption[] {
  const protocol = (mode: "doh" | "dot"): Unavailable | null => {
    if (!endpoints) return { kind: "needsServers" };
    const support = caps?.[mode];
    if (!support?.available)
      return { kind: "reason", code: support?.reason ?? "UNSUPPORTED_PLATFORM" };
    return endpoints[mode] ? null : { kind: "noEndpoint", mode };
  };
  return [
    { choice: "preferred", unavailable: endpoints ? null : { kind: "needsServers" } },
    { choice: "none", unavailable: null },
    { choice: "doh", unavailable: protocol("doh") },
    { choice: "dot", unavailable: protocol("dot") },
  ];
}

/** What a choice ends up as ("preferred": DoH, else DoT, else none — as in Rust). */
export function resolvedEncryption(
  choice: EncryptionChoice,
  caps: DnsCapabilities | undefined,
  endpoints: DnsEndpoints | null,
): EncryptionMode {
  if (!endpoints || choice === "none") return "none";
  if (choice === "doh" || choice === "dot") return choice;
  if (caps?.doh.available && endpoints.doh) return "doh";
  if (caps?.dot.available && endpoints.dot) return "dot";
  return "none";
}

/** The choice to keep when the selection or adapter changes: the same one if still possible. */
export function keepChoice(
  current: EncryptionChoice,
  options: EncryptionOption[],
): EncryptionChoice {
  const option = options.find((o) => o.choice === current);
  return option && !option.unavailable
    ? current
    : options.some((o) => o.choice === "preferred" && !o.unavailable)
      ? "preferred"
      : "none";
}

export function unavailableText(u: Unavailable): string {
  switch (u.kind) {
    case "reason":
      return reasonText(u.code);
    case "noEndpoint":
      return i18n.t("dns.noEndpoint", { mode: modeName(u.mode) });
    case "needsServers":
      return i18n.t("dns.encryption.needsServers");
  }
}

export function reasonText(code: string | undefined): string {
  const key = `dns.reason.${code}`;
  return key in en ? i18n.t(key as MessageKey) : i18n.t("dns.reason.UNSUPPORTED_PLATFORM");
}

export function modeName(mode: EncryptionMode): string {
  return mode === "doh"
    ? "DNS over HTTPS"
    : mode === "dot"
      ? "DNS over TLS"
      : i18n.t("common.none");
}

// --- Text --------------------------------------------------------------------------------------

export function familyServers(f: FamilyDns | undefined): string[] | null {
  return f && f.mode === "static" ? f.servers : null;
}

/** "IPv4 9.9.9.9, 149.112.112.112 · IPv6 2620:fe::fe · DoH", or "Automatic". */
export function summaryText(s: DnsSummary | undefined | null): string {
  if (!s) return "—";
  const v4 = familyServers(s.ipv4);
  const v6 = familyServers(s.ipv6);
  if (!v4 && !v6) return i18n.t("dns.label.automatic");
  const auto = i18n.t("dns.automaticShort");
  const parts = [`IPv4 ${v4?.join(", ") ?? auto}`, `IPv6 ${v6?.join(", ") ?? auto}`];
  if (s.encryption === "doh") parts.push("DoH");
  if (s.encryption === "dot") parts.push("DoT");
  return parts.join(" · ");
}

export function variantText(variant: DnsVariant): string {
  return i18n.t(`dns.variant.${variant.label}` as MessageKey);
}

/** How a configuration is named: provider and variant, preset, local resolver… */
export function labelText(label: DnsLabel | undefined | null, catalog: DnsCatalog | null): string {
  switch (label?.kind) {
    case "automatic":
      return i18n.t("dns.label.automatic");
    case "localResolver":
      return i18n.t("dns.label.localResolver");
    case "provider": {
      const found = findVariant(catalog, label.provider, label.variant);
      if (!found) return label.provider;
      return `${found.provider.name} · ${variantText(found.variant)}`;
    }
    case "preset":
      return i18n.t("dns.label.preset", { name: label.name });
    default:
      return i18n.t("dns.label.custom");
  }
}

/** The label a selection will be recorded under. */
export function labelForSelection(sel: Selection, presets: CustomDnsPreset[]): DnsLabel {
  switch (sel.kind) {
    case "provider":
      return { kind: "provider", provider: sel.provider, variant: sel.variant };
    case "preset": {
      const preset = presets.find((p) => p.id === sel.id);
      return preset ? { kind: "preset", name: preset.name } : { kind: "custom" };
    }
    case "custom":
      return { kind: "custom" };
    default:
      return { kind: sel.kind };
  }
}

/** The selection that matches a label (to preselect what the adapter uses). */
export function selectionForLabel(
  label: DnsLabel | undefined | null,
  presets: CustomDnsPreset[],
): Selection | null {
  switch (label?.kind) {
    case "provider":
      return { kind: "provider", provider: label.provider, variant: label.variant };
    case "localResolver":
      return { kind: "localResolver" };
    case "automatic":
      return { kind: "automatic" };
    case "preset": {
      const preset = presets.find((p) => p.name === label.name);
      return preset ? { kind: "preset", id: preset.id } : null;
    }
    default:
      return null;
  }
}

// --- Latency test ------------------------------------------------------------------------------

export interface LatencyRow {
  key: string;
  providerId: string;
  variantId: string;
  name: string;
  addr: string;
  outcome: ProbeOutcome | null;
}

/** One row per server of every provider's default variant (IPv6 ones on request). */
export function latencyRows(catalog: DnsCatalog | null, includeIpv6: boolean): LatencyRow[] {
  const rows: LatencyRow[] = [];
  for (const provider of catalog?.providers ?? []) {
    const variant = defaultVariant(provider);
    if (!variant) continue;
    const servers = includeIpv6 ? [...variant.ipv4, ...variant.ipv6] : variant.ipv4;
    for (const addr of servers) {
      rows.push({
        key: `${provider.id}/${addr}`,
        providerId: provider.id,
        variantId: variant.id,
        name: provider.name,
        addr,
        outcome: null,
      });
    }
  }
  return rows;
}

function rank(o: ProbeOutcome | null): number {
  switch (o?.kind) {
    case "answered":
      return 0;
    case "refused":
      return 1;
    case "timeout":
      return 2;
    case "unreachable":
      return 3;
    case "cancelled":
      return 4;
    default:
      return 5;
  }
}

function median(o: ProbeOutcome | null): number {
  return o?.kind === "answered" ? o.stats.medianUs : Number.MAX_SAFE_INTEGER;
}

/** Fastest answers first, then servers that refused, did not answer or were not tested. */
export function sortLatency(rows: LatencyRow[]): LatencyRow[] {
  return [...rows].sort(
    (a, b) =>
      rank(a.outcome) - rank(b.outcome) ||
      median(a.outcome) - median(b.outcome) ||
      a.name.localeCompare(b.name) ||
      a.addr.localeCompare(b.addr),
  );
}

/** "12.3 ms" in the UI language. */
export function formatMs(us: number, locale: string): string {
  const ms = new Intl.NumberFormat(locale, { minimumFractionDigits: 1, maximumFractionDigits: 1 });
  return `${ms.format(us / 1000)} ms`;
}

export function outcomeText(o: ProbeOutcome | null, locale: string): string {
  switch (o?.kind) {
    case "answered":
      return formatMs(o.stats.medianUs, locale);
    case "refused":
      return i18n.t("dns.latency.refused", { rcode: o.rcode });
    case "timeout":
      return i18n.t("dns.latency.timeout");
    case "unreachable":
      return i18n.t("dns.latency.unreachable");
    case "cancelled":
      return i18n.t("dns.latency.cancelled");
    default:
      return i18n.t("dns.latency.pending");
  }
}

// --- Presets -----------------------------------------------------------------------------------

export interface PresetDraft {
  name: string;
  ipv4: string;
  ipv6: string;
  doh: string;
  dot: string;
}

export interface PresetProblems extends CustomProblems {
  name?: "missing" | "tooLong" | "taken";
  limit?: true;
}

export function presetProblems(
  draft: PresetDraft,
  existing: CustomDnsPreset[],
  editingId: string | null = null,
): PresetProblems {
  const problems: PresetProblems = customProblems(draft);
  const name = draft.name.trim();
  if (name === "") problems.name = "missing";
  else if (name.length > MAX_PRESET_NAME) problems.name = "tooLong";
  else if (
    existing.some((p) => p.id !== editingId && p.name.toLowerCase() === name.toLowerCase())
  ) {
    problems.name = "taken";
  }
  if (editingId === null && existing.length >= MAX_PRESETS) problems.limit = true;
  return problems;
}

/** A new id in the backend's format (16 hex digits). */
export function newPresetId(): string {
  const bytes = crypto.getRandomValues(new Uint8Array(8));
  return [...bytes].map((b) => b.toString(16).padStart(2, "0")).join("");
}

export function presetFromDraft(draft: PresetDraft, id: string): CustomDnsPreset {
  const preset: CustomDnsPreset = {
    id,
    name: draft.name.trim(),
    ipv4: splitServers(draft.ipv4),
    ipv6: splitServers(draft.ipv6),
  };
  const doh = optional(draft.doh);
  const dot = optional(draft.dot);
  if (doh) preset.dohTemplate = doh;
  if (dot) preset.dotHost = dot;
  return preset;
}

/** Replaces the preset with the same id, or adds it at the end. */
export function upsertPreset(list: CustomDnsPreset[], preset: CustomDnsPreset): CustomDnsPreset[] {
  return list.some((p) => p.id === preset.id)
    ? list.map((p) => (p.id === preset.id ? preset : p))
    : [...list, preset];
}

export function removePreset(list: CustomDnsPreset[], id: string): CustomDnsPreset[] {
  return list.filter((p) => p.id !== id);
}

// --- Results -----------------------------------------------------------------------------------

export function noteText(code: string): string {
  const key = `dns.code.${code}`;
  return key in en ? i18n.t(key as MessageKey) : code;
}

/** The message for a DNS change and the notes (warnings) that come with it. */
export function describeDnsOutcome(
  result: DnsOpResult,
  restore: boolean,
): { kind: ToastKind; message: string; notes: string[] } {
  const report = result.report;
  if (!report) {
    const code = result.error?.code ?? result.entry?.errorCode;
    return { kind: "error", message: i18n.error(code), notes: [] };
  }
  const notes = (report.warnings ?? []).map(noteText);
  const adapter = result.adapterName;
  switch (report.status) {
    case "applied":
      if (report.detailCode === "NOTHING_TO_RESTORE") {
        return { kind: "info", message: i18n.t("dns.result.nothingToRestore"), notes };
      }
      return {
        kind: notes.length > 0 ? "warning" : "success",
        message: restore
          ? i18n.t("dns.result.restored", { adapter })
          : i18n.t("dns.result.applied", { adapter }),
        notes,
      };
    case "unverified":
      return {
        kind: "warning",
        message:
          report.detailCode === "DNS_NOT_ACTIVE"
            ? i18n.t("dns.result.notActive")
            : i18n.t("dns.result.unverified"),
        notes,
      };
    case "rolledBack":
      return { kind: "error", message: i18n.t("dns.result.rolledBack"), notes };
    default:
      return {
        kind: "error",
        message: i18n.error(result.entry?.errorCode ?? report.detailCode),
        notes,
      };
  }
}

/** The message after "apply to all connected adapters". */
export function describeAll(results: DnsOpResult[]): { kind: ToastKind; message: string } {
  const ok = results.filter((r) => r.success).length;
  const kind: ToastKind = ok === results.length ? "success" : ok === 0 ? "error" : "warning";
  return { kind, message: i18n.t("dns.result.all", { ok, total: results.length }) };
}
