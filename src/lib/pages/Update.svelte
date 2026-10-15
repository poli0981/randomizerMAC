<script lang="ts">
  import { CloudDownload, RefreshCw, CircleCheck, ExternalLink, Loader } from "@lucide/svelte";
  import DOMPurify from "dompurify";
  import { marked } from "marked";
  import PageHeader from "../components/PageHeader.svelte";
  import { app } from "../stores/app.svelte";
  import { api } from "../api";
  import { i18n, t } from "../i18n/index.svelte";
  import { formatDate, formatRelative } from "../format";

  const RELEASES = "https://github.com/poli0981/randomizerMAC/releases";

  let checking = $state(false);
  let installing = $state(false);
  let error = $state<string | null>(null);

  const update = $derived(app.update);
  const percent = $derived(
    app.updateProgress?.total
      ? Math.round((app.updateProgress.downloaded / app.updateProgress.total) * 100)
      : null,
  );
  const notesHtml = $derived.by(() => {
    if (!update?.notes) return "";
    const html = marked.parse(update.notes, { async: false, gfm: true }) as string;
    // Release notes come from our own signed manifest, but are still sanitized: no images,
    // scripts, styles or inline handlers.
    return DOMPurify.sanitize(html, {
      FORBID_TAGS: ["img", "style", "iframe", "form", "input", "script"],
      FORBID_ATTR: ["style", "onerror", "onclick"],
    });
  });
  const lastChecked = $derived(update?.checkedAt ?? app.settings?.lastUpdateCheck ?? null);

  async function check() {
    checking = true;
    error = null;
    try {
      app.update = await api.updateCheck();
    } catch (e) {
      const code = (e as { code?: string }).code;
      error = i18n.error(code);
    } finally {
      checking = false;
    }
  }

  async function install() {
    installing = true;
    error = null;
    try {
      await api.updateInstall();
    } catch (e) {
      installing = false;
      app.updateInstalling = false;
      const code = (e as { code?: string }).code;
      error = i18n.error(code);
    }
  }

  function onNotesClick(e: MouseEvent) {
    const a = (e.target as HTMLElement).closest("a");
    if (!a) return;
    e.preventDefault();
    const href = a.getAttribute("href");
    if (href) api.openUrl(href).catch(() => {});
  }
</script>

<PageHeader title={t("update.title")} subtitle={t("update.subtitle")}>
  {#snippet actions()}
    <button class="btn" onclick={() => api.openUrl(RELEASES)}
      ><ExternalLink size={16} aria-hidden="true" />{t("update.releases")}</button
    >
  {/snippet}
</PageHeader>

<div class="stack">
  <section class="card hero">
    <div>
      <p class="label muted">{t("update.current")}</p>
      <p class="version">v{app.info?.version}</p>
      <p class="subtle">
        {lastChecked
          ? t("update.lastChecked", { time: formatRelative(lastChecked, i18n.locale) })
          : t("update.never")}
      </p>
    </div>
    <button class="btn primary" onclick={check} disabled={checking || installing}>
      <RefreshCw size={16} aria-hidden="true" class={checking ? "spin" : ""} />
      {checking ? t("update.checking") : t("update.check")}
    </button>
  </section>

  {#if error}
    <div class="callout danger" role="alert">{error}</div>
  {/if}

  {#if update && !update.available && !checking}
    <div class="callout success" role="status">
      <CircleCheck size={18} aria-hidden="true" />
      <p>{t("update.upToDate")}</p>
    </div>
  {/if}

  {#if update?.available}
    <section class="card" aria-labelledby="new-version">
      <div class="card-head">
        <div>
          <h2 id="new-version" class="card-title">
            {t("update.available", { version: update.version ?? "" })}
          </h2>
          {#if update.date}<p class="subtle">
              {t("update.published", { date: formatDate(update.date, i18n.locale) })}
            </p>{/if}
        </div>
        <button
          class="btn primary"
          onclick={install}
          disabled={installing || !app.info?.packageKind}
        >
          {#if installing}<Loader size={16} class="spin" aria-hidden="true" />{:else}<CloudDownload
              size={16}
              aria-hidden="true"
            />{/if}
          {t("update.install")}
        </button>
      </div>
      {#if !app.info?.packageKind}
        <p class="subtle">{t("update.devBuild")}</p>
      {/if}
      {#if installing}
        <div
          class="progress"
          role="progressbar"
          aria-valuemin="0"
          aria-valuemax="100"
          aria-valuenow={percent ?? undefined}
        >
          <div
            class="bar"
            style:width={percent != null ? `${percent}%` : "35%"}
            class:indeterminate={percent == null}
          ></div>
        </div>
        <p class="subtle" aria-live="polite">
          {app.updateInstalling
            ? t("update.installing")
            : t("update.downloading", { percent: percent != null ? `${percent}%` : "" })}
        </p>
      {/if}
      {#if notesHtml}
        <h3 class="notes-title">{t("update.notes")}</h3>
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="notes" onclick={onNotesClick}>{@html notesHtml}</div>
      {/if}
    </section>
  {/if}
</div>

<style>
  .hero {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .version {
    font-size: 30px;
    font-weight: 700;
    letter-spacing: -0.02em;
  }
  .progress {
    height: 8px;
    border-radius: 999px;
    background: var(--surface-3);
    overflow: hidden;
    margin: 12px 0 6px;
  }
  .bar {
    height: 100%;
    background: var(--accent);
    transition: width 0.2s var(--ease);
  }
  .bar.indeterminate {
    animation: slide 1.2s ease-in-out infinite;
  }
  @keyframes slide {
    from {
      transform: translateX(-100%);
    }
    to {
      transform: translateX(300%);
    }
  }
  .notes-title {
    font-size: 14px;
    margin: 16px 0 8px;
  }
  .notes {
    max-height: 420px;
    overflow: auto;
    padding: 12px 16px;
    border-radius: var(--radius);
    background: var(--surface-2);
    border: 1px solid var(--border);
    line-height: 1.55;
  }
  .notes :global(h1),
  .notes :global(h2),
  .notes :global(h3) {
    font-size: 15px;
    margin: 12px 0 6px;
  }
  .notes :global(ul) {
    padding-left: 20px;
  }
  .notes :global(code) {
    font-family: var(--font-mono);
    font-size: 12.5px;
  }
  .notes :global(a) {
    color: var(--accent);
  }
</style>
