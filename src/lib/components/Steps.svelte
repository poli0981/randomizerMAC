<script lang="ts">
  import { Check, Loader } from "@lucide/svelte";
  import { t, type MessageKey } from "../i18n/index.svelte";
  import type { Stage } from "../types";

  interface Props {
    /** The stage of the running change; null when none runs. */
    stage: Stage | null;
    /** Whether a change runs here (the steps are shown). */
    active: boolean;
    /** Where the stage names come from: `stage.*` (MAC) or `dns.stage.*`. */
    prefix: "stage" | "dns.stage";
    label: string;
  }
  let { stage, active, prefix, label }: Props = $props();

  const base: Stage[] = ["preparing", "writing", "restarting", "verifying"];
  const list = $derived(stage === "rollingBack" ? [...base, "rollingBack" as Stage] : base);
  const index = $derived(stage ? list.indexOf(stage) : -1);
  const name = (s: Stage) => t(`${prefix}.${s}` as MessageKey);
  // One live region that is always there: "Step 2 of 4: Writing address".
  const announcement = $derived(
    active && stage
      ? t("progress.step", { n: index + 1, total: list.length, step: name(stage) })
      : "",
  );
</script>

<p class="sr-only" aria-live="polite">{announcement}</p>
{#if active}
  <ol class="steps" aria-label={label}>
    {#each list as s, i (s)}
      {@const current = s === stage}
      {@const done = i < index}
      <li class:current class:done aria-current={current ? "step" : undefined}>
        {#if current}<Loader size={14} class="spin" aria-hidden="true" />{:else if done}<Check
            size={14}
            aria-hidden="true"
          />{/if}
        {name(s)}
      </li>
    {/each}
  </ol>
{/if}

<style>
  .steps {
    list-style: none;
    display: flex;
    gap: 6px;
    padding: 0;
    margin: 14px 0 0;
    flex-wrap: wrap;
  }
  .steps li {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 10px;
    border-radius: 999px;
    background: var(--surface-2);
    color: var(--text-subtle);
    font-size: 12.5px;
  }
  .steps li.current {
    background: var(--accent-soft);
    color: var(--accent);
    font-weight: 600;
  }
  .steps li.done {
    color: var(--success);
  }
  @media (forced-colors: active) {
    .steps li {
      border: 1px solid CanvasText;
    }
    .steps li.current {
      outline: 2px solid Highlight;
    }
  }
</style>
