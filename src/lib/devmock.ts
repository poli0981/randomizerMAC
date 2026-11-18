// Development-only: lets the UI run in a normal browser (`npm run dev`, then open
// http://localhost:1420) with fake backend data. Never included in production builds.
import { emit } from "@tauri-apps/api/event";
import { mockIPC } from "@tauri-apps/api/mocks";
import catalogJson from "../../crates/randommac-core/src/dns/catalog.json";
import { version } from "../../package.json";
import type {
  Adapter,
  DnsCatalog,
  DnsEndpoints,
  DnsLabel,
  DnsOpResult,
  DnsRequest,
  DnsSummary,
  DnsView,
  FamilyDns,
  HistoryEntry,
  LatencyResult,
  Settings,
} from "./types";

const now = Date.now();
const iso = (minutesAgo: number) => new Date(now - minutesAgo * 60_000).toISOString();

const adapters: Adapter[] = [
  {
    id: "PCI\\VEN_10EC&DEV_8125\\01",
    name: "Ethernet",
    description: "Realtek Gaming 2.5GbE Family Controller",
    interfaceName: "Ethernet",
    kind: "ethernet",
    isVirtual: false,
    state: "up",
    connected: true,
    currentMac: "30:56:0F:09:8F:E4",
    permanentMac: "30:56:0F:09:8F:E4",
    overrideValue: null,
    supportsChange: "yes",
    methods: ["windowsRegistry"],
    nmConnection: null,
    driver: null,
    ipv4: ["192.0.2.107"],
    ipv6: ["2001:db8::1c2b:3a4d:5e6f:7081"],
    vendor: "GIGABYTE",
    hardwareVendor: "GIGABYTE",
    originalMac: "30:56:0F:09:8F:E4",
    changed: false,
    restorable: false,
  },
  {
    id: "PCI\\VEN_8086&DEV_51F1\\02",
    name: "Wi-Fi",
    description: "Intel(R) Wi-Fi 6E AX211 160MHz",
    interfaceName: "Wi-Fi",
    kind: "wireless",
    isVirtual: false,
    state: "up",
    connected: true,
    currentMac: "02:3C:91:5A:7E:10",
    permanentMac: "00:1B:21:3A:4B:5C",
    overrideValue: "023C915A7E10",
    supportsChange: "yes",
    methods: ["windowsRegistry"],
    nmConnection: null,
    driver: null,
    ipv4: ["198.51.100.42"],
    ipv6: [],
    vendor: null,
    hardwareVendor: "Intel",
    originalMac: "00:1B:21:3A:4B:5C",
    changed: true,
    restorable: true,
  },
  {
    id: "USB\\VID_0B95&PID_1790\\03",
    name: "Ethernet 3",
    description: "ASIX AX88179 USB 3.0 to Gigabit Ethernet Adapter",
    interfaceName: "Ethernet 3",
    kind: "ethernet",
    isVirtual: false,
    state: "disabled",
    connected: false,
    currentMac: "00:0E:C6:11:22:33",
    permanentMac: "00:0E:C6:11:22:33",
    overrideValue: null,
    supportsChange: "unknown",
    methods: ["windowsRegistry"],
    nmConnection: null,
    driver: null,
    ipv4: [],
    ipv6: [],
    vendor: null,
    hardwareVendor: null,
    originalMac: "00:0E:C6:11:22:33",
    changed: false,
    restorable: false,
  },
];

const history: HistoryEntry[] = [
  {
    id: "h1",
    timestamp: iso(12),
    adapterId: adapters[1]!.id,
    adapterName: "Wi-Fi",
    action: "apply",
    source: "ui",
    status: "applied",
    previousMac: "00:1B:21:3A:4B:5C",
    requestedMac: "02:3C:91:5A:7E:10",
    newMac: "02:3C:91:5A:7E:10",
    method: "windowsRegistry",
  },
  {
    id: "h2",
    timestamp: iso(180),
    adapterId: adapters[0]!.id,
    adapterName: "Ethernet",
    action: "restore",
    source: "tray",
    status: "applied",
    previousMac: "06:AA:12:34:56:78",
    newMac: "30:56:0F:09:8F:E4",
    method: "windowsRegistry",
  },
  {
    id: "h3",
    timestamp: iso(60 * 26),
    adapterId: adapters[0]!.id,
    adapterName: "Ethernet",
    action: "apply",
    source: "schedule",
    status: "rolledBack",
    previousMac: "30:56:0F:09:8F:E4",
    requestedMac: "0A:11:22:33:44:55",
    errorCode: "DRIVER_REJECTED",
    method: "windowsRegistry",
  },
];

