<script lang="ts">
  import { Copy, Download, Trash2, FolderOpen } from "@lucide/svelte";
  import { onDestroy, onMount, tick } from "svelte";
  import PageHeader from "../components/PageHeader.svelte";
  import { app } from "../stores/app.svelte";
  import { api } from "../api";
  import { t, type MessageKey } from "../i18n/index.svelte";
  import { formatTime } from "../format";
  import type { LogEntry } from "../types";

  const MAX = 2000;
  const levels = ["error", "warn", "info", "debug"] as const;

  let entries = $state<LogEntry[]>([]);
  let enabled = $state<Record<string, boolean>>({
    error: true,
    warn: true,
    info: true,
    debug: false,
  });
  let query = $state("");
  let autoScroll = $state(true);
  let list = $state<HTMLElement | null>(null);
  let timer: ReturnType<typeof setInterval> | undefined;
  let lastSeq: number | null = null;

  const filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return entries.filter(
      (e) =>
        (enabled[e.level] ?? (e.level === "trace" && enabled.debug)) &&
        (!q || e.message.toLowerCase().includes(q) || e.target.toLowerCase().includes(q)),
    );
  });

  async function poll() {
    try {
      const fresh = await api.logsGet(lastSeq);
      if (fresh.length) {
        lastSeq = fresh[fresh.length - 1]!.seq;
        entries = [...entries, ...fresh].slice(-MAX);
        if (autoScroll) {
          await tick();
          list?.scrollTo({ top: list.scrollHeight });
        }
      }
    } catch {
      /* keep polling */
    }
  }

  onMount(() => {
    void poll();
    timer = setInterval(poll, 1000);
  });
  onDestroy(() => clearInterval(timer));

  function lineText(e: LogEntry) {
    return `${e.timestamp} ${e.level.toUpperCase().padEnd(5)} ${e.target}: ${e.message}`;
  }

  async function copyAll() {
    try {
      await navigator.clipboard.writeText(filtered.map(lineText).join("\n"));
      app.toast("success", t("common.copied", { value: `${filtered.length}` }));
    } catch (e) {
      app.toastError(e);
    }
  }

  async function exportLog() {
    try {
      const path = await api.logsExport();
      if (path) app.toast("success", t("common.savedTo", { path }));
    } catch (e) {
      app.toastError(e);
    }
  }

  async function clearLog() {
    await api.logsClear().catch((e) => app.toastError(e));
    entries = [];
  }
</script>

<PageHeader title={t("logs.title")} subtitle={t("logs.subtitle")}>
  {#snippet actions()}
    <button class="btn" onclick={copyAll} disabled={filtered.length === 0}
      ><Copy size={16} aria-hidden="true" />{t("logs.copy")}</button
    >
    <button class="btn" onclick={exportLog}
      ><Download size={16} aria-hidden="true" />{t("common.export")}</button
    >
    <button class="btn" onclick={() => api.openLocation("logs").catch((e) => app.toastError(e))}
      ><FolderOpen size={16} aria-hidden="true" />{t("common.openFolder")}</button
    >
    <button class="btn danger" onclick={clearLog}
      ><Trash2 size={16} aria-hidden="true" />{t("logs.clear")}</button
    >
  {/snippet}
</PageHeader>

<section class="card log-card">
  <div class="toolbar">
    <div class="chips" role="group" aria-label="Levels">
      {#each levels as l (l)}
        <button
          class="chip {l}"
          aria-pressed={enabled[l]}
          onclick={() => (enabled[l] = !enabled[l])}
        >
          {t(`logs.level.${l}` as MessageKey)}
        </button>
      {/each}
    </div>
    <label class="sr-only" for="log-filter">{t("logs.filter")}</label>
    <input id="log-filter" class="input filter" placeholder={t("logs.filter")} bind:value={query} />
    <label class="auto">
      <input type="checkbox" bind:checked={autoScroll} />
      {t("logs.autoScroll")}
    </label>
  </div>

  <!-- The scrollable log must be focusable so keyboard users can scroll it. -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div class="lines mono" bind:this={list} role="log" aria-live="off" tabindex="0">
    {#if filtered.length === 0}
      <p class="empty muted">{t("logs.empty")}</p>
    {:else}
      {#each filtered as e (e.seq)}
        <div class="line {e.level}">
          <span class="time">{formatTime(e.timestamp)}</span>
          <span class="lvl">{e.level.toUpperCase()}</span>
          <span class="msg"><span class="target">{e.target}</span> {e.message}</span>
        </div>
      {/each}
    {/if}
  </div>
  <p class="subtle count">{t("logs.count", { shown: filtered.length, total: entries.length })}</p>
</section>

<style>
  .log-card {
    display: flex;
    flex-direction: column;
    gap: 10px;
    height: calc(100vh - 150px);
    min-height: 320px;
  }
  .toolbar {
    display: flex;
    gap: 10px;
    align-items: center;
    flex-wrap: wrap;
  }
  .chips {
    display: flex;
    gap: 6px;
  }
  .chip {
    border: 1px solid var(--border);
    background: transparent;
    border-radius: 999px;
    padding: 3px 12px;
    cursor: pointer;
    color: var(--text-muted);
    font-size: 12.5px;
  }
  .chip[aria-pressed="true"] {
    background: var(--accent-soft);
    border-color: var(--accent);
    color: var(--accent);
  }
  .chip.error[aria-pressed="true"] {
    background: var(--danger-soft);
    border-color: var(--danger);
    color: var(--danger);
  }
  .chip.warn[aria-pressed="true"] {
    background: var(--warning-soft);
    border-color: var(--warning);
    color: var(--warning);
  }
  .filter {
    flex: 1;
    min-width: 200px;
  }
  .auto {
    display: flex;
    gap: 6px;
    align-items: center;
    color: var(--text-muted);
  }
  .lines {
    flex: 1;
    overflow: auto;
    background: var(--surface-2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 8px 10px;
    font-size: 12.5px;
  }
  .line {
    display: grid;
    grid-template-columns: 96px 52px 1fr;
    gap: 10px;
    padding: 1px 0;
    white-space: pre-wrap;
    word-break: break-word;
    user-select: text;
  }
  .time {
    color: var(--text-subtle);
  }
  .lvl {
    font-weight: 700;
    color: var(--text-muted);
  }
  .line.error .lvl,
  .line.error .msg {
    color: var(--danger);
  }
  .line.warn .lvl {
    color: var(--warning);
  }
  .line.debug .msg,
  .line.trace .msg {
    color: var(--text-muted);
  }
  .target {
    color: var(--text-subtle);
  }
  .empty {
    padding: 24px;
    text-align: center;
    font-family: var(--font-ui);
  }
  .count {
    text-align: right;
  }
</style>
