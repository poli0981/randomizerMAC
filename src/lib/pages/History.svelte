<script lang="ts">
  import { Download, Trash2, Search } from "@lucide/svelte";
  import PageHeader from "../components/PageHeader.svelte";
  import ConfirmDialog from "../components/ConfirmDialog.svelte";
  import { app } from "../stores/app.svelte";
  import { api } from "../api";
  import { i18n, t, type MessageKey } from "../i18n/index.svelte";
  import { formatDateTime, methodLabel } from "../format";
  import { formatMac } from "../mac";

  let query = $state("");
  let adapterFilter = $state("");
  let statusFilter = $state<"" | "success" | "failed">("");
  let confirmClear = $state(false);

  const adapterNames = $derived(
    [...new Map(app.history.map((h) => [h.adapterId, h.adapterName])).entries()].sort((a, b) =>
      a[1].localeCompare(b[1]),
    ),
  );

  const filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    const hexQ = /^[0-9a-f:.\- ]+$/i.test(q) ? q.replace(/[^0-9a-f]/gi, "") : null;
    return app.history.filter((h) => {
      if (adapterFilter && h.adapterId !== adapterFilter) return false;
      const ok =
        h.status === "applied" || h.status === "unverified" || h.status === "rebootRequired";
      if (statusFilter === "success" && !ok) return false;
      if (statusFilter === "failed" && ok) return false;
      if (!q) return true;
      if (h.adapterName.toLowerCase().includes(q)) return true;
      if (hexQ) {
        return [h.previousMac, h.newMac, h.requestedMac].some((m) =>
          m
            ?.replace(/[^0-9a-f]/gi, "")
            .toLowerCase()
            .includes(hexQ.toLowerCase()),
        );
      }
      return false;
    });
  });

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
    <div class="table-wrap">
      <table class="table">
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
              <td>{t(`history.action.${h.action}` as MessageKey)}</td>
              <td class="mono nowrap">
                {formatMac(h.previousMac, app.format)} → {formatMac(
                  h.newMac ?? h.requestedMac,
                  app.format,
                )}
              </td>
              <td>
                <span class="badge {statusTone(h.status)}" title={h.error ?? h.errorCode ?? ""}>
                  {t(`history.status.${h.status}` as MessageKey)}
                </span>
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
  .empty {
    padding: 32px;
    text-align: center;
  }
</style>
