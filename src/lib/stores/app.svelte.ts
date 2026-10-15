import { api, events, toApiError } from "../api";
import { i18n } from "../i18n/index.svelte";
import type {
  Adapter,
  ApiError,
  AppInfo,
  BlacklistEntry,
  HistoryEntry,
  MacFormat,
  Page,
  Settings,
  Stage,
  UpdateInfo,
  Vendor,
} from "../types";

export type ToastKind = "success" | "error" | "info" | "warning";

export interface Toast {
  id: number;
  kind: ToastKind;
  message: string;
}

const DEFAULT_FORMAT: MacFormat = { separator: "colon", uppercase: true };

class AppStore {
  info = $state<AppInfo | null>(null);
  settings = $state<Settings | null>(null);
  adapters = $state<Adapter[]>([]);
  adaptersError = $state<ApiError | null>(null);
  adaptersLoading = $state(false);
  history = $state<HistoryEntry[]>([]);
  blacklist = $state<BlacklistEntry[]>([]);
  vendors = $state<Vendor[]>([]);
  page = $state<Page>("dashboard");
  selectedId = $state<string | null>(null);
  busy = $state<{ adapterId: string; stage: Stage | null } | null>(null);
  update = $state<UpdateInfo | null>(null);
  updateProgress = $state<{ downloaded: number; total: number | null } | null>(null);
  updateInstalling = $state(false);
  toasts = $state<Toast[]>([]);
  ready = $state(false);
  fatal = $state<string | null>(null);

  #toastId = 0;
  #saveQueue: Promise<unknown> = Promise.resolve();
  #pendingSaves = 0;

  get format(): MacFormat {
    return this.settings?.macFormat ?? DEFAULT_FORMAT;
  }

  get selected(): Adapter | null {
    return this.adapters.find((a) => a.id === this.selectedId) ?? null;
  }

  get visibleAdapters(): Adapter[] {
    const showVirtual = this.settings?.showVirtualAdapters ?? false;
    return this.adapters.filter((a) => showVirtual || !a.isVirtual || a.id === this.selectedId);
  }

  get readOnly(): boolean {
    return this.info?.readOnly ?? false;
  }

  async init() {
    try {
      const [info, settings] = await Promise.all([api.appInfo(), api.settingsGet()]);
      this.info = info;
      this.settings = settings;
      i18n.setLanguage(settings.language);
      this.selectedId = settings.selectedAdapterId ?? null;
    } catch (e) {
      this.fatal = toApiError(e).message;
      return;
    }
    await Promise.all([
      this.refreshAdapters(),
      this.refreshHistory(),
      this.refreshBlacklist(),
      api.listVendors().then((v) => (this.vendors = v)),
    ]);
    this.#subscribe();
    this.ready = true;
  }

  #subscribe() {
    void events.onProgress((e) => {
      if (this.busy && this.busy.adapterId === e.adapterId) this.busy.stage = e.stage;
    });
    void events.onBusy((e) => {
      this.busy = e.busy ? { adapterId: e.adapterId, stage: null } : null;
    });
    void events.onAdaptersChanged(() => void this.refreshAdapters(true));
    void events.onHistoryChanged(() => void this.refreshHistory());
    void events.onBlacklistChanged(() => void this.refreshBlacklist());
    void events.onSettingsChanged((s) => {
      // While our own saves are queued, their echo would undo edits that are shown but not yet
      // saved; the last save brings the authoritative copy anyway.
      if (this.#pendingSaves > 0) return;
      this.settings = s;
      i18n.setLanguage(s.language);
    });
    void events.onNavigate((p) => this.navigate(p));
    void events.onUpdateAvailable((u) => (this.update = u));
    void events.onUpdateProgress((p) => (this.updateProgress = p));
    void events.onUpdateInstalling(() => (this.updateInstalling = true));
  }

  async refreshAdapters(quiet = false) {
    if (!quiet) this.adaptersLoading = true;
    try {
      const list = await api.listAdapters();
      this.adapters = list;
      this.adaptersError = null;
      if (!this.selectedId || !list.some((a) => a.id === this.selectedId)) {
        const first =
          list.find((a) => !a.isVirtual && a.connected) ??
          list.find((a) => !a.isVirtual) ??
          list[0];
        this.selectedId = first?.id ?? null;
      }
    } catch (e) {
      this.adaptersError = toApiError(e);
    } finally {
      this.adaptersLoading = false;
    }
  }

  async refreshHistory() {
    try {
      this.history = await api.historyList();
    } catch (e) {
      this.toastError(e);
    }
  }

  async refreshBlacklist() {
    try {
      this.blacklist = await api.blacklistGet();
    } catch (e) {
      this.toastError(e);
    }
  }

  /**
   * Applies a modification to the settings and saves it. The change shows immediately; saves
   * run one after another and each re-applies its change to the latest copy, so quick
   * successive edits (two checkboxes, a toggle while autostart is being written) never
   * overwrite each other.
   */
  saveSettings(mutate: (s: Settings) => void): Promise<boolean> {
    if (!this.settings) return Promise.resolve(false);
    const shown = $state.snapshot(this.settings) as Settings;
    mutate(shown);
    this.settings = shown;
    this.#pendingSaves++;
    const job = this.#saveQueue.then(async () => {
      try {
        const base = this.settings ?? shown;
        const next = $state.snapshot(base) as Settings;
        mutate(next);
        const saved = await api.settingsUpdate(next);
        // Later queued edits are already visible; the last save brings the authoritative copy.
        if (this.#pendingSaves === 1) this.settings = saved;
        i18n.setLanguage(saved.language);
        return true;
      } catch (e) {
        this.toastError(e);
        // Reload the authoritative copy so the UI does not show unsaved values.
        this.settings = await api.settingsGet().catch(() => this.settings);
        return false;
      } finally {
        this.#pendingSaves--;
      }
    });
    this.#saveQueue = job.catch(() => false);
    return job;
  }

  select(id: string) {
    this.selectedId = id;
    if (this.settings && this.settings.selectedAdapterId !== id) {
      void this.saveSettings((s) => (s.selectedAdapterId = id));
    }
  }

  navigate(page: Page) {
    this.page = page;
  }

  toast(kind: ToastKind, message: string) {
    const id = ++this.#toastId;
    this.toasts.push({ id, kind, message });
    if (this.toasts.length > 4) this.toasts.shift();
    const timeout = kind === "error" ? 8000 : kind === "warning" ? 6500 : 4500;
    setTimeout(() => this.dismiss(id), timeout);
  }

  toastError(e: unknown) {
    const err = toApiError(e);
    this.toast("error", i18n.error(err.code));
    console.warn(err);
  }

  dismiss(id: number) {
    this.toasts = this.toasts.filter((t) => t.id !== id);
  }
}

export const app = new AppStore();
