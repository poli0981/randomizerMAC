<script lang="ts">
  import { tick } from "svelte";
  import { CircleCheck, CircleAlert, Info, TriangleAlert, X } from "@lucide/svelte";
  import { app } from "../stores/app.svelte";
  import { t } from "../i18n/index.svelte";

  const icons = { success: CircleCheck, error: CircleAlert, info: Info, warning: TriangleAlert };

  /** Closes a notification; when its close button had focus, focus moves to a neighbour. */
  function close(id: number, button: HTMLElement) {
    const hadFocus = document.activeElement === button;
    const ids = app.toasts.map((x) => x.id);
    const at = ids.indexOf(id);
    const neighbour = ids[at + 1] ?? ids[at - 1];
    app.dismiss(id);
    if (!hadFocus) return;
    void tick().then(() => {
      const next =
        neighbour === undefined
          ? null
          : document.querySelector<HTMLElement>(`[data-toast-close="${neighbour}"]`);
      (next ?? document.getElementById("main"))?.focus();
    });
  }
</script>

<!-- A labelled region. What screen readers hear comes from two text-only live regions (errors
     as alerts), so a notification is read once and its close button is not read with it. They
     pause while hovered or focused; errors stay until closed. -->
<section class="toasts" aria-label={t("toasts.label")}>
  <div class="sr-only" aria-live="polite">
    {#each app.toasts.filter((x) => x.kind !== "error") as x (x.id)}<p>{x.message}</p>{/each}
  </div>
  <div class="sr-only" role="alert">
    {#each app.toasts.filter((x) => x.kind === "error") as x (x.id)}<p>{x.message}</p>{/each}
  </div>
  <div class="stack">
    {#each app.toasts as toast (toast.id)}
      {@const Icon = icons[toast.kind]}
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="toast {toast.kind}"
        onmouseenter={() => app.pauseToast(toast.id)}
        onmouseleave={() => app.resumeToast(toast.id)}
        onfocusin={() => app.pauseToast(toast.id)}
        onfocusout={() => app.resumeToast(toast.id)}
      >
        <Icon size={18} aria-hidden="true" />
        <p>{toast.message}</p>
        <button
          class="btn ghost icon small"
          data-toast-close={toast.id}
          aria-label={t("toasts.close", { message: toast.message })}
          onclick={(e) => close(toast.id, e.currentTarget)}
        >
          <X size={14} aria-hidden="true" />
        </button>
      </div>
    {/each}
  </div>
</section>

<style>
  .toasts {
    position: fixed;
    right: 16px;
    bottom: 16px;
    z-index: 50;
    width: min(380px, calc(100vw - 32px));
    pointer-events: none;
  }
  .stack {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .toast {
    pointer-events: auto;
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 12px 10px 12px 14px;
    border-radius: var(--radius);
    background: var(--surface);
    border: 1px solid var(--border);
    border-left: 4px solid var(--info);
    box-shadow: var(--shadow);
    animation: slide 0.18s var(--ease);
  }
  .toast p {
    flex: 1;
    padding-top: 1px;
  }
  .toast.success {
    border-left-color: var(--success);
  }
  .toast.success :global(svg:first-child) {
    color: var(--success);
  }
  .toast.error {
    border-left-color: var(--danger);
  }
  .toast.error :global(svg:first-child) {
    color: var(--danger);
  }
  .toast.warning {
    border-left-color: var(--warning);
  }
  .toast.warning :global(svg:first-child) {
    color: var(--warning);
  }
  .toast.info :global(svg:first-child) {
    color: var(--info);
  }
  @keyframes slide {
    from {
      opacity: 0;
      transform: translateY(8px);
    }
  }
</style>
