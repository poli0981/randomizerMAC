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

export type Page = "dashboard" | "history" | "settings" | "logs" | "update" | "about" | "social";
