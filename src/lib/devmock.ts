// Development-only: lets the UI run in a normal browser (`npm run dev`, then open
// http://localhost:1420) with fake backend data. Never included in production builds.
import { mockIPC } from "@tauri-apps/api/mocks";
import type { Adapter, HistoryEntry, Settings } from "./types";

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
    ipv4: ["192.168.1.107"],
    ipv6: ["fe80::1c2b:3a4d:5e6f:7081"],
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
    ipv4: ["10.0.0.42"],
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
  schemaVersion: 2,
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
};

function randomMac() {
  const b = crypto.getRandomValues(new Uint8Array(6));
  b[0] = (b[0]! & 0xfc) | 0x02;
  return [...b].map((x) => x.toString(16).padStart(2, "0").toUpperCase()).join(":");
}

export function installDevMock() {
  mockIPC((cmd, payload) => {
    const args = (payload ?? {}) as Record<string, unknown>;
    switch (cmd) {
      case "app_info":
        return {
          version: "2.0.0",
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
                message: "RandomMac starting version=2.0.0 os=windows elevated=true",
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
          currentVersion: "2.0.0",
          available: false,
          version: null,
          date: null,
          notes: null,
          checkedAt: new Date().toISOString(),
        };
      case "plugin:event|listen":
        return 1;
      default:
        return null;
    }
  });
}
