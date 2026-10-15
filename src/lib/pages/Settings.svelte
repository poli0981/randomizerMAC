<script lang="ts">
  import {
    FolderOpen,
    Download,
    Upload,
    Archive,
    RotateCcw,
    Plus,
    Trash2,
    History as HistoryIcon,
  } from "@lucide/svelte";
  import PageHeader from "../components/PageHeader.svelte";
  import Toggle from "../components/Toggle.svelte";
  import AdapterPicker from "../components/AdapterPicker.svelte";
  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import { app } from "../stores/app.svelte";
  import { api } from "../api";
  import { i18n, t, type MessageKey } from "../i18n/index.svelte";
  import { formatDate } from "../format";
  import type { Settings, Separator } from "../types";

  const s = $derived(app.settings);
  const readOnly = $derived(app.readOnly);
  const isLinux = $derived(app.info?.os === "linux");
  const isWindows = $derived(app.info?.os === "windows");
  const imitable = $derived(app.vendors.filter((v) => v.imitable));

  let pattern = $state("");
  let scope = $state("");
  let note = $state("");
  let patternError = $state<string | null>(null);
  let confirmReset = $state(false);

  const intervals = [15, 30, 60, 120, 360, 720, 1440];

  async function save(mutate: (s: Settings) => void) {
    await app.saveSettings(mutate);
  }

  async function addRule(e: SubmitEvent) {
    e.preventDefault();
    patternError = null;
    try {
      await api.blacklistAdd(pattern, scope || null, note || null);
      pattern = "";
      note = "";
      await app.refreshBlacklist();
      app.toast("success", t("settings.blacklist.added"));
    } catch (err) {
      const code = (err as { code?: string }).code;
      patternError = i18n.error(code);
    }
  }

  async function removeRule(id: string) {
    try {
      await api.blacklistRemove(id);
      await app.refreshBlacklist();
    } catch (e) {
      app.toastError(e);
    }
  }

  async function run<T>(fn: () => Promise<T>, onDone?: (r: T) => void) {
    try {
      const r = await fn();
      onDone?.(r);
    } catch (e) {
      app.toastError(e);
    }
  }

  function savedTo(path: string | null) {
    if (path) app.toast("success", t("common.savedTo", { path }));
  }

  function adapterName(id: string | undefined) {
    if (!id) return t("settings.blacklist.allAdapters");
    return app.adapters.find((a) => a.id === id)?.name ?? id;
  }

  function intervalLabel(m: number) {
    return m < 60 ? t("common.minutes", { n: m }) : t("common.hours", { n: m / 60 });
  }
</script>

<PageHeader title={t("settings.title")} subtitle={t("settings.subtitle")} />

