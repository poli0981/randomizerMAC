<script lang="ts">
  import { Download, Trash2, Search } from "@lucide/svelte";
  import PageHeader from "../components/PageHeader.svelte";
  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import { app } from "../stores/app.svelte";
  import { api } from "../api";
  import { i18n, t, type MessageKey } from "../i18n/index.svelte";
  import { formatDateTime, methodLabel } from "../format";
  import { changeText, entryKind, filterHistory, type HistoryFilters } from "../history";
  import { labelText } from "../dns";
  import { rescueFocus } from "../focus";

  let query = $state("");
  let adapterFilter = $state("");
  let statusFilter = $state<HistoryFilters["status"]>("");
  let kindFilter = $state<HistoryFilters["kind"]>("");
  let confirmClear = $state(false);

  // Provider names for DNS entries (the catalog is loaded once, by whichever page needs it).
  $effect(() => {
    if (app.history.some((h) => entryKind(h) === "dns")) void app.loadDnsCatalog();
  });

  const adapterNames = $derived(
    [...new Map(app.history.map((h) => [h.adapterId, h.adapterName])).entries()].sort((a, b) =>
      a[1].localeCompare(b[1]),
    ),
  );

  const filtered = $derived(
    filterHistory(
      app.history,
      { query, adapterId: adapterFilter, status: statusFilter, kind: kindFilter },
      app.dnsCatalog,
    ),
  );

  async function exportAs(format: "csv" | "json") {
    try {
      const path = await api.historyExport(format);
      if (path) app.toast("success", t("common.savedTo", { path }));
    } catch (e) {
      app.toastError(e);
    }
  }

  async function clear() {
    confirmClear = false;
    try {
      await api.historyClear();
      await app.refreshHistory();
      // "Clear history" is disabled now that there is nothing left.
      await rescueFocus();
    } catch (e) {
      app.toastError(e);
    }
  }

  function statusTone(s: string) {
    if (s === "applied") return "success";
    if (s === "failed" || s === "rolledBack") return "danger";
    return "warning";
  }
</script>

<PageHeader title={t("history.title")} subtitle={t("history.subtitle")}>
  {#snippet actions()}
    <button class="btn" onclick={() => exportAs("csv")} disabled={app.history.length === 0}>
      <Download size={16} aria-hidden="true" />{t("history.exportCsv")}
    </button>
    <button class="btn" onclick={() => exportAs("json")} disabled={app.history.length === 0}>
      <Download size={16} aria-hidden="true" />{t("history.exportJson")}
    </button>
    <button
      class="btn danger"
      onclick={() => (confirmClear = true)}
      disabled={app.history.length === 0}
    >
      <Trash2 size={16} aria-hidden="true" />{t("history.clear")}
    </button>
  {/snippet}
</PageHeader>

<section class="card">
  <div class="filters">
    <div class="search">
      <Search size={16} aria-hidden="true" />
      <label for="history-search" class="sr-only">{t("history.search")}</label>
      <input
        id="history-search"
        class="input"
        placeholder={t("history.search")}
        bind:value={query}
      />
    </div>
    <label class="sr-only" for="history-adapter">{t("history.adapterFilter")}</label>
    <select id="history-adapter" class="select" bind:value={adapterFilter}>
      <option value="">{t("history.adapterFilter")}: {t("common.all")}</option>
      {#each adapterNames as [id, name] (id)}<option value={id}>{name}</option>{/each}
    </select>
    <label class="sr-only" for="history-kind">{t("history.kindFilter")}</label>
    <select id="history-kind" class="select" bind:value={kindFilter}>
      <option value="">{t("history.kindFilter")}: {t("common.all")}</option>
      <option value="mac">{t("history.kind.mac")}</option>
      <option value="dns">{t("history.kind.dns")}</option>
    </select>
    <label class="sr-only" for="history-status">{t("history.statusFilter")}</label>
    <select id="history-status" class="select" bind:value={statusFilter}>
      <option value="">{t("history.statusFilter")}: {t("common.all")}</option>
      <option value="success">{t("history.success")}</option>
      <option value="failed">{t("history.failed")}</option>
    </select>
  </div>

  {#if app.history.length === 0}
    <p class="empty muted">{t("history.empty")}</p>
  {:else if filtered.length === 0}
    <p class="empty muted">{t("history.noMatch")}</p>
  {:else}
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div class="table-wrap" role="region" aria-label={t("history.tableLabel")} tabindex="0">
      <table class="table">
        <caption class="sr-only">{t("history.tableLabel")}</caption>
        <thead>
          <tr>
            <th scope="col">{t("history.col.time")}</th>
            <th scope="col">{t("history.col.adapter")}</th>
            <th scope="col">{t("history.col.action")}</th>
            <th scope="col">{t("history.col.change")}</th>
            <th scope="col">{t("history.col.status")}</th>
            <th scope="col">{t("history.col.source")}</th>
          </tr>
        </thead>
        <tbody>
          {#each filtered as h (h.id)}
            <tr>
              <td class="nowrap">{formatDateTime(h.timestamp, i18n.locale)}</td>
              <td>{h.adapterName}</td>
              <td>
                {t(`history.action.${h.action}` as MessageKey)}
                {#if entryKind(h) === "dns"}<span class="badge accent">DNS</span>{/if}
              </td>
              <td class="mono" class:nowrap={entryKind(h) === "mac"}>
                {changeText(h, app.format)}
                {#if h.dns?.label && h.action === "apply"}
                  <span class="subtle label">{labelText(h.dns.label, app.dnsCatalog)}</span>
                {/if}
              </td>
              <td>
                <span class="badge {statusTone(h.status)}">
                  {t(`history.status.${h.status}` as MessageKey)}
                </span>
                {#if h.errorCode && h.status !== "applied"}
                  <span class="subtle detail">{i18n.error(h.errorCode)}</span>
                {/if}
              </td>
              <td class="subtle"
                >{t(`history.source.${h.source}` as MessageKey)}{h.method
                  ? ` · ${methodLabel(h.method)}`
                  : ""}</td
              >
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  {/if}
</section>

<ConfirmDialog
  open={confirmClear}
  title={t("history.clearTitle")}
  body={t("history.clearBody", { count: app.history.length })}
  confirmLabel={t("history.clear")}
  danger
  onconfirm={clear}
  oncancel={() => (confirmClear = false)}
/>

<style>
  .filters {
    display: flex;
    gap: 10px;
    margin-bottom: 14px;
    flex-wrap: wrap;
  }
  .filters .select {
    width: auto;
    min-width: 170px;
  }
  .search {
    position: relative;
    flex: 1;
    min-width: 220px;
  }
  .search :global(svg) {
    position: absolute;
    left: 10px;
    top: 50%;
    transform: translateY(-50%);
    color: var(--text-subtle);
  }
  .search .input {
    padding-left: 34px;
  }
  .table-wrap {
    overflow: auto;
    max-height: calc(100vh - 260px);
  }
  .nowrap {
    white-space: nowrap;
  }
  .detail {
    display: block;
    margin-top: 2px;
    max-width: 260px;
  }
  .label {
    display: block;
    font-family: var(--font-ui);
    font-size: 12px;
  }
  .empty {
    padding: 32px;
    text-align: center;
  }
</style>
