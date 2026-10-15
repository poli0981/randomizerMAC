<script lang="ts">
  import { onMount, tick } from "svelte";
  import { ShieldAlert, FlaskConical, Info, TriangleAlert, CloudDownload, X } from "@lucide/svelte";
  import Sidebar from "./lib/components/Sidebar.svelte";
  import Toasts from "./lib/components/Toasts.svelte";
  import Dashboard from "./lib/pages/Dashboard.svelte";
  import History from "./lib/pages/History.svelte";
  import Settings from "./lib/pages/Settings.svelte";
  import Logs from "./lib/pages/Logs.svelte";
  import Update from "./lib/pages/Update.svelte";
  import About from "./lib/pages/About.svelte";
  import Social from "./lib/pages/Social.svelte";
  import { app } from "./lib/stores/app.svelte";
  import { api } from "./lib/api";
  import { t } from "./lib/i18n/index.svelte";
  import type { Page } from "./lib/types";

  const pages: Page[] = ["dashboard", "history", "settings", "logs", "update", "about", "social"];
  const isDev = import.meta.env.DEV;

  let collapsed = $state(readCollapsed());
  let dismissed = $state<Record<string, boolean>>({});

  function readCollapsed(): boolean {
    try {
      return localStorage.getItem("sidebarCollapsed") === "1";
    } catch {
      return false;
    }
  }

  $effect(() => {
    try {
      localStorage.setItem("sidebarCollapsed", collapsed ? "1" : "0");
    } catch {
      /* storage unavailable */
    }
  });

  // Theme: "system" follows prefers-color-scheme via CSS; light/dark force a palette.
  $effect(() => {
    const theme = app.settings?.theme ?? "system";
    if (theme === "system") delete document.documentElement.dataset.theme;
    else document.documentElement.dataset.theme = theme;
  });

  // Move focus to the page heading after navigation (screen readers announce the new page).
  let previousPage: Page | null = null;
  $effect(() => {
    const page = app.page;
    if (previousPage !== null && previousPage !== page) {
      void tick().then(() => {
        document.querySelector<HTMLElement>("[data-page-heading]")?.focus();
        document.querySelector("main")?.scrollTo({ top: 0 });
      });
    }
    previousPage = page;
  });

  onMount(async () => {
    await app.init();
    await tick();
    api.uiReady().catch(() => {});
  });

  function onKeydown(e: KeyboardEvent) {
    const ctrl = e.ctrlKey || e.metaKey;
    const key = e.key.toLowerCase();
    if (ctrl && !e.shiftKey && /^[1-7]$/.test(e.key)) {
      e.preventDefault();
      app.navigate(pages[Number(e.key) - 1]!);
      return;
    }
    if (e.key === "F5") {
      e.preventDefault();
      void app.refreshAdapters();
      return;
    }
    // Suppress browser shortcuts that make no sense in a desktop app.
    const blocked =
      (ctrl && ["p", "f", "g", "u", "j", "s", "o"].includes(key)) ||
      (ctrl && key === "r") ||
      (ctrl && e.shiftKey && ["r", "i", "c"].includes(key) && !isDev) ||
      ["F3", "F7"].includes(e.key) ||
      (e.key === "F12" && !isDev);
    if (blocked) e.preventDefault();
  }

  function onContextMenu(e: MouseEvent) {
    const target = e.target as HTMLElement;
    const editable = target.closest("input, textarea, [contenteditable='true']");
    const hasSelection = !!window.getSelection()?.toString();
    if (!editable && !hasSelection && !isDev) e.preventDefault();
  }

  const startup = $derived(app.info?.startup);
</script>

<svelte:window onkeydown={onKeydown} oncontextmenu={onContextMenu} />

