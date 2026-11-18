// Keeping keyboard focus somewhere useful when the focused control disappears or is disabled.
import { tick } from "svelte";

/** Focuses the current page's heading (every page has one, see PageHeader). */
export function focusPageHeading(): void {
  document.querySelector<HTMLElement>("[data-page-heading]")?.focus();
}

/**
 * After a change that may remove or disable the focused control (clearing a list, deleting a
 * row): once the page has updated, focus `fallback` — or the page heading — if focus fell back
 * to the page itself.
 */
export async function rescueFocus(fallback?: () => HTMLElement | null | undefined): Promise<void> {
  await tick();
  const active = document.activeElement;
  if (active && active !== document.body) return;
  const target = fallback?.();
  if (target) target.focus();
  else focusPageHeading();
}
