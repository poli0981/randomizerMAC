<script lang="ts">
  import { CircleCheck, CircleAlert, Info, TriangleAlert, X } from "@lucide/svelte";
  import { app } from "../stores/app.svelte";
  import { t } from "../i18n/index.svelte";

  const icons = { success: CircleCheck, error: CircleAlert, info: Info, warning: TriangleAlert };
</script>

<div class="toasts" aria-live="polite" aria-atomic="false">
  {#each app.toasts as toast (toast.id)}
    {@const Icon = icons[toast.kind]}
    <div class="toast {toast.kind}" role={toast.kind === "error" ? "alert" : "status"}>
      <Icon size={18} aria-hidden="true" />
      <p>{toast.message}</p>
      <button
        class="btn ghost icon small"
        aria-label={t("common.close")}
        onclick={() => app.dismiss(toast.id)}
      >
        <X size={14} aria-hidden="true" />
      </button>
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: 16px;
    bottom: 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    z-index: 50;
    width: min(380px, calc(100vw - 32px));
    pointer-events: none;
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
