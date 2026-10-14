<script lang="ts">
  import {
    Wifi,
    Cable,
    Network,
    RefreshCw,
    Shuffle,
    Check,
    Undo2,
    Power,
    TriangleAlert,
    Loader,
  } from "@lucide/svelte";
  import PageHeader from "../components/PageHeader.svelte";
  import MacValue from "../components/MacValue.svelte";
  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import { app } from "../stores/app.svelte";
  import { api } from "../api";
  import { i18n, t, type MessageKey } from "../i18n/index.svelte";
  import { canonical, formatMac, parseMac } from "../mac";
  import { formatRelative } from "../format";
  import { describeOutcome } from "../outcome";
  import type { Adapter, GenerationMode, MacInfo, Stage } from "../types";

  const kindIcon = { ethernet: Cable, wireless: Wifi, other: Network };
  const stages: Stage[] = ["preparing", "writing", "restarting", "verifying"];

  let input = $state("");
  let info = $state<MacInfo | null>(null);
  let confirmOpen = $state(false);
  let restoring = $state(false);
  let lastMessage = $state<{ kind: string; message: string } | null>(null);
  let inspectTimer: ReturnType<typeof setTimeout> | undefined;

  const adapter = $derived(app.selected);
  const busyHere = $derived(!!app.busy && app.busy.adapterId === adapter?.id);
  const anyBusy = $derived(!!app.busy);
  const mode = $derived<GenerationMode>(app.settings?.generationMode ?? { kind: "random" });
  const parsed = $derived(parseMac(input));
  const inputInvalid = $derived(input.trim().length > 0 && !parsed);
  const canApply = $derived(
    !!adapter &&
      !!parsed &&
      !anyBusy &&
      !app.readOnly &&
      adapter.state !== "disabled" &&
      adapter.methods.length > 0 &&
      (info?.valid ?? true),
  );
  const recent = $derived(app.history.filter((h) => h.adapterId === adapter?.id).slice(0, 5));
  const imitable = $derived(app.vendors.filter((v) => v.imitable));

  // Reset the editor when another adapter is selected.
  let lastAdapterId: string | null = null;
  $effect(() => {
    const id = adapter?.id ?? null;
    if (id !== lastAdapterId) {
      lastAdapterId = id;
      input = "";
      info = null;
      lastMessage = null;
    }
  });

  function onInput(value: string) {
    input = value;
    lastMessage = null;
    clearTimeout(inspectTimer);
    if (!parseMac(value)) {
      info = null;
      return;
    }
    inspectTimer = setTimeout(async () => {
      try {
        info = await api.inspectMac(value, adapter?.id ?? null);
      } catch (e) {
        app.toastError(e);
      }
    }, 200);
  }

  async function randomize() {
    if (!adapter || anyBusy) return;
    try {
      info = await api.generateMac(adapter.id, mode);
      input = formatMac(info.mac, app.format);
      lastMessage = null;
    } catch (e) {
      app.toastError(e);
    }
  }

  function requestApply() {
    if (!canApply) return;
    if (app.settings?.confirmBeforeApply) {
      confirmOpen = true;
    } else {
      void apply();
    }
  }

  async function apply() {
    confirmOpen = false;
    if (!adapter || !parsed) return;
    const mac = canonical(input);
    if (!mac) return;
    try {
      const result = await api.applyMac(adapter.id, mac);
      const out = describeOutcome(result, false, adapter, app.format, app.info?.os);
      lastMessage = out;
      app.toast(out.kind, out.message);
      if (result.success) {
        input = "";
        info = null;
      }
    } catch (e) {
      app.toastError(e);
    }
  }

  async function restore() {
    if (!adapter || anyBusy) return;
    restoring = true;
    try {
      const result = await api.restoreMac(adapter.id);
      const out = describeOutcome(result, true, adapter, app.format, app.info?.os);
      lastMessage = out;
      app.toast(out.kind, out.message);
    } catch (e) {
      app.toastError(e);
    } finally {
      restoring = false;
    }
  }

  async function enable(a: Adapter) {
    try {
      await api.setAdapterEnabled(a.id, true);
      await app.refreshAdapters(true);
    } catch (e) {
      app.toastError(e);
    }
  }

  async function setMode(kind: string) {
    await app.saveSettings((s) => {
      if (kind === "vendor") {
        const current =
          s.generationMode.kind === "vendor" ? s.generationMode.vendorId : imitable[0]?.id;
        s.generationMode = { kind: "vendor", vendorId: current ?? "apple" };
      } else {
        s.generationMode = { kind: kind as "random" | "keepVendor" };
      }
    });
  }

  async function setVendor(vendorId: string) {
    await app.saveSettings((s) => (s.generationMode = { kind: "vendor", vendorId }));
  }

  function onListKey(e: KeyboardEvent) {
    const list = app.visibleAdapters;
    const idx = list.findIndex((a) => a.id === app.selectedId);
    let next: number;
    if (e.key === "ArrowDown") next = Math.min(list.length - 1, idx + 1);
    else if (e.key === "ArrowUp") next = Math.max(0, idx - 1);
    else if (e.key === "Home") next = 0;
    else if (e.key === "End") next = list.length - 1;
    else return;
    e.preventDefault();
    const target = list[next];
    if (target) {
      app.select(target.id);
      document.getElementById(`adapter-${next}`)?.focus();
    }
  }

  function onKey(e: KeyboardEvent) {
    if (!(e.ctrlKey || e.metaKey)) return;
    if (e.key.toLowerCase() === "r" && !e.shiftKey) {
      e.preventDefault();
      void randomize();
    } else if (e.key === "Enter") {
      e.preventDefault();
      requestApply();
    }
  }

  function stateKey(a: Adapter): MessageKey {
    if (a.state === "disabled") return "dashboard.state.disabled";
    if (a.connected) return "dashboard.state.up";
    if (a.state === "unknown") return "dashboard.state.unknown";
    return "dashboard.state.down";
  }

  function methodLabel(m: string): string {
    return (
      {
        windowsRegistry: "Windows (NetworkAddress)",
        networkManager: "NetworkManager",
        iproute2: "iproute2",
        mock: "Demo",
      }[m] ?? m
    );
  }

  const warningKeys: Record<string, MessageKey> = {
    multicast: "warning.multicast",
    zero: "warning.zero",
    sameAsCurrent: "warning.sameAsCurrent",
    blacklisted: "warning.blacklisted",
    wifiNeedsLocal: "warning.wifiNeedsLocal",
  };
