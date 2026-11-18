<script lang="ts">
  import { onMount } from "svelte";
  import { Copy, Gauge, Loader, RefreshCw, Save, Square, Trash2, Undo2, Zap } from "@lucide/svelte";
  import PageHeader from "../components/PageHeader.svelte";
  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import Steps from "../components/Steps.svelte";
  import { rescueFocus } from "../focus";
  import { app } from "../stores/app.svelte";
  import { api, events, toApiError } from "../api";
  import { i18n, t, type MessageKey } from "../i18n/index.svelte";
  import * as dns from "../dns";
  import type { CustomDnsPreset, DnsRequest, DnsView, EncryptionChoice } from "../types";

  type Source = "provider" | "preset" | "custom" | "localResolver" | "automatic";

  let view = $state<DnsView | null>(null);
  let viewError = $state<string | null>(null);
  let loadedFor = "";

  let source = $state<Source>("provider");
  let providerId = $state("");
  let variantId = $state("");
  let presetId = $state("");
  let custom = $state({ ipv4: "", ipv6: "", doh: "", dot: "" });
  let encryption = $state<EncryptionChoice>(
    app.settings?.dns.preferEncryption === false ? "none" : "preferred",
  );

  let message = $state<{ kind: string; message: string; notes: string[] } | null>(null);
  let checking = $state(false);
  let confirmAll = $state(false);
  let pending = $state<{ all: boolean; request: DnsRequest } | null>(null);

  let presetName = $state("");
  let presetTried = $state(false);
  let presetNote = $state("");

  let rows = $state<dns.LatencyRow[]>([]);
  let running = $state(false);
  let includeIpv6 = $state(false);
  let latencyStatus = $state("");

  const adapter = $derived(app.selected);
  const adapters = $derived(app.visibleAdapters);
  const catalog = $derived(app.dnsCatalog);
  const presets = $derived(app.settings?.dns.presets ?? []);
  const allowFallback = $derived(app.settings?.dns.allowFallback ?? false);
  const caps = $derived(view?.capabilities);
  const canChange = $derived(!!caps?.supported && !!caps.canChange && !app.readOnly);

  const provider = $derived(catalog?.providers.find((p) => p.id === providerId) ?? null);
  const found = $derived(dns.findVariant(catalog, providerId, variantId || undefined));
  const selection = $derived.by<dns.Selection>(() => {
    switch (source) {
      case "provider":
        return { kind: "provider", provider: providerId, variant: found?.variant.id ?? variantId };
      case "preset":
        return { kind: "preset", id: presetId };
      case "custom":
        return { kind: "custom", ...custom };
      default:
        return { kind: source };
    }
  });
  const endpoints = $derived(dns.endpointsFor(selection, catalog, presets));
  const options = $derived(dns.encryptionOptions(caps, endpoints));
  const choice = $derived(dns.keepChoice(encryption, options));
  const mode = $derived(dns.resolvedEncryption(choice, caps, endpoints));
  const usable = $derived(dns.selectionUsable(selection, catalog, presets));
  const customErrors = $derived(source === "custom" ? dns.customProblems(custom) : {});
  const presetErrors = $derived(dns.presetProblems({ name: presetName, ...custom }, presets));

  const busyHere = $derived(app.busy?.kind === "dns" && app.busy.adapterId === adapter?.id);
  const anyBusy = $derived(!!app.busy || checking);
  const configured = $derived(view?.status?.configured);
  const effective = $derived([
    ...(view?.status?.effectiveIpv4 ?? []),
    ...(view?.status?.effectiveIpv6 ?? []),
  ]);
  const sortedRows = $derived(running ? rows : dns.sortLatency(rows));

  onMount(() => {
    void app.loadDnsCatalog().then((c) => {
      if (c && !providerId) providerId = c.providers[0]?.id ?? "";
    });
  });

  // Reload when the adapter changes and after every DNS change (UI, tray, another window).
  $effect(() => {
    const id = adapter?.id;
    void app.dnsRevision;
    if (id) void refresh(id);
  });

  async function refresh(id: string) {
    try {
      const next = await api.dnsView(id);
      if (adapter?.id !== id) return;
      view = next;
      viewError = null;
      if (loadedFor !== id) {
        loadedFor = id;
        message = null;
        prefill(next);
      }
    } catch (e) {
      view = null;
      viewError = i18n.error(toApiError(e).code);
    }
  }

  /** Starts from what the adapter uses: its provider, preset or servers. */
  function prefill(v: DnsView) {
    const sel = dns.selectionForLabel(v.matched, presets);
    if (sel?.kind === "provider") {
      source = "provider";
      providerId = sel.provider;
      variantId = sel.variant;
    } else if (sel?.kind === "preset") {
      source = "preset";
      presetId = sel.id;
    } else if (sel?.kind === "localResolver") {
      source = "localResolver";
    } else if (!sel && v.status) {
      source = "custom";
      custom.ipv4 = dns.familyServers(v.status.configured.ipv4)?.join(", ") ?? "";
      custom.ipv6 = dns.familyServers(v.status.configured.ipv6)?.join(", ") ?? "";
      custom.doh = v.status.dohTemplate ?? "";
      custom.dot = v.status.dotServerName ?? "";
    }
  }

  function chooseProvider(id: string) {
    providerId = id;
    const p = catalog?.providers.find((x) => x.id === id);
    variantId = p ? (dns.defaultVariant(p)?.id ?? "") : "";
  }

  function chooseEncryption(next: EncryptionChoice) {
    encryption = next;
    const prefer = next !== "none";
    if (app.settings && app.settings.dns.preferEncryption !== prefer) {
      void app.saveSettings((s) => (s.dns.preferEncryption = prefer));
    }
  }

  function setFallback(on: boolean) {
    void app.saveSettings((s) => (s.dns.allowFallback = on));
  }

  function problemText(p: dns.ServerProblem): string {
    return "value" in p
      ? t(`dns.err.${p.code}` as MessageKey, { value: p.value })
      : t("dns.err.tooMany");
  }

  function optionLabel(c: EncryptionChoice): string {
    return t(`dns.encryption.${c}` as MessageKey);
  }

  async function copy(value: string) {
    try {
      await navigator.clipboard.writeText(value);
      const shown = value.length > 42 ? `${value.slice(0, 40)}…` : value;
      app.toast("success", t("common.copied", { value: shown }));
    } catch (e) {
      app.toastError(e);
    }
  }

  /** Applies the selection; plain DNS is first checked to answer from this computer. */
  async function apply(all: boolean) {
    confirmAll = false;
    if (!adapter || !usable || !canChange || anyBusy) return;
    const request = dns.requestFor(selection, choice, allowFallback);
    if (endpoints && mode === "none" && !app.info?.mock) {
      checking = true;
      const servers = [...endpoints.ipv4, ...endpoints.ipv6];
      const results = await api.dnsLatencyTest(servers, 1).catch(() => null);
      checking = false;
      if (results && !results.some((r) => r.outcome.kind === "answered")) {
        pending = { all, request };
        return;
      }
    }
    await run(all, request);
  }

  async function run(all: boolean, request: DnsRequest) {
    pending = null;
    message = null;
    const id = adapter?.id;
    if (!id) return;
    try {
      if (all) {
        message = { ...dns.describeAll(await api.dnsApplyAll(request)), notes: [] };
      } else {
        message = dns.describeDnsOutcome(await api.dnsApply(id, request), false);
      }
    } catch (e) {
      message = { kind: "error", message: i18n.error(toApiError(e).code), notes: [] };
    }
    await refresh(id);
  }

  async function restore() {
    const id = adapter?.id;
    if (!id || anyBusy || !view?.restorable) return;
    message = null;
    try {
      message = dns.describeDnsOutcome(await api.dnsRestore(id), true);
    } catch (e) {
      message = { kind: "error", message: i18n.error(toApiError(e).code), notes: [] };
    }
    await refresh(id);
  }

  async function savePreset() {
    presetTried = true;
    presetNote = "";
    const draft = { name: presetName, ...custom };
    if (dns.hasProblems(dns.presetProblems(draft, presets))) return;
    const preset = dns.presetFromDraft(draft, dns.newPresetId());
    const ok = await app.saveSettings((s) => {
      s.dns.presets = dns.upsertPreset(s.dns.presets, preset);
    });
    if (ok) {
      presetNote = t("dns.presets.saved", { name: preset.name });
      presetName = "";
      presetTried = false;
      source = "preset";
      presetId = preset.id;
    }
  }

  async function deletePreset(p: CustomDnsPreset) {
    const ok = await app.saveSettings((s) => {
      s.dns.presets = dns.removePreset(s.dns.presets, p.id);
    });
    if (ok) {
      presetNote = t("dns.presets.deleted", { name: p.name });
      if (presetId === p.id) presetId = "";
      void rescueFocus(() => document.querySelector<HTMLElement>("[data-preset-delete]"));
    }
  }

  function usePreset(p: CustomDnsPreset) {
    source = "preset";
    presetId = p.id;
  }

  async function runLatency() {
    rows = dns.latencyRows(catalog, includeIpv6);
    if (rows.length === 0) return;
    running = true;
    let done = 0;
    latencyStatus = t("dns.latency.running", { done, total: rows.length });
    const unlisten = await events.onLatency((e) => {
      const row = rows[e.index];
      if (row && row.addr === e.addr) {
        row.outcome = e.outcome;
        done += 1;
        latencyStatus = t("dns.latency.running", { done, total: rows.length });
      }
    });
    try {
      const results = await api.dnsLatencyTest(
        rows.map((r) => r.addr),
        3,
      );
      results.forEach((r, i) => {
        const row = rows[i];
        if (row) row.outcome = r.outcome;
      });
    } catch (e) {
      app.toastError(e);
    } finally {
      unlisten();
      running = false;
    }
    const sorted = dns.sortLatency(rows);
    const best = sorted[0];
    const answered = rows.filter((r) => r.outcome?.kind === "answered").length;
    latencyStatus =
      best?.outcome?.kind === "answered"
        ? t("dns.latency.done", {
            answered,
            total: rows.length,
            name: best.name,
            ms: dns.formatMs(best.outcome.stats.medianUs, i18n.locale),
          })
        : t("dns.latency.noneAnswered");
  }

  function useRow(row: dns.LatencyRow) {
    source = "provider";
    providerId = row.providerId;
    variantId = row.variantId;
  }
