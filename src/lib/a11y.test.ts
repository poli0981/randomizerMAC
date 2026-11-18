// Accessibility checks of the whole app in jsdom: axe-core on every page and on an open dialog,
// plus the roles and names that keyboard and screen-reader users rely on. The backend is the dev
// mock (src/lib/devmock.ts). Colour contrast is checked by hand (jsdom has no layout).
import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";
import { cleanup, fireEvent, render, screen, within } from "@testing-library/svelte";
import { clearMocks } from "@tauri-apps/api/mocks";
import { tick } from "svelte";
import App from "../App.svelte";
import { axeViolations, polyfillJsdom } from "../test/axe";
import { installDevMock } from "./devmock";
import { i18n, type MessageKey } from "./i18n/index.svelte";
import { pages } from "./nav";
import { app } from "./stores/app.svelte";
import type { Page } from "./types";

const titles: Record<Page, MessageKey> = {
  dashboard: "dashboard.title",
  dns: "dns.title",
  history: "history.title",
  settings: "settings.title",
  logs: "logs.title",
  update: "update.title",
  about: "about.title",
  social: "social.title",
};

async function open(page: Page) {
  app.navigate(page);
  await tick();
  await vi.waitFor(() =>
    expect(screen.getByRole("heading", { level: 1 }).textContent?.trim()).toBe(
      i18n.t(titles[page]),
    ),
  );
}

beforeAll(async () => {
  polyfillJsdom();
  installDevMock();
  i18n.setLanguage("en");
  render(App);
  await vi.waitFor(() => expect(app.ready).toBe(true), { timeout: 5000 });
});

afterAll(() => {
  cleanup();
  clearMocks();
});

describe("accessibility", () => {
  it("reports what it finds (the scan is not a no-op)", async () => {
    const probe = document.createElement("div");
    probe.innerHTML = '<button></button><img src="x.png">';
    document.body.appendChild(probe);
    const found = await axeViolations(probe);
    probe.remove();
    expect(found.some((v) => v.startsWith("button-name"))).toBe(true);
    expect(found.some((v) => v.startsWith("image-alt"))).toBe(true);
  });

  it("has a skip link, landmarks and named navigation", async () => {
    await open("dashboard");
    const skip = screen.getByRole("link", { name: "Skip to content" });
    const main = screen.getByRole("main");
    expect(skip.getAttribute("href")).toBe("#main");
    await fireEvent.click(skip);
    expect(document.activeElement).toBe(main);
    const nav = screen.getByRole("navigation", { name: i18n.t("nav.label") });
    const buttons = within(nav).getAllByRole("button");
    expect(buttons.filter((b) => b.hasAttribute("aria-keyshortcuts"))).toHaveLength(pages.length);
    const collapse = within(nav).getByRole("button", { name: i18n.t("nav.collapse") });
    expect(collapse.getAttribute("aria-expanded")).toBe("true");
    expect(screen.getByRole("region", { name: i18n.t("toasts.label") })).toBeTruthy();
    expect(document.documentElement.lang).toBe("en");
  });

  it.each(pages)("finds no axe violations on the %s page", async (page) => {
    await open(page);
    if (page === "dns") {
      // Wait for the adapter's DNS settings and the provider catalog.
      await vi.waitFor(() =>
        expect(screen.getByRole("combobox", { name: i18n.t("dns.provider") })).toBeTruthy(),
      );
    }
    expect(await axeViolations()).toEqual([]);
  });

  it("names every control of the DNS page", async () => {
    await open("dns");
    await vi.waitFor(() =>
      expect(screen.getByRole("radio", { name: i18n.t("dns.source.provider") })).toBeTruthy(),
    );
    for (const name of ["dns.source.custom", "dns.source.localResolver", "dns.source.automatic"]) {
      expect(screen.getByRole("radio", { name: i18n.t(name as MessageKey) })).toBeTruthy();
    }
    const dot = screen.getByRole("radio", { name: i18n.t("dns.encryption.dot") });
    // The dev mock is a Windows adapter: no DNS over TLS, and the reason is the description.
    expect((dot as HTMLInputElement).disabled).toBe(true);
    expect(dot.getAttribute("aria-describedby")).toBeTruthy();
    expect(screen.getByRole("button", { name: i18n.t("dns.apply") })).toBeTruthy();

    await fireEvent.click(screen.getByRole("radio", { name: i18n.t("dns.source.custom") }));
    const ipv4 = screen.getByRole("textbox", { name: i18n.t("dns.custom.ipv4") });
    await fireEvent.input(ipv4, { target: { value: "not an address" } });
    await vi.waitFor(() => expect(ipv4.getAttribute("aria-invalid")).toBe("true"));
    expect(ipv4.getAttribute("aria-describedby")).toContain("error");
    expect(await axeViolations()).toEqual([]);
  });

  it("keeps an open dialog accessible and returns focus", async () => {
    await open("history");
    const clear = screen.getByRole("button", { name: i18n.t("history.clear") });
    clear.focus();
    await fireEvent.click(clear);
    const dialog = await screen.findByRole("dialog");
    expect(dialog.getAttribute("aria-labelledby")).toBeTruthy();
    expect(dialog.getAttribute("aria-describedby")).toBeTruthy();
    // Focus starts on the safe choice.
    expect(document.activeElement?.textContent?.trim()).toBe(i18n.t("common.cancel"));
    expect(await axeViolations()).toEqual([]);
    await fireEvent.click(within(dialog).getByRole("button", { name: i18n.t("common.cancel") }));
    await vi.waitFor(() => expect(document.activeElement).toBe(clear));
  });

  it("describes switches and names the history table", async () => {
    await open("settings");
    const switches = screen.getAllByRole("switch");
    expect(switches.length).toBeGreaterThan(3);
    for (const s of switches) expect(s.getAttribute("aria-checked")).toBeNull();
    expect(screen.getByRole("option", { name: "Tiếng Việt" }).getAttribute("lang")).toBe("vi");
    await open("history");
    expect(screen.getByRole("region", { name: i18n.t("history.tableLabel") })).toBeTruthy();
    expect(screen.getByRole("table", { name: i18n.t("history.tableLabel") })).toBeTruthy();
  });

  it("follows the interface language", async () => {
    i18n.setLanguage("vi");
    await tick();
    expect(document.documentElement.lang).toBe("vi");
    await open("dns");
    expect(await axeViolations()).toEqual([]);
    i18n.setLanguage("en");
  });
});
