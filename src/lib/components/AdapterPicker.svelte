<script lang="ts">
  import { app } from "../stores/app.svelte";
  import { t } from "../i18n/index.svelte";

  interface Props {
    selected: string[];
    disabled?: boolean;
    label: string;
    onchange: (ids: string[]) => void;
  }
  let { selected, disabled = false, label, onchange }: Props = $props();

  // Keep ids of adapters that are currently unplugged (v1 silently dropped them).
  const missing = $derived(selected.filter((id) => !app.adapters.some((a) => a.id === id)));
  const candidates = $derived(app.adapters.filter((a) => !a.isVirtual || selected.includes(a.id)));

  function toggle(id: string, on: boolean) {
    const next = on ? [...new Set([...selected, id])] : selected.filter((x) => x !== id);
    onchange(next);
  }
</script>

<fieldset class="picker" {disabled}>
  <legend class="sr-only">{label}</legend>
  {#if candidates.length === 0 && missing.length === 0}
    <p class="subtle">{t("settings.noAdaptersToPick")}</p>
  {/if}
  {#each candidates as a (a.id)}
    <label class="item">
      <input
        type="checkbox"
        checked={selected.includes(a.id)}
        onchange={(e) => toggle(a.id, (e.currentTarget as HTMLInputElement).checked)}
      />
      <span>{a.name}</span>
      <span class="subtle">{a.description}</span>
      {#if app.info?.os === "linux" && !a.methods.includes("networkManager")}
        <span class="badge warning">iproute2</span>
      {/if}
    </label>
  {/each}
  {#each missing as id (id)}
    <label class="item missing">
      <input type="checkbox" checked onchange={() => toggle(id, false)} />
      <span class="mono subtle">{id}</span>
    </label>
  {/each}
</fieldset>

<style>
  .picker {
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 6px 10px;
    margin: 6px 0 4px;
    display: flex;
    flex-direction: column;
  }
  .picker:disabled {
    opacity: 0.55;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 0;
    cursor: pointer;
    min-width: 0;
  }
  .item + .item {
    border-top: 1px solid var(--border);
  }
  .item .subtle {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