</script>

<PageHeader title={t("dns.title")} subtitle={t("dns.subtitle")}>
  {#snippet actions()}
    <button class="btn" onclick={() => adapter && refresh(adapter.id)} disabled={!adapter}>
      <RefreshCw size={16} aria-hidden="true" />{t("common.refresh")}
    </button>
  {/snippet}
</PageHeader>

{#if adapters.length === 0}
  <p class="card muted">{t("dns.noAdapters")}</p>
{:else}
  <div class="stack">
    <section class="card" aria-labelledby="dns-current-title">
      <div class="card-head">
        <h2 id="dns-current-title" class="card-title">{t("dns.current.title")}</h2>
        <div class="adapter-pick">
          <label for="dns-adapter" class="label">{t("dns.adapter")}</label>
          <select
            id="dns-adapter"
            class="select"
            value={adapter?.id ?? ""}
            onchange={(e) => app.select((e.currentTarget as HTMLSelectElement).value)}
          >
            {#each adapters as a (a.id)}
              <option value={a.id}>{a.name} — {a.description}</option>
            {/each}
          </select>
        </div>
      </div>

      {#if viewError}
        <p class="callout danger">{t("dns.readError", { error: viewError })}</p>
      {:else if view && !view.capabilities.supported}
        <p class="callout warning">
          {t("dns.unsupported", { reason: dns.reasonText(view.capabilities.reason) })}
        </p>
      {:else if view}
        {#if view.statusErrorCode}
          <p class="callout danger">
            {t("dns.readError", { error: i18n.error(view.statusErrorCode) })}
          </p>
        {/if}
        <dl class="facts">
          <dt>{t("dns.current.configured")}</dt>
          <dd class="mono">{dns.summaryText(configured)}</dd>
          <dt>{t("dns.current.inUse")}</dt>
          <dd class="mono">{effective.length > 0 ? effective.join(", ") : "—"}</dd>
          <dt>{t("dns.current.encryption")}</dt>
          <dd>
            {#if configured?.encryption === "doh" || configured?.encryption === "dot"}
              {view.status?.fallback
                ? t("dns.current.fallback", { mode: dns.modeName(configured.encryption) })
                : dns.modeName(configured.encryption)}
              {#if view.status?.dohTemplate}
                <span class="subtle mono"
                  >{t("dns.current.template", { value: view.status.dohTemplate })}</span
                >
              {/if}
              {#if view.status?.dotServerName}
                <span class="subtle mono"
                  >{t("dns.current.serverName", { value: view.status.dotServerName })}</span
                >
              {/if}
            {:else}
              {t("common.none")}
            {/if}
          </dd>
          <dt>{t("dns.current.matches")}</dt>
          <dd>{dns.labelText(view.matched, catalog)}</dd>
        </dl>
        {#if view.status?.profileOverride}
          <p class="callout warning">{t("dns.profileOverride")}</p>
        {/if}
        {#if view.restorable}
          <p class="subtle">{t("dns.current.restorable")}</p>
        {/if}
        {#if !canChange}
          <p class="callout info">
            {t("dns.viewOnly", {
              reason: dns.reasonText(view.capabilities.changeReason ?? "NOT_ELEVATED"),
            })}
          </p>
        {/if}
      {:else}
        <p class="muted" aria-live="polite">{t("common.loading")}</p>
      {/if}
    </section>

    {#if view?.capabilities.supported}
      <section class="card" aria-labelledby="dns-servers-title">
        <h2 id="dns-servers-title">{t("dns.source.title")}</h2>
        <fieldset class="sources">
          <legend class="sr-only">{t("dns.source.legend")}</legend>
          {#each ["provider", "preset", "custom", "localResolver", "automatic"] as const as s (s)}
            <label class="radio">
              <input type="radio" name="dns-source" value={s} bind:group={source} />
              {t(`dns.source.${s}` as MessageKey)}
            </label>
          {/each}
        </fieldset>

        {#if source === "provider"}
          <div class="pick">
            <div class="field">
              <label for="dns-provider">{t("dns.provider")}</label>
              <select
                id="dns-provider"
                class="select"
                value={providerId}
                onchange={(e) => chooseProvider((e.currentTarget as HTMLSelectElement).value)}
              >
                {#each catalog?.providers ?? [] as p (p.id)}
                  <option value={p.id}>{p.name}</option>
                {/each}
              </select>
            </div>
            <div class="field">
              <label for="dns-variant">{t("dns.variant")}</label>
              <select
                id="dns-variant"
                class="select"
                value={found?.variant.id ?? ""}
                onchange={(e) => (variantId = (e.currentTarget as HTMLSelectElement).value)}
              >
                {#each provider?.variants ?? [] as v (v.id)}
                  <option value={v.id}>{dns.variantText(v)}</option>
                {/each}
              </select>
            </div>
          </div>
          {#if found}
            <h3 class="sub">{t("dns.endpoints.title")}</h3>
            <dl class="endpoints">
              {#each [["IPv4", found.variant.ipv4], ["IPv6", found.variant.ipv6]] as const as [label, list] (label)}
                {#if list.length > 0}
                  <dt>{label}</dt>
                  <dd>
                    {#each list as ip (ip)}
                      <span class="value">
                        <span class="mono">{ip}</span>
                        <button
                          class="btn ghost icon small"
                          aria-label={t("dns.endpoints.copy", { what: ip })}
                          title={t("common.copy")}
                          onclick={() => copy(ip)}><Copy size={14} aria-hidden="true" /></button
                        >
                      </span>
                    {/each}
                  </dd>
                {/if}
              {/each}
              {#each [["DoH", found.variant.doh], ["DoT", found.variant.dot]] as const as [label, value] (label)}
                {#if value}
                  <dt>{label}</dt>
                  <dd>
                    <span class="value">
                      <span class="mono">{value}</span>
                      <button
                        class="btn ghost icon small"
                        aria-label={t("dns.endpoints.copy", { what: `${label} ${value}` })}
                        title={t("common.copy")}
                        onclick={() => copy(value)}><Copy size={14} aria-hidden="true" /></button
                      >
                    </span>
                  </dd>
                {/if}
              {/each}
              {#if found.variant.dnscrypt.length > 0}
                <dt>{t("dns.endpoints.dnscrypt")}</dt>
                <dd>
                  <ul class="stamps">
                    {#each found.variant.dnscrypt as stamp (stamp.stamp)}
                      <li>
                        <span>{stamp.name} <span class="subtle mono">{stamp.address}</span></span>
                        <button
                          class="btn small"
                          aria-label={t("dns.endpoints.copy", {
                            what: `${stamp.name} (${stamp.address})`,
                          })}
                          onclick={() => copy(stamp.stamp)}
                          ><Copy size={14} aria-hidden="true" />{t("common.copy")}</button
                        >
                      </li>
                    {/each}
                  </ul>
                  <p class="subtle">{t("dns.endpoints.dnscryptHint")}</p>
                </dd>
              {/if}
            </dl>
            <p class="subtle small">
              {t("dns.endpoints.verified", {
                provider: found.provider.name,
                date: found.provider.verifiedOn,
              })}
            </p>
          {/if}
        {:else if source === "preset"}
          {#if presets.length === 0}
            <p class="muted">{t("dns.noPresets")}</p>
          {:else}
            <div class="field narrow">
              <label for="dns-preset">{t("dns.preset")}</label>
              <select id="dns-preset" class="select" bind:value={presetId}>
                <option value="" disabled>—</option>
                {#each presets as p (p.id)}
                  <option value={p.id}>{p.name}</option>
                {/each}
              </select>
            </div>
          {/if}
        {:else if source === "custom"}
          <div class="pick">
            {#each [["ipv4", "dns.custom.ipv4"], ["ipv6", "dns.custom.ipv6"]] as const as [f, key] (f)}
              {@const problem = customErrors[f]}
              <div class="field">
                <label for="dns-{f}">{t(key)}</label>
                <input
                  id="dns-{f}"
                  class="input mono"
                  bind:value={custom[f]}
                  spellcheck="false"
                  autocomplete="off"
                  aria-invalid={problem ? "true" : undefined}
                  aria-describedby="dns-{f}-hint{problem ? ` dns-${f}-error` : ''}"
                />
                <p id="dns-{f}-hint" class="subtle small">{t("dns.custom.serversHint")}</p>
                {#if problem}
                  <p id="dns-{f}-error" class="error-text small">{problemText(problem)}</p>
                {/if}
              </div>
            {/each}
            <div class="field">
              <label for="dns-doh">{t("dns.custom.doh")}</label>
              <input
                id="dns-doh"
                class="input mono"
                bind:value={custom.doh}
                spellcheck="false"
                autocomplete="off"
                placeholder="https://dns.example/dns-query"
                aria-invalid={customErrors.doh ? "true" : undefined}
                aria-describedby={customErrors.doh ? "dns-doh-error" : undefined}
              />
              {#if customErrors.doh}
                <p id="dns-doh-error" class="error-text small">
                  {customErrors.doh === "ipMismatch" ? t("dns.err.dohIp") : t("dns.err.dohBad")}
                </p>
              {/if}
            </div>
            <div class="field">
              <label for="dns-dot">{t("dns.custom.dot")}</label>
              <input
                id="dns-dot"
                class="input mono"
                bind:value={custom.dot}
                spellcheck="false"
                autocomplete="off"
                placeholder="dns.example"
                aria-invalid={customErrors.dot ? "true" : undefined}
                aria-describedby={customErrors.dot ? "dns-dot-error" : undefined}
              />
              {#if customErrors.dot}
                <p id="dns-dot-error" class="error-text small">{t("dns.err.dotBad")}</p>
              {/if}
            </div>
          </div>
          {#if customErrors.empty && (custom.ipv4 !== "" || custom.ipv6 !== "")}
            <p class="error-text small">{t("dns.err.empty")}</p>
          {/if}
          <div class="row save-preset">
            <div class="field">
              <label for="dns-preset-name">{t("dns.presets.name")}</label>
              <input
                id="dns-preset-name"
                class="input"
                maxlength="40"
                bind:value={presetName}
                aria-invalid={presetTried && presetErrors.name ? "true" : undefined}
                aria-describedby={presetTried && (presetErrors.name || presetErrors.limit)
                  ? "dns-preset-error"
                  : undefined}
              />
            </div>
            <button class="btn" onclick={savePreset} disabled={!usable}>
              <Save size={16} aria-hidden="true" />{t("dns.presets.save")}
            </button>
          </div>
          {#if presetTried && (presetErrors.name || presetErrors.limit)}
            <p id="dns-preset-error" class="error-text small">
              {presetErrors.limit
                ? t("dns.err.presetLimit")
                : presetErrors.name === "taken"
                  ? t("dns.err.presetTaken")
                  : t("dns.err.presetName")}
            </p>
          {/if}
        {:else if source === "localResolver"}
          <p class="muted">{t("dns.source.localResolverHint")}</p>
        {/if}
      </section>

      <section class="card" aria-labelledby="dns-encryption-title">
        <h2 id="dns-encryption-title">{t("dns.encryption.title")}</h2>
        <fieldset class="options">
          <legend class="sr-only">{t("dns.encryption.title")}</legend>
          {#each options as o (o.choice)}
            <div class="option">
              <label class="radio" class:disabled={!!o.unavailable}>
                <input
                  type="radio"
                  name="dns-encryption"
                  value={o.choice}
                  checked={choice === o.choice}
                  disabled={!!o.unavailable}
                  aria-describedby={o.unavailable ? `dns-enc-${o.choice}-why` : undefined}
                  onchange={() => chooseEncryption(o.choice)}
                />
                {optionLabel(o.choice)}
              </label>
              {#if o.unavailable}
                <span id="dns-enc-{o.choice}-why" class="subtle small"
                  >{dns.unavailableText(o.unavailable)}</span
                >
              {/if}
            </div>
          {/each}
        </fieldset>
        {#if endpoints}
          <p class="subtle">{t("dns.encryption.resolved", { mode: dns.modeName(mode) })}</p>
        {/if}
        <label class="check">
          <input
            type="checkbox"
            checked={allowFallback}
            disabled={mode === "none"}
            aria-describedby="dns-fallback-hint"
            onchange={(e) => setFallback((e.currentTarget as HTMLInputElement).checked)}
          />
          {t("dns.encryption.fallback")}
        </label>
        <p id="dns-fallback-hint" class="subtle small">{t("dns.encryption.fallbackHint")}</p>
        <p class="subtle small">{t("dns.encryption.defaultsHint")}</p>
      </section>

      <section class="card" aria-label={t("dns.apply")}>
        <div class="buttons">
          <button
            class="btn primary"
            onclick={() => apply(false)}
            disabled={!canChange}
            aria-disabled={!usable || anyBusy}
          >
            <Zap size={16} aria-hidden="true" />{t("dns.apply")}
          </button>
          <button
            class="btn"
            onclick={() => usable && !anyBusy && (confirmAll = true)}
            disabled={!canChange}
            aria-disabled={!usable || anyBusy}
          >
            {t("dns.applyAll")}
          </button>
          <button
            class="btn end"
            onclick={restore}
            disabled={!canChange}
            aria-disabled={!view?.restorable || anyBusy}
            title={t("dns.restoreHint")}
          >
            <Undo2 size={16} aria-hidden="true" />{t("dns.restore")}
          </button>
        </div>
        {#if checking}
          <p class="subtle" role="status">
            <Loader size={14} class="spin" aria-hidden="true" />
            {t("dns.checking")}
          </p>
        {/if}
        <Steps
          stage={app.busy?.stage ?? null}
          active={busyHere}
          prefix="dns.stage"
          label={t("dns.progress")}
        />
        <div role="status">
          {#if message && !busyHere}
            <div class="result {message.kind}">
              <p>{message.message}</p>
              {#each message.notes as note (note)}<p class="small">{note}</p>{/each}
            </div>
          {/if}
        </div>
      </section>
    {/if}

    <section class="card" aria-labelledby="dns-latency-title">
      <div class="card-head">
        <div>
          <h2 id="dns-latency-title" class="card-title">{t("dns.latency.title")}</h2>
          <p class="subtle">{t("dns.latency.subtitle")}</p>
        </div>
        <div class="row">
          <label class="check">
            <input type="checkbox" bind:checked={includeIpv6} disabled={running} />
            {t("dns.latency.ipv6")}
          </label>
          {#if running}
            <button class="btn" onclick={() => api.dnsLatencyCancel().catch(() => {})}>
              <Square size={16} aria-hidden="true" />{t("dns.latency.stop")}
            </button>
          {:else}
            <button class="btn" onclick={runLatency} disabled={!catalog}>
              <Gauge size={16} aria-hidden="true" />{t("dns.latency.run")}
            </button>
          {/if}
        </div>
      </div>
      <p class="sr-only" role="status">{latencyStatus}</p>
      {#if rows.length > 0}
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
        <div class="table-wrap" role="region" aria-label={t("dns.latency.title")} tabindex="0">
          <table class="table">
            <caption class="sr-only">{t("dns.latency.title")}</caption>
            <thead>
              <tr>
                <th scope="col">{t("dns.latency.col.provider")}</th>
                <th scope="col">{t("dns.latency.col.server")}</th>
                <th scope="col">{t("dns.latency.col.median")}</th>
                <th scope="col">{t("dns.latency.col.answers")}</th>
                <th scope="col"><span class="sr-only">{t("dns.apply")}</span></th>
              </tr>
            </thead>
            <tbody>
              {#each sortedRows as row (row.key)}
                <tr>
                  <td>{row.name}</td>
                  <td class="mono">{row.addr}</td>
                  <td class="mono nowrap">{dns.outcomeText(row.outcome, i18n.locale)}</td>
                  <td class="subtle">
                    {row.outcome?.kind === "answered"
                      ? `${row.outcome.answered}/${row.outcome.attempts}`
                      : "—"}
                  </td>
                  <td>
                    <button
                      class="btn small"
                      onclick={() => useRow(row)}
                      aria-label={t("dns.latency.use", { name: `${row.name} (${row.addr})` })}
                      >{t("dns.presets.useShort")}</button
                    >
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>
      {/if}
    </section>

    <section class="card" aria-labelledby="dns-presets-title">
      <h2 id="dns-presets-title">{t("dns.presets.title")}</h2>
      <p class="subtle">{t("dns.presets.subtitle")}</p>
      <p class="sr-only" role="status">{presetNote}</p>
      {#if presets.length === 0}
        <p class="muted">{t("dns.presets.empty")}</p>
      {:else}
        <ul class="presets">
          {#each presets as p (p.id)}
            <li>
              <div class="grow">
                <strong>{p.name}</strong>
                <span class="subtle mono">{[...p.ipv4, ...p.ipv6].join(", ")}</span>
              </div>
              <button
                class="btn small"
                onclick={() => usePreset(p)}
                aria-label={t("dns.presets.use", { name: p.name })}
                >{t("dns.presets.useShort")}</button
              >
              <button
                class="btn ghost icon small"
                onclick={() => deletePreset(p)}
                data-preset-delete
                aria-label={t("dns.presets.delete", { name: p.name })}
                title={t("common.remove")}><Trash2 size={14} aria-hidden="true" /></button
              >
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  </div>
{/if}

<ConfirmDialog
  open={confirmAll}
  title={t("dns.confirmAll.title")}
  body={t("dns.confirmAll.body", {
    name: dns.labelText(dns.labelForSelection(selection, presets), catalog),
  })}
  confirmLabel={t("dns.applyAll")}
  onconfirm={() => apply(true)}
  oncancel={() => (confirmAll = false)}
/>

<ConfirmDialog
  open={pending !== null}
  title={t("dns.check.title")}
  body={t("dns.check.body")}
  confirmLabel={t("dns.check.apply")}
  danger
  onconfirm={() => pending && run(pending.all, pending.request)}
  oncancel={() => (pending = null)}
/>

<style>
  .adapter-pick {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .adapter-pick .select {
    width: auto;
    max-width: 420px;
  }
  .facts {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px 16px;
    margin: 4px 0 8px;
  }
  .facts dt {
    color: var(--text-muted);
  }
  .facts dd {
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 4px 10px;
    min-width: 0;
    overflow-wrap: anywhere;
  }
  .sources,
  .options {
    border: none;
    padding: 0;
    margin: 8px 0 12px;
    display: flex;
    flex-wrap: wrap;
    gap: 8px 18px;
  }
  .options {
    flex-direction: column;
    gap: 6px;
  }
  .option {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 4px 12px;
  }
  .radio,
  .check {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    cursor: pointer;
  }
  .radio.disabled,
  .check:has(input:disabled) {
    color: var(--text-subtle);
    cursor: default;
  }
  .pick {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: 12px 16px;
  }
  .narrow {
    max-width: 360px;
  }
  .sub {
    font-size: 13px;
    font-weight: 600;
    margin: 16px 0 6px;
  }
  .endpoints {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: 6px 16px;
    margin: 0 0 8px;
  }
  .endpoints dt {
    color: var(--text-muted);
  }
  .endpoints dd {
    margin: 0;
    display: flex;
    flex-wrap: wrap;
    gap: 4px 14px;
    min-width: 0;
  }
  .value {
    display: inline-flex;
    align-items: center;
    gap: 2px;
    overflow-wrap: anywhere;
  }
  .stamps {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 6px;
    width: 100%;
  }
  .stamps li {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
  }
  .save-preset {
    margin-top: 14px;
    align-items: flex-end;
  }
  .save-preset .field {
    flex: 1;
    min-width: 200px;
    max-width: 360px;
  }
  .small {
    font-size: 12.5px;
  }
  .error-text {
    color: var(--danger);
  }
  .buttons {
    display: flex;
    gap: 8px;
    align-items: center;
    flex-wrap: wrap;
  }
  .buttons .end {
    margin-left: auto;
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
  .table-wrap {
    overflow: auto;
    margin-top: 10px;
  }
  .nowrap {
    white-space: nowrap;
  }
  .presets {
    list-style: none;
    margin: 8px 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }
  .presets li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 0;
  }
  .presets li + li {
    border-top: 1px solid var(--border);
  }
  .grow {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
    overflow-wrap: anywhere;
  }
</style>
