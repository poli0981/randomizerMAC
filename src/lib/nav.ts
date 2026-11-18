// The pages in sidebar order; Ctrl+1 … Ctrl+8 open them in this order.
import type { Page } from "./types";

export const pages: readonly Page[] = [
  "dashboard",
  "dns",
  "history",
  "settings",
  "logs",
  "update",
  "about",
  "social",
];

interface ShortcutKey {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
}

/** The page a Ctrl+<digit> (⌘ on macOS) key press opens, or null for any other key. */
export function pageForShortcut(e: ShortcutKey): Page | null {
  if (!(e.ctrlKey || e.metaKey) || e.shiftKey || e.altKey || !/^[1-9]$/.test(e.key)) {
    return null;
  }
  return pages[Number(e.key) - 1] ?? null;
}

/** The shortcut of a page, e.g. "Ctrl+2" for the DNS page. */
export function shortcutOf(page: Page): string {
  return `Ctrl+${pages.indexOf(page) + 1}`;
}