{#if app.fatal}
  <div class="fatal" role="alert">
    <TriangleAlert size={28} aria-hidden="true" />
    <p>{t("error.INTERNAL")}</p>
    <p class="mono subtle">{app.fatal}</p>
  </div>
{:else}
  <div class="shell">
    <Sidebar bind:collapsed />
    <main>
      {#if app.ready}
        <div class="banners">
          {#if app.info?.readOnly}
            <div class="callout warning">
              <ShieldAlert size={18} aria-hidden="true" />
              <div class="grow">
                <strong>{t("banner.readOnly.title")}</strong>
                <p>{t("banner.readOnly.body")}</p>
              </div>
              <button
                class="btn small"
                onclick={() => api.relaunchElevated().catch((e) => app.toastError(e))}
              >
                {t("banner.readOnly.action")}
              </button>
            </div>
          {/if}
          {#if app.info?.mock}
            <div class="callout info">
              <FlaskConical size={18} aria-hidden="true" />
              <p>{t("banner.mock")}</p>
            </div>
          {/if}
          {#if startup?.v1Import && !dismissed.v1}
            <div class="callout success">
              <Info size={18} aria-hidden="true" />
              <p class="grow">
                {t("banner.v1Import", {
                  history: startup.v1Import.history,
                  blacklist: startup.v1Import.blacklist,
                })}
              </p>
              <button
                class="btn ghost icon small"
                aria-label={t("common.close")}
                onclick={() => (dismissed.v1 = true)}><X size={14} aria-hidden="true" /></button
              >
            </div>
          {/if}
          {#if startup?.legacyTaskRemoved && !dismissed.legacy}
            <div class="callout info">
              <Info size={18} aria-hidden="true" />
              <p class="grow">{t("banner.legacyTask")}</p>
              <button
                class="btn ghost icon small"
                aria-label={t("common.close")}
                onclick={() => (dismissed.legacy = true)}><X size={14} aria-hidden="true" /></button
              >
            </div>
          {/if}
          {#if startup?.recoveredFiles.length && !dismissed.recovered}
            <div class="callout warning">
              <TriangleAlert size={18} aria-hidden="true" />
              <p class="grow">{t("banner.recovered")}</p>
              <button
                class="btn ghost icon small"
                aria-label={t("common.close")}
                onclick={() => (dismissed.recovered = true)}
                ><X size={14} aria-hidden="true" /></button
              >
            </div>
          {/if}
          {#if app.update?.available && app.page !== "update" && !dismissed.update}
            <div class="callout info">
              <CloudDownload size={18} aria-hidden="true" />
              <p class="grow">{t("banner.update", { version: app.update.version ?? "" })}</p>
              <button class="btn small" onclick={() => app.navigate("update")}
                >{t("banner.update.action")}</button
              >
              <button
                class="btn ghost icon small"
                aria-label={t("common.close")}
                onclick={() => (dismissed.update = true)}><X size={14} aria-hidden="true" /></button
              >
            </div>
          {/if}
        </div>

        {#if app.page === "dashboard"}<Dashboard />
        {:else if app.page === "history"}<History />
        {:else if app.page === "settings"}<Settings />
        {:else if app.page === "logs"}<Logs />
        {:else if app.page === "update"}<Update />
        {:else if app.page === "about"}<About />
        {:else}<Social />{/if}
      {:else}
        <p class="loading muted" aria-live="polite">{t("common.loading")}</p>
      {/if}
    </main>
  </div>
  <Toasts />
{/if}

<style>
  .shell {
    display: flex;
    height: 100vh;
  }
  main {
    flex: 1;
    min-width: 0;
    overflow-y: auto;
    padding: 22px 26px 40px;
  }
  .banners {
    display: flex;
    flex-direction: column;
    gap: 8px;
    margin-bottom: 14px;
  }
  .banners:empty {
    display: none;
  }
  .banners .callout {
    align-items: center;
  }
  .grow {
    flex: 1;
  }
  .loading {
    padding: 40px;
    text-align: center;
  }
  .fatal {
    height: 100vh;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 10px;
    color: var(--danger);
    padding: 24px;
    text-align: center;
  }
</style>
