<script lang="ts">
  import { t } from "../i18n/index.svelte";

  interface Props {
    open: boolean;
    title: string;
    body?: string;
    confirmLabel?: string;
    danger?: boolean;
    onconfirm: () => void;
    oncancel: () => void;
  }
  let { open, title, body, confirmLabel, danger = false, onconfirm, oncancel }: Props = $props();
  let dialog = $state<HTMLDialogElement | null>(null);

  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    if (!open && dialog.open) dialog.close();
  });
</script>

<dialog
  bind:this={dialog}
  aria-labelledby="confirm-title"
  onclose={() => open && oncancel()}
  onclick={(e) => e.target === dialog && oncancel()}
>
  <div class="content">
    <h2 id="confirm-title">{title}</h2>
    {#if body}<p class="muted">{body}</p>{/if}
    <div class="actions">
      <button class="btn" type="button" onclick={oncancel}>{t("common.cancel")}</button>
      <button class="btn" class:primary={!danger} class:danger type="button" onclick={onconfirm}>
        {confirmLabel ?? t("common.confirm")}
      </button>
    </div>
  </div>
</dialog>

<style>
  dialog {
    border: 1px solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
    color: var(--text);
    padding: 0;
    width: min(440px, calc(100vw - 32px));
    box-shadow: var(--shadow);
  }
  dialog::backdrop {
    background: rgb(0 0 0 / 0.45);
  }
  .content {
    padding: 22px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  h2 {
    font-size: 17px;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 8px;
  }
</style>