let settings: Settings = {
  schemaVersion: 3,
  language: "system",
  theme: "system",
  startWithSystem: true,
  startMinimized: false,
  closeToTray: true,
  showNotifications: true,
  confirmBeforeApply: false,
  autoCheckUpdates: true,
  generationMode: { kind: "random" },
  macFormat: { separator: "colon", uppercase: true },
  showVirtualAdapters: false,
  selectedAdapterId: adapters[1]!.id,
  automation: {
    changeOnStartup: true,
    startupAdapterIds: [adapters[1]!.id],
    intervalEnabled: false,
    intervalMinutes: 60,
    intervalAdapterIds: [],
    restoreOnExit: false,
  },
  linux: { method: "auto" },
  history: { maxEntries: 500, retentionDays: 90 },
  legacy: {},
  dns: {
    preferEncryption: true,
    allowFallback: false,
    presets: [{ id: "0a1b2c3d4e5f6071", name: "Home router", ipv4: ["192.0.2.1"], ipv6: [] }],
  },
};

// --- DNS ---------------------------------------------------------------------------------------

const catalog = catalogJson as DnsCatalog;
/** The demo Wi-Fi adapter starts on Quad9 with DNS over HTTPS (from the real catalog). */
const quad9 = catalog.providers.find((p) => p.id === "quad9")?.variants.find((v) => v.default);
/** The server "DHCP" hands out in the demo: a documentation address (RFC 5737). */
const DHCP_DNS = "192.0.2.1";

interface MockDns {
  summary: DnsSummary;
  doh?: string;
  label?: DnsLabel;
  /** Settings from before the first change (what "Restore original" puts back). */
  original?: { summary: DnsSummary; doh?: string; label?: DnsLabel };
}

const automatic: DnsSummary = {
  ipv4: { mode: "automatic" },
  ipv6: { mode: "automatic" },
  encryption: "none",
};

const family = (servers: string[]): FamilyDns =>
  servers.length > 0 ? { mode: "static", servers } : { mode: "automatic" };

const dnsState = new Map<string, MockDns>([
  [adapters[0]!.id, { summary: automatic, label: { kind: "automatic" } }],
  [
    adapters[1]!.id,
    {
      summary: {
        ipv4: family(quad9?.ipv4 ?? []),
        ipv6: family(quad9?.ipv6 ?? []),
        encryption: "doh",
      },
      doh: quad9?.doh,
      label: { kind: "provider", provider: "quad9", variant: quad9?.id ?? "recommended" },
      original: { summary: automatic, label: { kind: "automatic" } },
    },
  ],
]);

function findAdapter(adapterId: string): Adapter {
  const adapter = adapters.find((a) => a.id === adapterId);
  if (!adapter) throw { code: "ADAPTER_NOT_FOUND", message: "no such adapter" };
  return adapter;
}

function dnsView(adapterId: string): DnsView {
  const adapter = findAdapter(adapterId);
  const disabled = adapter.state === "disabled";
  const s = dnsState.get(adapterId) ?? { summary: automatic, label: { kind: "automatic" } };
  const static4 = s.summary.ipv4.mode === "static" ? s.summary.ipv4.servers : null;
  const static6 = s.summary.ipv6.mode === "static" ? s.summary.ipv6.servers : [];
  return {
    capabilities: {
      supported: true,
      canChange: !disabled,
      changeReason: disabled ? "ADAPTER_DISABLED" : undefined,
      ipv4: { available: true },
      ipv6: { available: true },
      doh: { available: true },
      dot: { available: false, reason: "DOT_UNSUPPORTED_WINDOWS" },
      method: "windowsIpHelper",
    },
    status: {
      configured: s.summary,
      dohTemplate: s.doh,
      fallback: s.doh ? false : undefined,
      effectiveIpv4: adapter.connected ? (static4 ?? [DHCP_DNS]) : [],
      effectiveIpv6: adapter.connected ? static6 : [],
      profileOverride: false,
    },
    restorable: s.original !== undefined,
    matched: s.label,
  };
}

