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
  const uid = $props.id();
  let dialog = $state<HTMLDialogElement | null>(null);
  let cancelButton = $state<HTMLButtonElement | null>(null);
  /** The control that had focus when the dialog opened (focus goes back to it). */
  let opener: HTMLElement | null = null;

  $effect(() => {
    if (!dialog) return;
    if (open && !dialog.open) {
      opener = document.activeElement instanceof HTMLElement ? document.activeElement : null;
      dialog.showModal();
      // Start on the safe choice.
      cancelButton?.focus();
    }
    if (!open && dialog.open) dialog.close();
  });

  function onclose() {
    if (open) oncancel();
    const target = opener;
    opener = null;
    // The opener may be gone or disabled now (e.g. "Clear history" with nothing left to clear).
    if (target?.isConnected && !target.matches(":disabled")) target.focus();
    else document.querySelector<HTMLElement>("[data-page-heading]")?.focus();
  }
</script>

<dialog
  bind:this={dialog}
  aria-labelledby="{uid}-title"
  aria-describedby={body ? `${uid}-body` : undefined}
  {onclose}
  onclick={(e) => e.target === dialog && oncancel()}
>
  <div class="content">
    <h2 id="{uid}-title">{title}</h2>
    {#if body}<p id="{uid}-body" class="muted">{body}</p>{/if}
    <div class="actions">
      <button class="btn" type="button" bind:this={cancelButton} onclick={oncancel}
        >{t("common.cancel")}</button
      >
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