{#if s}
  <div class="grid">
    <section class="card" aria-labelledby="general-title">
      <h2 id="general-title">{t("settings.general")}</h2>
      <div class="fields">
        <div class="field">
          <label for="lang">{t("settings.language")}</label>
          <select
            id="lang"
            class="select"
            value={s.language}
            onchange={(e) =>
              save(
                (x) =>
                  (x.language = (e.currentTarget as HTMLSelectElement)
                    .value as Settings["language"]),
              )}
          >
            <option value="system">{t("settings.language.system")}</option>
            <option value="en">English</option>
            <option value="vi">Tiếng Việt</option>
          </select>
        </div>
        <div class="field">
          <label for="theme">{t("settings.theme")}</label>
          <select
            id="theme"
            class="select"
            value={s.theme}
            onchange={(e) =>
              save(
                (x) =>
                  (x.theme = (e.currentTarget as HTMLSelectElement).value as Settings["theme"]),
              )}
          >
            <option value="system">{t("settings.theme.system")}</option>
            <option value="light">{t("settings.theme.light")}</option>
            <option value="dark">{t("settings.theme.dark")}</option>
          </select>
        </div>
        <div class="field">
          <label for="sep">{t("settings.macFormat")}</label>
          <select
            id="sep"
            class="select"
            value={s.macFormat.separator}
            onchange={(e) =>
              save(
                (x) =>
                  (x.macFormat.separator = (e.currentTarget as HTMLSelectElement)
                    .value as Separator),
              )}
          >
            <option value="colon">{t("settings.separator.colon")}</option>
            <option value="hyphen">{t("settings.separator.hyphen")}</option>
            <option value="dot">{t("settings.separator.dot")}</option>
            <option value="none">{t("settings.separator.none")}</option>
          </select>
        </div>
      </div>
      <Toggle
        checked={s.macFormat.uppercase}
        label={t("settings.uppercase")}
        onchange={(v) => save((x) => (x.macFormat.uppercase = v))}
      />
      <Toggle
        checked={s.showVirtualAdapters}
        label={t("dashboard.showVirtual")}
        onchange={(v) => save((x) => (x.showVirtualAdapters = v))}
      />
    </section>

    <section class="card" aria-labelledby="startup-title">
      <h2 id="startup-title">{t("settings.startup")}</h2>
      <Toggle
        checked={s.startWithSystem}
        label={t("settings.startWithSystem")}
        description={t("settings.startWithSystem.desc")}
        disabled={readOnly && isWindows}
        hint={readOnly && isWindows ? t("settings.readOnlyHint") : undefined}
        onchange={(v) => save((x) => (x.startWithSystem = v))}
      />
      <Toggle
        checked={s.startMinimized}
        label={t("settings.startMinimized")}
        description={t("settings.startMinimized.desc")}
        onchange={(v) => save((x) => (x.startMinimized = v))}
      />
      <Toggle
        checked={s.closeToTray && (app.info?.trayAvailable ?? true)}
        label={t("settings.closeToTray")}
        disabled={!(app.info?.trayAvailable ?? true)}
        hint={app.info && !app.info.trayAvailable ? t("settings.trayUnavailable") : undefined}
        onchange={(v) => save((x) => (x.closeToTray = v))}
      />
      <Toggle
        checked={s.showNotifications}
        label={t("settings.notifications")}
        description={t("settings.notifications.desc")}
        onchange={(v) => save((x) => (x.showNotifications = v))}
      />
      <Toggle
        checked={s.confirmBeforeApply}
        label={t("settings.confirm")}
        onchange={(v) => save((x) => (x.confirmBeforeApply = v))}
      />
    </section>

    <section class="card" aria-labelledby="gen-title">
      <h2 id="gen-title">{t("settings.generation")}</h2>
      <p class="subtle">{t("settings.generation.desc")}</p>
      <div class="modes" role="radiogroup" aria-labelledby="gen-title">
        {#each ["random", "keepVendor", "vendor"] as kind (kind)}
          <label class="mode" class:active={s.generationMode.kind === kind}>
            <input
              type="radio"
              name="gen-mode"
              value={kind}
              checked={s.generationMode.kind === kind}
              onchange={() =>
                save((x) => {
                  x.generationMode =
                    kind === "vendor"
                      ? {
                          kind: "vendor",
                          vendorId:
                            x.generationMode.kind === "vendor"
                              ? x.generationMode.vendorId
                              : (imitable[0]?.id ?? "apple"),
                        }
                      : { kind: kind as "random" | "keepVendor" };
                })}
            />
            <span class="mode-title">{t(`mode.${kind}` as MessageKey)}</span>
            <span class="subtle">{t(`mode.${kind}.desc` as MessageKey)}</span>
          </label>
        {/each}
      </div>
      {#if s.generationMode.kind === "vendor"}
        <div class="field narrow">
          <label for="vendor">{t("dashboard.vendorSelect")}</label>
          <select
            id="vendor"
            class="select"
            value={s.generationMode.vendorId}
            onchange={(e) =>
              save(
                (x) =>
                  (x.generationMode = {
                    kind: "vendor",
                    vendorId: (e.currentTarget as HTMLSelectElement).value,
                  }),
              )}
          >
            {#each imitable as v (v.id)}<option value={v.id}>{v.name}</option>{/each}
          </select>
        </div>
      {/if}
    </section>

    <section class="card" aria-labelledby="auto-title">
      <h2 id="auto-title">{t("settings.automation")}</h2>
      <Toggle
        checked={s.automation.changeOnStartup}
        label={t("settings.changeOnStartup")}
        disabled={readOnly}
        onchange={(v) => save((x) => (x.automation.changeOnStartup = v))}
      />
      {#if s.automation.changeOnStartup}
        <p class="subtle">{t("settings.changeOnStartup.desc")}</p>
        <AdapterPicker
          label={t("settings.changeOnStartup")}
          selected={s.automation.startupAdapterIds}
          onchange={(ids) => save((x) => (x.automation.startupAdapterIds = ids))}
        />
      {/if}
      <Toggle
        checked={s.automation.intervalEnabled}
        label={t("settings.interval")}
        disabled={readOnly}
        onchange={(v) => save((x) => (x.automation.intervalEnabled = v))}
      />
      {#if s.automation.intervalEnabled}
        <div class="row interval">
          <label for="interval" class="subtle">{t("settings.interval.desc")}</label>
          <select
            id="interval"
            class="select"
            value={s.automation.intervalMinutes}
            onchange={(e) =>
              save(
                (x) =>
                  (x.automation.intervalMinutes = Number(
                    (e.currentTarget as HTMLSelectElement).value,
                  )),
              )}
          >
            {#each intervals as m (m)}<option value={m}>{intervalLabel(m)}</option>{/each}
          </select>
        </div>
        <AdapterPicker
          label={t("settings.interval")}
          selected={s.automation.intervalAdapterIds}
          onchange={(ids) => save((x) => (x.automation.intervalAdapterIds = ids))}
        />
      {/if}
      {#if isLinux && (s.automation.changeOnStartup || s.automation.intervalEnabled)}
        <p class="subtle warn">{t("settings.linuxWarning")}</p>
      {/if}
      <Toggle
        checked={s.automation.restoreOnExit}
        label={t("settings.restoreOnExit")}
        description={t("settings.restoreOnExit.desc")}
        disabled={readOnly}
        onchange={(v) => save((x) => (x.automation.restoreOnExit = v))}
      />
    </section>

    <section class="card wide" aria-labelledby="bl-title">
      <h2 id="bl-title">{t("settings.blacklist")}</h2>
      <p class="subtle">{t("settings.blacklist.desc")}</p>
      <form class="bl-form" onsubmit={addRule}>
        <div class="field">
          <label for="bl-pattern">{t("settings.blacklist.pattern")}</label>
          <input
            id="bl-pattern"
            class="input mono"
            bind:value={pattern}
            placeholder="AA:BB:CC"
            aria-invalid={!!patternError}
            aria-describedby="bl-error"
            spellcheck="false"
            autocomplete="off"
          />
        </div>
        <div class="field">
          <label for="bl-scope">{t("settings.blacklist.scope")}</label>
          <select id="bl-scope" class="select" bind:value={scope}>
            <option value="">{t("settings.blacklist.allAdapters")}</option>
            {#each app.adapters as a (a.id)}<option value={a.id}>{a.name}</option>{/each}
          </select>
        </div>
        <div class="field">
          <label for="bl-note">{t("settings.blacklist.note")}</label>
          <input id="bl-note" class="input" bind:value={note} maxlength="200" />
        </div>
        <button class="btn primary" type="submit" disabled={!pattern.trim()}
          ><Plus size={16} aria-hidden="true" />{t("common.add")}</button
        >
      </form>
      <p id="bl-error" class="error-text" aria-live="polite">{patternError ?? ""}</p>
      {#if app.blacklist.length === 0}
        <p class="subtle">{t("settings.blacklist.empty")}</p>
      {:else}
        <ul class="rules">
          {#each app.blacklist as r (r.id)}
            <li>
              <span class="mono">{r.pattern}</span>
              <span class="badge">{adapterName(r.adapterId)}</span>
              <span class="subtle note">{r.note ?? ""}</span>
              <span class="subtle">{formatDate(r.createdAt, i18n.locale)}</span>
              <button
                class="btn ghost icon small"
                aria-label="{t('common.remove')} {r.pattern}"
                onclick={() => removeRule(r.id)}
              >
                <Trash2 size={14} aria-hidden="true" />
              </button>
            </li>
          {/each}
        </ul>
      {/if}
      <div class="row">
        <button
          class="btn small"
          onclick={() =>
            run(
              api.blacklistImport,
              (n) =>
                n != null && app.toast("success", t("settings.blacklist.imported", { count: n })),
            )}
        >
          <Upload size={14} aria-hidden="true" />{t("common.import")}
        </button>
        <button
          class="btn small"
          onclick={() => run(api.blacklistExport, savedTo)}
          disabled={app.blacklist.length === 0}
        >
          <Download size={14} aria-hidden="true" />{t("common.export")}
        </button>
      </div>
    </section>

    {#if isLinux}
      <section class="card" aria-labelledby="linux-title">
        <h2 id="linux-title">{t("settings.linux")}</h2>
        <p class="label">{t("settings.linuxMethod")}</p>
        <div class="modes" role="radiogroup" aria-labelledby="linux-title">
          {#each ["auto", "networkManager", "iproute2"] as m (m)}
            <label class="mode" class:active={s.linux.method === m}>
              <input
                type="radio"
                name="linux-method"
                value={m}
                checked={s.linux.method === m}
                onchange={() => save((x) => (x.linux.method = m as Settings["linux"]["method"]))}
              />
              <span class="mode-title">{t(`settings.linuxMethod.${m}` as MessageKey)}</span>
              <span class="subtle">{t(`settings.linuxMethod.${m}.desc` as MessageKey)}</span>
            </label>
          {/each}
        </div>
      </section>
    {/if}

    <section class="card" aria-labelledby="upd-title">
      <h2 id="upd-title">{t("settings.updates")}</h2>
      <Toggle
        checked={s.autoCheckUpdates}
        label={t("settings.autoCheck")}
        description={t("settings.autoCheck.desc")}
        onchange={(v) => save((x) => (x.autoCheckUpdates = v))}
      />
    </section>

    <section class="card wide" aria-labelledby="data-title">
      <h2 id="data-title">{t("settings.data")}</h2>
      <p class="subtle">
        {t("settings.dataLocation")}: <span class="mono">{app.info?.dataDir}</span>
      </p>
      <div class="row buttons">
        <button class="btn small" onclick={() => run(() => api.openLocation("data"))}
          ><FolderOpen size={14} aria-hidden="true" />{t("settings.openData")}</button
        >
        <button class="btn small" onclick={() => run(() => api.openLocation("logs"))}
          ><FolderOpen size={14} aria-hidden="true" />{t("settings.openLogs")}</button
        >
        <button class="btn small" onclick={() => run(api.settingsExport, savedTo)}
          ><Download size={14} aria-hidden="true" />{t("settings.exportSettings")}</button
        >
        <button
          class="btn small"
          onclick={() =>
            run(api.settingsImport, (r) => r && app.toast("success", t("settings.saved")))}
          ><Upload size={14} aria-hidden="true" />{t("settings.importSettings")}</button
        >
        <button class="btn small" onclick={() => run(api.bundleExport, savedTo)}
          ><Archive size={14} aria-hidden="true" />{t("settings.exportAll")}</button
        >
        {#if isWindows}
          <button
            class="btn small"
            onclick={() =>
              run(api.v1Import, (r) =>
                r
                  ? app.toast(
                      "success",
                      t("banner.v1Import", { history: r.history, blacklist: r.blacklist }),
                    )
                  : app.toast("info", t("settings.importV1.none")),
              )}
          >
            <HistoryIcon size={14} aria-hidden="true" />{t("settings.importV1")}
          </button>
        {/if}
      </div>
      <div class="row retention">
        <label for="hist-max">{t("settings.historyMax")}</label>
        <input
          id="hist-max"
          class="input num"
          type="number"
          min="10"
          max="10000"
          value={s.history.maxEntries}
          onchange={(e) =>
            save(
              (x) =>
                (x.history.maxEntries = Number((e.currentTarget as HTMLInputElement).value) || 500),
            )}
        />
        <span class="subtle">{t("settings.historyMaxUnit")}</span>
        <label for="hist-days">{t("settings.historyDays")}</label>
        <input
          id="hist-days"
          class="input num"
          type="number"
          min="0"
          max="3650"
          value={s.history.retentionDays}
          onchange={(e) =>
            save(
              (x) =>
                (x.history.retentionDays =
                  Number((e.currentTarget as HTMLInputElement).value) || 0),
            )}
        />
        <span class="subtle">{t("settings.historyDaysUnit")}</span>
      </div>
      <div class="row">
        <button class="btn small danger" onclick={() => (confirmReset = true)}
          ><RotateCcw size={14} aria-hidden="true" />{t("settings.reset")}</button
        >
      </div>
    </section>
  </div>
{/if}

<ConfirmDialog
  open={confirmReset}
  title={t("settings.resetTitle")}
  body={t("settings.resetBody")}
  confirmLabel={t("settings.reset")}
  danger
  onconfirm={() => {
    confirmReset = false;
    void run(api.settingsReset, (r) => (app.settings = r));
  }}
  oncancel={() => (confirmReset = false)}
/>

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 16px;
    align-items: start;
  }
  .wide {
    grid-column: 1 / -1;
  }
  @media (max-width: 1050px) {
    .grid {
      grid-template-columns: 1fr;
    }
  }
  h2 {
    font-size: 15px;
    margin-bottom: 8px;
  }
  .fields {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(170px, 1fr));
    gap: 12px;
    margin-bottom: 6px;
  }
  .modes {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin: 10px 0;
  }
  .mode {
    display: grid;
    grid-template-columns: auto 1fr;
    column-gap: 10px;
    padding: 10px 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    cursor: pointer;
  }
  .mode.active {
    border-color: var(--accent);
    background: var(--accent-soft);
  }
  .mode input {
    grid-row: span 2;
    margin-top: 3px;
  }
  .mode-title {
    font-weight: 600;
  }
  .narrow {
    max-width: 260px;
  }
  .interval {
    margin: 4px 0;
  }
  .interval .select {
    width: auto;
  }
  .warn {
    color: var(--warning);
    margin: 6px 0;
  }
  .bl-form {
    display: grid;
    grid-template-columns: 1.2fr 1fr 1.4fr auto;
    gap: 10px;
    align-items: end;
    margin-top: 12px;
  }
  @media (max-width: 900px) {
    .bl-form {
      grid-template-columns: 1fr;
    }
  }
  .error-text {
    color: var(--danger);
    min-height: 1.2em;
    font-size: 13px;
    margin: 4px 0;
  }
  .rules {
    list-style: none;
    padding: 0;
    margin: 0 0 12px;
    border: 1px solid var(--border);
    border-radius: var(--radius);
  }
  .rules li {
    display: grid;
    grid-template-columns: 170px auto 1fr auto auto;
    align-items: center;
    gap: 10px;
    padding: 6px 10px;
  }
  .rules li + li {
    border-top: 1px solid var(--border);
  }
  .note {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .buttons {
    margin: 10px 0;
  }
  .retention {
    margin: 10px 0 14px;
  }
  .num {
    width: 96px;
  }
</style>