/** The endpoints and label of a request, as the backend resolves them. */
function resolve(request: DnsRequest): { endpoints: DnsEndpoints | null; label: DnsLabel } {
  const target = request.target;
  switch (target.kind) {
    case "automatic":
      return { endpoints: null, label: { kind: "automatic" } };
    case "localResolver":
      return {
        endpoints: { ipv4: ["127.0.0.1"], ipv6: ["::1"] },
        label: { kind: "localResolver" },
      };
    case "provider": {
      const provider = catalog.providers.find((p) => p.id === target.provider);
      const variant =
        provider?.variants.find((v) => v.id === target.variant) ??
        provider?.variants.find((v) => v.default);
      if (!provider || !variant) throw { code: "INVALID_DNS", message: "unknown DNS provider" };
      return {
        endpoints: variant,
        label: { kind: "provider", provider: provider.id, variant: variant.id },
      };
    }
    case "preset": {
      const preset = settings.dns.presets.find((p) => p.id === target.id);
      if (!preset) throw { code: "UNKNOWN_DNS_PRESET", message: "unknown DNS preset" };
      return {
        endpoints: { ...preset, doh: preset.dohTemplate, dot: preset.dotHost },
        label: { kind: "preset", name: preset.name },
      };
    }
    default:
      return { endpoints: target, label: { kind: "custom" } };
  }
}

function dnsEntry(
  adapter: Adapter,
  action: "apply" | "restore",
  previous: DnsSummary,
  next: MockDns,
): HistoryEntry {
  const entry: HistoryEntry = {
    id: "d" + String(history.length + 1),
    timestamp: new Date().toISOString(),
    adapterId: adapter.id,
    adapterName: adapter.name,
    action,
    source: "ui",
    status: "applied",
    method: "windowsIpHelper",
    kind: "dns",
    dns: { previous, requested: next.summary, observed: next.summary, label: next.label },
  };
  history.unshift(entry);
  return entry;
}

function dnsResult(
  adapter: Adapter,
  entry: HistoryEntry | null,
  summary: DnsSummary,
  detailCode?: string,
): DnsOpResult {
  return {
    adapterId: adapter.id,
    adapterName: adapter.name,
    entry,
    report: {
      method: "windowsIpHelper",
      status: "applied",
      observed: summary,
      effective: [],
      cacheFlushed: true,
      detailCode,
    },
    error: null,
    success: true,
  };
}

function dnsApply(adapterId: string, request: DnsRequest): DnsOpResult {
  const adapter = findAdapter(adapterId);
  if (adapter.state === "disabled") {
    throw { code: "UNSUPPORTED", message: "the adapter is disabled" };
  }
  const { endpoints, label } = resolve(request);
  const before = dnsState.get(adapterId) ?? { summary: automatic };
  const encrypt = request.encryption === "doh" || request.encryption === "preferred";
  const doh = encrypt ? endpoints?.doh : undefined;
  const summary: DnsSummary = endpoints
    ? {
        ipv4: family(endpoints.ipv4),
        ipv6: family(endpoints.ipv6),
        encryption: doh ? "doh" : "none",
      }
    : automatic;
  const original = before.original ?? {
    summary: before.summary,
    doh: before.doh,
    label: before.label,
  };
  const next: MockDns = { summary, doh, label, original };
  dnsState.set(adapterId, next);
  return dnsResult(adapter, dnsEntry(adapter, "apply", before.summary, next), summary);
}

function dnsRestore(adapterId: string): DnsOpResult {
  const adapter = findAdapter(adapterId);
  const current = dnsState.get(adapterId) ?? { summary: automatic };
  if (!current.original) return dnsResult(adapter, null, current.summary, "NOTHING_TO_RESTORE");
  const next: MockDns = { ...current.original };
  dnsState.set(adapterId, next);
  return dnsResult(adapter, dnsEntry(adapter, "restore", current.summary, next), next.summary);
}

