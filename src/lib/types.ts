// TypeScript mirrors of the Rust DTOs (serde camelCase). Keep in sync with
// crates/randommac-net/src/types.rs, crates/randommac-core and src-tauri/src/commands.rs.

export type AdapterKind = "ethernet" | "wireless" | "other";
export type AdapterState = "up" | "down" | "disabled" | "unknown";
export type ChangeMethod = "windowsRegistry" | "networkManager" | "iproute2" | "mock";
export type Support = "yes" | "unknown" | "no";
export type Stage = "preparing" | "writing" | "restarting" | "verifying" | "rollingBack";
export type ChangeStatus = "applied" | "unverified" | "rolledBack" | "rebootRequired" | "failed";
export type ChangeSource = "ui" | "tray" | "startup" | "schedule" | "exit" | "cli" | "importV1";
export type HistoryAction = "apply" | "restore";
export type HistoryKind = "mac" | "dns";
export type OpKind = "mac" | "dns";

export interface NmConnection {
  uuid: string;
  name: string;
  setting: string;
  clonedMac: string;
}

export interface Adapter {
  id: string;
  name: string;
  description: string;
  interfaceName: string;
  kind: AdapterKind;
  isVirtual: boolean;
  state: AdapterState;
  connected: boolean;
  currentMac: string | null;
  permanentMac: string | null;
  overrideValue: string | null;
  supportsChange: Support;
  methods: ChangeMethod[];
  nmConnection: NmConnection | null;
  driver: string | null;
  ipv4: string[];
  ipv6: string[];
  vendor: string | null;
  hardwareVendor: string | null;
  originalMac: string | null;
  changed: boolean;
  restorable: boolean;
}

export interface ChangeReport {
  method: ChangeMethod;
  status: ChangeStatus;
  previousMac: string | null;
  requestedMac: string | null;
  observedMac: string | null;
  detailCode: string | null;
  nmConnectionUuid: string | null;
  nmPreviousValue: string | null;
  temporary: boolean;
}

export interface HistoryEntry {
  id: string;
  timestamp: string;
  adapterId: string;
  adapterName: string;
  action: HistoryAction;
  source: ChangeSource;
  status: ChangeStatus;
  previousMac?: string;
  requestedMac?: string;
  newMac?: string;
  method?: string;
  errorCode?: string;
  error?: string;
  /** Absent for MAC entries (files written before 2.1 have no kind). */
  kind?: HistoryKind;
  dns?: DnsHistory;
}

export interface ApiError {
  code: string;
  message: string;
}

export interface OpResult {
  adapterId: string;
  adapterName: string;
  entry: HistoryEntry;
  report: ChangeReport | null;
  error: ApiError | null;
  success: boolean;
}

export interface MacInfo {
  valid: boolean;
  error: string | null;
  mac: string | null;
  vendor: string | null;
  warnings: string[];
}

export type GenerationMode =
  { kind: "random" } | { kind: "keepVendor" } | { kind: "vendor"; vendorId: string };

export type Separator = "colon" | "hyphen" | "dot" | "none";

export interface MacFormat {
  separator: Separator;
  uppercase: boolean;
}

export interface Settings {
  schemaVersion: number;
  language: "system" | "en" | "vi";
  theme: "system" | "light" | "dark";
  startWithSystem: boolean;
  startMinimized: boolean;
  closeToTray: boolean;
  showNotifications: boolean;
  confirmBeforeApply: boolean;
  autoCheckUpdates: boolean;
  lastUpdateCheck?: string;
  generationMode: GenerationMode;
  macFormat: MacFormat;
  showVirtualAdapters: boolean;
  selectedAdapterId?: string;
  automation: {
    changeOnStartup: boolean;
    startupAdapterIds: string[];
    intervalEnabled: boolean;
    intervalMinutes: number;
    intervalAdapterIds: string[];
    restoreOnExit: boolean;
  };
  linux: { method: "auto" | "networkManager" | "iproute2" };
  history: { maxEntries: number; retentionDays: number };
  legacy: { importedFromV1At?: string; legacyTaskRemovedAt?: string };
  dns: DnsSettings;
}

export interface BlacklistEntry {
  id: string;
  pattern: string;
  adapterId?: string;
  note?: string;
  createdAt: string;
}

export interface LogEntry {
  seq: number;
  timestamp: string;
  level: "error" | "warn" | "info" | "debug" | "trace";
  target: string;
  message: string;
}

export interface UpdateInfo {
  currentVersion: string;
  available: boolean;
  version: string | null;
  date: string | null;
  notes: string | null;
  checkedAt: string;
}

export interface Capabilities {
  platform: string;
  elevated: boolean;
  canChange: boolean;
  networkManager: boolean;
  iproute2: boolean;
  pkexec: boolean;
}

export interface V1ImportReport {
  settings: boolean;
  history: number;
  blacklist: number;
  autostart: boolean;
}

export interface StartupReport {
  v1Import: V1ImportReport | null;
  legacyTaskRemoved: boolean;
  recoveredFiles: string[];
}

