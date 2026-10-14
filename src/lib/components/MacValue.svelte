<script lang="ts">
  import { Copy } from "@lucide/svelte";
  import { app } from "../stores/app.svelte";
  import { t } from "../i18n/index.svelte";
  import { formatMac } from "../mac";

  interface Props {
    label: string;
    mac: string | null;
    vendor?: string | null;
    emphasis?: boolean;
  }
  let { label, mac, vendor = null, emphasis = false }: Props = $props();
  const shown = $derived(formatMac(mac, app.format));

  async function copy() {
    if (!mac) return;
    try {
      await navigator.clipboard.writeText(shown);
      app.toast("success", t("common.copied", { value: shown }));
    } catch (e) {
      app.toastError(e);
    }
  }
</script>

<div class="mac" class:emphasis>
  <div class="head">
    <span class="label">{label}</span>
    {#if mac}
      <button
        class="btn ghost icon small copy"
        onclick={copy}
        aria-label="{t('common.copy')} {label}"
        title={t("common.copy")}
      >
        <Copy size={14} aria-hidden="true" />
      </button>
    {/if}
  </div>
  <span class="value mono" aria-label="{label}: {shown}">{shown}</span>
  <span class="vendor subtle">{vendor ?? " "}</span>
</div>

<style>
  .mac {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 12px 14px;
    border-radius: var(--radius);
    background: var(--surface-2);
    border: 1px solid var(--border);
    min-width: 0;
  }
  .mac.emphasis {
    border-color: color-mix(in srgb, var(--accent) 55%, var(--border));
    background: color-mix(in srgb, var(--accent-soft) 60%, var(--surface-2));
  }
  .label {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 4px;
    min-height: 20px;
  }
  .copy {
    margin: -6px -8px -6px 0;
  }
  /* The whole address must stay readable: wrap rather than truncate in very narrow cards. */
  .value {
    font-size: 15px;
    font-weight: 600;
    overflow-wrap: anywhere;
  }
  .emphasis .value {
    color: var(--accent);
  }
  .vendor {
    min-height: 1.2em;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
</style>