/** A stable, plausible round-trip time for an address (8–60 ms; 10.x addresses never answer). */
function fakeLatency(addr: string): LatencyResult {
  if (addr.startsWith("10.")) return { addr, outcome: { kind: "timeout" } };
  const seed = [...addr].reduce((n, c) => (n * 31 + (c.codePointAt(0) ?? 0)) % 9973, 7);
  const median = 8_000 + (seed % 52_000);
  return {
    addr,
    outcome: {
      kind: "answered",
      stats: { medianUs: median, minUs: median - 900, maxUs: median + 2_400 },
      answered: 3,
      attempts: 3,
      rcode: 0,
    },
  };
}

/** The events the real backend sends after a DNS change. */
function announce(results: DnsOpResult[]): DnsOpResult[] {
  void emit("history:changed");
  for (const r of results) void emit("dns:changed", r.adapterId);
  return results;
}

function randomMac() {
  const b = crypto.getRandomValues(new Uint8Array(6));
  b[0] = (b[0]! & 0xfc) | 0x02;
  return [...b].map((x) => x.toString(16).padStart(2, "0").toUpperCase()).join(":");
}

export function installDevMock() {
  mockIPC(
    (cmd, payload) => {
      const args = (payload ?? {}) as Record<string, unknown>;
      switch (cmd) {
        case "app_info":
          return {
            version,
            os: "windows",
            arch: "x86_64",
            osVersion: "Windows 11",
            elevated: true,
            readOnly: false,
            mock: true,
            packageKind: "nsis",
            trayAvailable: true,
            capabilities: {
              platform: "windows",
              elevated: true,
              canChange: true,
              networkManager: false,
              iproute2: false,
              pkexec: false,
            },
            dataDir: "C:\\Users\\you\\AppData\\Local\\io.github.poli0981.randommac",
            logDir: "C:\\Users\\you\\AppData\\Local\\io.github.poli0981.randommac\\logs",
            ouiSourceDate: "2026-06-25",
            autostartEnabled: true,
            startup: { v1Import: null, legacyTaskRemoved: false, recoveredFiles: [] },
            launchedMinimized: false,
          };
        case "settings_get":
          return settings;
        case "settings_update":
          settings = args.settings as Settings;
          return settings;
        case "list_adapters":
          return adapters;
        case "history_list":
          return history;
        case "dns_catalog":
          return catalog;
        case "dns_view":
          return dnsView(args.adapterId as string);
        case "dns_apply":
          return announce([dnsApply(args.adapterId as string, args.request as DnsRequest)])[0];
        case "dns_apply_all":
          return announce(
            adapters
              .filter((a) => a.connected && !a.isVirtual)
              .map((a) => dnsApply(a.id, args.request as DnsRequest)),
          );
        case "dns_restore":
          return announce([dnsRestore(args.adapterId as string)])[0];
        case "dns_latency_test":
          return new Promise((done) =>
            setTimeout(() => done((args.servers as string[]).map(fakeLatency)), 400),
          );
        case "blacklist_get":
          return [
            {
              id: "b1",
              pattern: "02:00:5E:*",
              note: "Lab network",
              createdAt: iso(5000),
            },
          ];
        case "list_vendors":
          return [
            { id: "apple", name: "Apple", imitable: true, ouiCount: 1533 },
            { id: "intel", name: "Intel", imitable: true, ouiCount: 677 },
            { id: "samsung", name: "Samsung", imitable: true, ouiCount: 891 },
          ];
        case "generate_mac":
          return { valid: true, error: null, mac: randomMac(), vendor: null, warnings: [] };
        case "inspect_mac":
          return { valid: true, error: null, mac: null, vendor: null, warnings: [] };
        case "logs_get":
          return args.after
            ? []
            : [
                {
                  seq: 1,
                  timestamp: iso(1),
                  level: "info",
                  target: "randommac_lib",
                  message: `RandomMac starting version=${version} os=windows elevated=true`,
                },
                {
                  seq: 2,
                  timestamp: iso(1),
                  level: "warn",
                  target: "randommac_lib::updater",
                  message: "update check failed: offline",
                },
              ];
        case "update_check":
          return {
            currentVersion: version,
            available: false,
            version: null,
            date: null,
            notes: null,
            checkedAt: new Date().toISOString(),
          };
        default:
          return null;
      }
    },
    { shouldMockEvents: true },
  );
}