</script>

<svelte:window onkeydown={onKey} />

<PageHeader title={t("dashboard.title")} subtitle={t("dashboard.subtitle")}>
  {#snippet actions()}
    <button
      class="btn"
      onclick={() => app.refreshAdapters()}
      disabled={app.adaptersLoading}
      title="F5"
    >
      <RefreshCw size={16} aria-hidden="true" class={app.adaptersLoading ? "spin" : ""} />
      {t("common.refresh")}
    </button>
  {/snippet}
</PageHeader>

<div class="layout">
  <section class="card adapters" aria-labelledby="adapters-title">
    <div class="card-head">
      <h2 id="adapters-title" class="card-title">{t("dashboard.adapters")}</h2>
      <span class="badge">{app.visibleAdapters.length}</span>
    </div>

    {#if app.adaptersError}
      <div class="callout danger" role="alert">
        <TriangleAlert size={18} aria-hidden="true" />
        <div class="stack-sm">
          <p>
            {t("dashboard.loadFailed")} <span class="subtle">({app.adaptersError.message})</span>
          </p>
          <button class="btn small" onclick={() => app.refreshAdapters()}
            >{t("dashboard.retry")}</button
          >
        </div>
      </div>
    {:else if app.visibleAdapters.length === 0}
      <div class="empty">
        <p>{t("dashboard.noAdapters")}</p>
        <p class="subtle">{t("dashboard.noAdaptersHint")}</p>
      </div>
    {:else}
      <ul
        class="adapter-list"
        role="listbox"
        aria-labelledby="adapters-title"
        tabindex="-1"
        onkeydown={onListKey}
      >
        {#each app.visibleAdapters as a, i (a.id)}
          {@const Icon = kindIcon[a.kind]}
          {@const selected = a.id === app.selectedId}
          <li
            id="adapter-{i}"
            role="option"
            aria-selected={selected}
            tabindex={selected ? 0 : -1}
            class:selected
            onclick={() => app.select(a.id)}
            onkeydown={(e) =>
              (e.key === "Enter" || e.key === " ") && (e.preventDefault(), app.select(a.id))}
          >
            <span class="kind" class:wireless={a.kind === "wireless"}
              ><Icon size={18} aria-hidden="true" /></span
            >
            <span class="info">
              <span class="name-row">
                <span class="name">{a.name}</span>
                {#if a.changed}<span class="badge accent">{t("dashboard.badge.changed")}</span>{/if}
                {#if a.isVirtual}<span class="badge">{t("dashboard.badge.virtual")}</span>{/if}
                {#if a.state === "disabled"}<span class="badge warning"
                    >{t("dashboard.badge.disabled")}</span
                  >{/if}
              </span>
              <span class="desc subtle">{a.description}</span>
              <span class="meta">
                <span class="dot" class:on={a.connected} aria-hidden="true"></span>
                <span class="subtle">{t(stateKey(a))}</span>
                <span class="mono subtle">{formatMac(a.currentMac, app.format)}</span>
              </span>
            </span>
            {#if app.busy?.adapterId === a.id}<Loader
                size={16}
                class="spin"
                aria-hidden="true"
              />{/if}
          </li>
        {/each}
      </ul>
    {/if}

    <label class="virtual-toggle">
      <input
        type="checkbox"
        checked={app.settings?.showVirtualAdapters ?? false}
        onchange={(e) =>
          app.saveSettings(
            (s) => (s.showVirtualAdapters = (e.currentTarget as HTMLInputElement).checked),
          )}
      />
      {t("dashboard.showVirtual")}
    </label>
  </section>

  <div class="detail">
    {#if !adapter}
      <section class="card empty-detail">
        <p class="muted">{t("dashboard.selectPrompt")}</p>
      </section>
    {:else}
      {@const Icon = kindIcon[adapter.kind]}
      <section class="card" aria-labelledby="adapter-title">
        <div class="adapter-head">
          <span class="kind big" class:wireless={adapter.kind === "wireless"}
            ><Icon size={22} aria-hidden="true" /></span
          >
          <div class="grow">
            <h2 id="adapter-title" class="card-title">{adapter.name}</h2>
            <p class="subtle">{adapter.description}</p>
          </div>
          <span class="badge" class:success={adapter.connected}>{t(stateKey(adapter))}</span>
        </div>

        {#if adapter.state === "disabled"}
          <div class="callout warning">
            <TriangleAlert size={18} aria-hidden="true" />
            <div class="stack-sm">
              <p>{t("dashboard.disabledHint")}</p>
              <button
                class="btn small"
                onclick={() => enable(adapter)}
                disabled={app.readOnly || anyBusy}
              >
                <Power size={14} aria-hidden="true" />{t("dashboard.enable")}
              </button>
            </div>
          </div>
        {/if}

        <div class="macs">
          <MacValue
            label={t("dashboard.currentMac")}
            mac={adapter.currentMac}
            vendor={adapter.vendor ??
              (adapter.currentMac && (parseMac(adapter.currentMac)?.[0] ?? 0) & 2
                ? t("dashboard.localVendor")
                : null)}
          />
          <MacValue
            label={t("dashboard.originalMac")}
            mac={adapter.originalMac}
            vendor={adapter.hardwareVendor}
          />
          <MacValue
            label={t("dashboard.previewMac")}
            mac={parsed ? canonical(input) : null}
            vendor={info?.vendor ?? null}
            emphasis
          />
        </div>

        <dl class="facts">
          {#if adapter.ipv4.length || adapter.ipv6.length}
            <dt>{t("dashboard.ipAddresses")}</dt>
            <dd class="mono">{[...adapter.ipv4, ...adapter.ipv6.slice(0, 1)].join(", ")}</dd>
          {/if}
          <dt>{t("dashboard.method")}</dt>
          <dd>
            {adapter.methods.length ? adapter.methods.map(methodLabel).join(" · ") : "—"}
            <span class="subtle"
              >· {t(`dashboard.support.${adapter.supportsChange}` as MessageKey)}</span
            >
          </dd>
          {#if adapter.nmConnection}
            <dt>{t("dashboard.nmProfile")}</dt>
            <dd>{adapter.nmConnection.name}</dd>
          {/if}
          {#if adapter.overrideValue}
            <dt>{t("dashboard.override")}</dt>
            <dd class="mono">{adapter.overrideValue}</dd>
          {/if}
        </dl>
      </section>

      <section class="card" aria-labelledby="new-mac-title">
        <h2 id="new-mac-title" class="card-title">{t("dashboard.inputLabel")}</h2>
        <div class="editor">
          <div class="field grow">
            <label for="mac-input" class="sr-only">{t("dashboard.inputLabel")}</label>
            <input
              id="mac-input"
              class="input mono big-input"
              placeholder={t("dashboard.inputPlaceholder")}
              value={input}
              oninput={(e) => onInput((e.currentTarget as HTMLInputElement).value)}
              aria-invalid={inputInvalid}
              aria-describedby="mac-help"
              spellcheck="false"
              autocomplete="off"
              autocapitalize="characters"
              disabled={busyHere}
            />
          </div>
          <div class="field mode">
            <label for="mode-select" class="sr-only">{t("dashboard.mode")}</label>
            <select
              id="mode-select"
              class="select"
              value={mode.kind}
              onchange={(e) => setMode((e.currentTarget as HTMLSelectElement).value)}
            >
              <option value="random">{t("mode.random")}</option>
              <option value="keepVendor">{t("mode.keepVendor")}</option>
              <option value="vendor">{t("mode.vendor")}</option>
            </select>
          </div>
          {#if mode.kind === "vendor"}
            <div class="field mode">
              <label for="vendor-select" class="sr-only">{t("dashboard.vendorSelect")}</label>
              <select
                id="vendor-select"
                class="select"
                value={mode.vendorId}
                onchange={(e) => setVendor((e.currentTarget as HTMLSelectElement).value)}
              >
                {#each imitable as v (v.id)}<option value={v.id}>{v.name}</option>{/each}
              </select>
            </div>
          {/if}
        </div>

        <div id="mac-help" class="help" aria-live="polite">
          {#if inputInvalid}
            <p class="error-text">{t("error.INVALID_MAC")}</p>
          {:else if info}
            {#each info.warnings as w (w)}
              {#if warningKeys[w]}<p class="warn-text">
                  <TriangleAlert size={14} aria-hidden="true" />{t(warningKeys[w]!)}
                </p>{/if}
            {/each}
          {/if}
          {#if app.info?.os === "windows" && adapter.kind === "wireless" && mode.kind !== "random"}
            <p class="subtle">{t("warning.universalMode")}</p>
          {/if}
          {#if adapter.nmConnection && adapter.methods[0] === "networkManager"}
            <p class="subtle">{t("dashboard.nmNote", { name: adapter.nmConnection.name })}</p>
          {:else if adapter.methods[0] === "iproute2"}
            <p class="subtle">{t("dashboard.temporary")}</p>
          {/if}
        </div>

        <div class="buttons">
          <button class="btn" onclick={randomize} disabled={anyBusy}>
            <Shuffle size={16} aria-hidden="true" />{t("dashboard.randomize")}<kbd
              >{t("dashboard.shortcutRandomize")}</kbd
            >
          </button>
          <button class="btn primary" onclick={requestApply} disabled={!canApply}>
            <Check size={16} aria-hidden="true" />{t("dashboard.apply")}<kbd
              >{t("dashboard.shortcutApply")}</kbd
            >
          </button>
          <span class="grow"></span>
          {#if adapter.originalMac && adapter.changed}
            <button
              class="btn ghost"
              onclick={() => onInput(formatMac(adapter.originalMac, app.format))}
              disabled={anyBusy}
            >
              {t("dashboard.useOriginal")}
            </button>
          {/if}
          <button
            class="btn"
            onclick={restore}
            disabled={anyBusy || app.readOnly || (!adapter.restorable && !adapter.changed)}
          >
            <Undo2 size={16} aria-hidden="true" />{t("dashboard.restore")}
          </button>
        </div>

        {#if busyHere}
          <ol class="steps" aria-live="polite" aria-label={t("common.loading")}>
            {#each app.busy?.stage === "rollingBack" ? [...stages, "rollingBack" as Stage] : stages as s (s)}
              {@const current = app.busy?.stage === s}
              {@const done = stages.indexOf(s) < stages.indexOf(app.busy?.stage ?? "preparing")}
              <li class:current class:done>
                {#if current}<Loader
                    size={14}
                    class="spin"
                    aria-hidden="true"
                  />{:else if done}<Check size={14} aria-hidden="true" />{/if}
                {t(`stage.${s}` as MessageKey)}
              </li>
            {/each}
          </ol>
        {:else if lastMessage && !restoring}
          <p class="result {lastMessage.kind}" role="status">{lastMessage.message}</p>
        {/if}
      </section>

      <section class="card" aria-labelledby="recent-title">
        <div class="card-head">
          <h2 id="recent-title" class="card-title">{t("dashboard.recent")}</h2>
          <button class="btn ghost small" onclick={() => app.navigate("history")}
            >{t("dashboard.viewAll")}</button
          >
        </div>
        {#if recent.length === 0}
          <p class="subtle">{t("dashboard.noRecent")}</p>
        {:else}
          <ul class="recent">
            {#each recent as h (h.id)}
              <li>
                <span
                  class="badge"
                  class:success={h.status === "applied"}
                  class:danger={h.status === "failed" || h.status === "rolledBack"}
                >
                  {t(`history.status.${h.status}` as MessageKey)}
                </span>
                <span class="mono"
                  >{formatMac(h.previousMac, app.format)} → {formatMac(
                    h.newMac ?? h.requestedMac,
                    app.format,
                  )}</span
                >
                <span class="subtle right"
                  >{t(`history.source.${h.source}` as MessageKey)} · {formatRelative(
                    h.timestamp,
                    i18n.locale,
                  )}</span
                >
              </li>
            {/each}
          </ul>
        {/if}
      </section>
    {/if}
  </div>
</div>

<ConfirmDialog
  open={confirmOpen}
  title={t("dashboard.confirmTitle")}
  body={adapter
    ? t("dashboard.confirmBody", {
        adapter: adapter.name,
        mac: formatMac(canonical(input), app.format),
      })
    : ""}
  confirmLabel={t("dashboard.apply")}
  onconfirm={apply}
  oncancel={() => (confirmOpen = false)}
/>

<style>
  .layout {
    display: grid;
    grid-template-columns: minmax(270px, 310px) minmax(0, 1fr);
    gap: 16px;
    align-items: start;
  }
  @media (max-width: 1000px) {
    .layout {
      grid-template-columns: 1fr;
    }
  }
  .adapters {
    position: sticky;
    top: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .adapter-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
    max-height: calc(100vh - 260px);
    overflow-y: auto;
  }
  .adapter-list li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px;
    border-radius: var(--radius);
    border: 1px solid transparent;
    cursor: pointer;
  }
  .adapter-list li:hover {
    background: var(--surface-2);
  }
  .adapter-list li.selected {
    background: var(--accent-soft);
    border-color: color-mix(in srgb, var(--accent) 45%, transparent);
  }
  .kind {
    flex: none;
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    border-radius: 9px;
    background: var(--surface-3);
    color: var(--text-muted);
  }
  .kind.wireless {
    color: var(--accent);
  }
  .kind.big {
    width: 42px;
    height: 42px;
    border-radius: 11px;
  }
  .info {
    display: flex;
    flex-direction: column;
    min-width: 0;
    flex: 1;
  }
  .name-row {
    display: flex;
    align-items: center;
    gap: 6px;
    min-width: 0;
  }
  .name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .desc {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .meta {
    display: flex;
    align-items: center;
    gap: 6px;
    margin-top: 2px;
  }
  .meta .mono {
    margin-left: auto;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--text-subtle);
  }
  .dot.on {
    background: var(--success);
  }
  .virtual-toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 8px;
    padding-top: 10px;
    border-top: 1px solid var(--border);
    color: var(--text-muted);
    cursor: pointer;
    font-size: 13px;
  }
  .detail {
    display: flex;
    flex-direction: column;
    gap: 16px;
    min-width: 0;
  }
  .adapter-head {
    display: flex;
    align-items: center;
    gap: 12px;
    margin-bottom: 14px;
  }
  .grow {
    flex: 1;
    min-width: 0;
  }
  .macs {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(196px, 1fr));
    gap: 10px;
    margin: 12px 0;
  }
  .facts {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px 16px;
    margin: 0;
    font-size: 13px;
  }
  .facts dt {
    color: var(--text-muted);
  }
  .facts dd {
    margin: 0;
    overflow-wrap: anywhere;
  }
  .editor {
    display: flex;
    gap: 10px;
    margin-top: 10px;
    flex-wrap: wrap;
  }
  .editor .mode {
    width: 180px;
  }
  .big-input {
    font-size: 16px;
    letter-spacing: 0.04em;
  }
  .help {
    min-height: 22px;
    margin: 8px 0 12px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 13px;
  }
  .error-text {
    color: var(--danger);
  }
  .warn-text {
    color: var(--warning);
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .buttons {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }
  .steps {
    list-style: none;
    display: flex;
    gap: 6px;
    padding: 0;
    margin: 14px 0 0;
    flex-wrap: wrap;
  }
  .steps li {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--text-subtle);
    font-size: 12.5px;
  }
  .steps li.current {
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }
  .steps li.done {
    color: var(--success);
  }
  .result {
    margin-top: 14px;
    padding: 10px 12px;
    border-radius: var(--radius);
    background: var(--surface-2);
  }
  .result.success {
    background: var(--success-soft);
    color: var(--success);
  }
  .result.error {
    background: var(--danger-soft);
    color: var(--danger);
  }
  .result.warning {
    background: var(--warning-soft);
    color: var(--warning);
  }
  .recent {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }
  .recent li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 0;
    border-top: 1px solid var(--border);
    font-size: 13px;
  }
  .recent li:first-child {
    border-top: none;
  }
  .right {
    margin-left: auto;
    white-space: nowrap;
  }
  .empty,
  .empty-detail {
    padding: 24px 8px;
    text-align: center;
  }
  .stack-sm {
    display: flex;
    flex-direction: column;
    gap: 8px;
    align-items: flex-start;
  }
  :global(.spin) {
    animation: spin 1s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
