import { describe, expect, it } from "vitest";
import { pageForShortcut, pages, shortcutOf } from "./nav";

const key = (
  k: string,
  mods: Partial<Record<"ctrl" | "meta" | "shift" | "alt", boolean>> = {},
) => ({
  key: k,
  ctrlKey: mods.ctrl ?? false,
  metaKey: mods.meta ?? false,
  shiftKey: mods.shift ?? false,
  altKey: mods.alt ?? false,
});

describe("navigation", () => {
  it("puts DNS second and keeps every page once", () => {
    expect(pages[0]).toBe("dashboard");
    expect(pages[1]).toBe("dns");
    expect(new Set(pages).size).toBe(pages.length);
    expect(pages).toHaveLength(8);
  });

  it("maps Ctrl+1 … Ctrl+8 to the pages in order", () => {
    pages.forEach((page, i) => {
      expect(pageForShortcut(key(String(i + 1), { ctrl: true }))).toBe(page);
      expect(shortcutOf(page)).toBe(`Ctrl+${i + 1}`);
    });
    expect(pageForShortcut(key("2", { meta: true }))).toBe("dns");
  });

  it("ignores other keys and modifier combinations", () => {
    expect(pageForShortcut(key("9", { ctrl: true }))).toBeNull();
    expect(pageForShortcut(key("0", { ctrl: true }))).toBeNull();
    expect(pageForShortcut(key("2"))).toBeNull();
    expect(pageForShortcut(key("2", { ctrl: true, shift: true }))).toBeNull();
    expect(pageForShortcut(key("2", { ctrl: true, alt: true }))).toBeNull();
    expect(pageForShortcut(key("a", { ctrl: true }))).toBeNull();
  });
});