export interface AppInfo {
  version: string;
  os: string;
  arch: string;
  osVersion: string;
  elevated: boolean;
  readOnly: boolean;
  mock: boolean;
  packageKind: string | null;
  trayAvailable: boolean;
  capabilities: Capabilities;
  dataDir: string;
  logDir: string;
  ouiSourceDate: string;
  autostartEnabled: boolean;
  startup: StartupReport;
  launchedMinimized: boolean;
}

export interface Vendor {
  id: string;
  name: string;
  imitable: boolean;
  ouiCount: number;
}

export type Page =
  "dashboard" | "dns" | "history" | "settings" | "logs" | "update" | "about" | "social";

// --- DNS (crates/randommac-core/src/dns, crates/randommac-net/src/dns_*.rs) ---

export type FamilyDns = { mode: "automatic" } | { mode: "static"; servers: string[] };
export type EncryptionMode = "none" | "doh" | "dot";
export type EncryptionChoice = "none" | "doh" | "dot" | "preferred";

export interface DnsSummary {
  ipv4: FamilyDns;
  ipv6: FamilyDns;
  encryption: EncryptionMode;
}

export interface DnsEndpoints {
  ipv4: string[];
  ipv6: string[];
  doh?: string;
  dot?: string;
}

export type DnsTarget =
  | { kind: "automatic" }
  | { kind: "localResolver" }
  | { kind: "provider"; provider: string; variant?: string }
  | { kind: "preset"; id: string }
  | ({ kind: "custom" } & DnsEndpoints);

export interface DnsRequest {
  target: DnsTarget;
  encryption: EncryptionChoice;
  allowFallback: boolean;
}

export type DnsLabel =
  | { kind: "automatic" }
  | { kind: "localResolver" }
  | { kind: "provider"; provider: string; variant: string }
  | { kind: "preset"; name: string }
  | { kind: "custom" };

export type VariantLabel =
  | "standard"
  | "noFiltering"
  | "malware"
  | "family"
  | "adult"
  | "adBlocking"
  | "child"
  | "childNoAds"
  | "ecs"
  | "social"
  | "uncensored";

export interface DnscryptStamp {
  name: string;
  address: string;
  stamp: string;
}

export interface DnsVariant {
  id: string;
  default: boolean;
  label: VariantLabel;
  ipv4: string[];
  ipv6: string[];
  doh?: string;
  dot?: string;
  dnscrypt: DnscryptStamp[];
}

export interface DnsProvider {
  id: string;
  name: string;
  homepage: string;
  sources: string[];
  verifiedOn: string;
  variants: DnsVariant[];
}

export interface DnsCatalog {
  providers: DnsProvider[];
}

export interface CustomDnsPreset {
  id: string;
  name: string;
  ipv4: string[];
  ipv6: string[];
  dohTemplate?: string;
  dotHost?: string;
}

export interface DnsSettings {
  preferEncryption: boolean;
  allowFallback: boolean;
  presets: CustomDnsPreset[];
}

export interface FeatureSupport {
  available: boolean;
  reason?: string;
}

export type DnsMethod = "windowsIpHelper" | "networkManager" | "mock";

export interface DnsCapabilities {
  supported: boolean;
  reason?: string;
  canChange: boolean;
  changeReason?: string;
  ipv4: FeatureSupport;
  ipv6: FeatureSupport;
  doh: FeatureSupport;
  dot: FeatureSupport;
  method?: DnsMethod;
}

export interface DnsStatus {
  configured: DnsSummary;
  dohTemplate?: string;
  dotServerName?: string;
  fallback?: boolean;
  effectiveIpv4: string[];
  effectiveIpv6: string[];
  profileOverride: boolean;
  nmConnectionUuid?: string;
}

export interface DnsView {
  capabilities: DnsCapabilities;
  status?: DnsStatus;
  statusErrorCode?: string;
  restorable: boolean;
  matched?: DnsLabel;
}

export interface DnsReport {
  method: DnsMethod;
  status: ChangeStatus;
  previous?: DnsSummary;
  requested?: DnsSummary;
  observed?: DnsSummary;
  effective: string[];
  detailCode?: string;
  warnings?: string[];
  cacheFlushed: boolean;
  nmConnectionUuid?: string;
}

export interface DnsOpResult {
  adapterId: string;
  adapterName: string;
  entry: HistoryEntry | null;
  report: DnsReport | null;
  error: ApiError | null;
  success: boolean;
}

export interface DnsHistory {
  previous?: DnsSummary;
  requested?: DnsSummary;
  observed?: DnsSummary;
  label?: DnsLabel;
  warnings?: string[];
}

export interface LatencyStats {
  medianUs: number;
  minUs: number;
  maxUs: number;
}

export type ProbeOutcome =
  | { kind: "answered"; stats: LatencyStats; answered: number; attempts: number; rcode: number }
  | { kind: "timeout" }
  | { kind: "refused"; rcode: number }
  | { kind: "unreachable"; message: string }
  | { kind: "cancelled" };

export interface LatencyResult {
  addr: string;
  outcome: ProbeOutcome;
}
