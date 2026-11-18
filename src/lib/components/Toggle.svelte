<script lang="ts">
  interface Props {
    checked: boolean;
    label: string;
    description?: string;
    disabled?: boolean;
    hint?: string;
    onchange: (value: boolean) => void;
  }
  let { checked, label, description, disabled = false, hint, onchange }: Props = $props();
  const id = $props.id();
  const describedBy = $derived(
    [description ? `${id}-desc` : "", hint ? `${id}-hint` : ""].filter(Boolean).join(" ") ||
      undefined,
  );
</script>

<div class="toggle" class:disabled>
  <div class="text">
    <label for={id}>{label}</label>
    {#if description}<p id="{id}-desc" class="subtle">{description}</p>{/if}
    {#if hint}<p id="{id}-hint" class="hint">{hint}</p>{/if}
  </div>
  <input
    {id}
    type="checkbox"
    role="switch"
    {checked}
    {disabled}
    aria-describedby={describedBy}
    onchange={(e) => onchange((e.currentTarget as HTMLInputElement).checked)}
  />
</div>

<style>
  .toggle {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    padding: 10px 0;
  }
  .toggle + :global(.toggle) {
    border-top: 1px solid var(--border);
  }
  .text {
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }
  label {
    font-weight: 500;
    cursor: pointer;
  }
  .disabled label {
    cursor: default;
    opacity: 0.6;
  }
  .hint {
    font-size: 12px;
    color: var(--warning);
  }
  input {
    appearance: none;
    flex: none;
    width: 40px;
    height: 22px;
    border-radius: 999px;
    background: var(--surface-3);
    border: 1px solid var(--input-border);
    position: relative;
    cursor: pointer;
    transition: background 0.15s var(--ease);
    margin: 0;
  }
  input::after {
    content: "";
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--text-muted);
    transition:
      transform 0.15s var(--ease),
      background 0.15s;
  }
  input:checked {
    background: var(--accent);
    border-color: var(--accent);
  }
  input:checked::after {
    transform: translateX(18px);
    background: var(--accent-contrast);
  }
  input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  /* Windows contrast themes would paint the track and the knob alike: use system colours. */
  @media (forced-colors: active) {
    input {
      forced-color-adjust: none;
      background: Canvas;
      border-color: CanvasText;
    }
    input::after {
      background: CanvasText;
    }
    input:checked {
      background: Highlight;
      border-color: Highlight;
    }
    input:checked::after {
      background: HighlightText;
    }
    input:disabled {
      border-color: GrayText;
    }
    input:disabled::after {
      background: GrayText;
    }
  }
</style>
