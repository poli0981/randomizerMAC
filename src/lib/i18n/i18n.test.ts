import { describe, expect, it, vi as mock } from "vitest";
import { en } from "./en";
import { vi } from "./vi";
import { i18n } from "./index.svelte";

const placeholders = (s: string) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort();

describe("translations", () => {
  it("vi has exactly the keys of en", () => {
    expect(Object.keys(vi).sort()).toEqual(Object.keys(en).sort());
  });

  it("placeholders match in every language", () => {
    for (const key of Object.keys(en) as (keyof typeof en)[]) {
      expect(placeholders(vi[key]), key).toEqual(placeholders(en[key]));
    }
  });

  it("no empty strings", () => {
    for (const [key, value] of Object.entries(vi)) {
      expect(value.trim(), key).not.toBe("");
    }
  });

  it("covers every backend error code", () => {
    const codes = [
      "ADAPTER_NOT_FOUND",
      "NOT_ELEVATED",
      "PERMISSION_DENIED",
      "AUTH_CANCELLED",
      "AUTH_UNAVAILABLE",
      "UNSUPPORTED",
      "INVALID_INPUT",
      "INVALID_MAC",
      "INVALID_PATTERN",
      "BLACKLISTED",
      "BLACKLIST_REJECTED",
      "NM_UNAVAILABLE",
      "NM_NO_ACTIVE_CONNECTION",
      "VERIFY_TIMEOUT",
      "ROLLBACK_FAILED",
      "NO_ORIGINAL",
      "COMMAND_FAILED",
      "TIMEOUT",
      "OS_ERROR",
      "IO_ERROR",
      "STORAGE_ERROR",
      "DRIVER_REJECTED",
      "BUSY",
      "GENERATE_FAILED",
      "AUTOSTART_FAILED",
      "UPDATE_FAILED",
      "NO_UPDATE",
      "PARSE_ERROR",
      "INTERNAL",
    ];
    for (const code of codes) {
      expect(`error.${code}` in en, code).toBe(true);
    }
  });
});

describe("t()", () => {
  it("interpolates and switches language", () => {
    i18n.setLanguage("en");
    expect(i18n.t("common.copied", { value: "X" })).toBe("Copied X");
    i18n.setLanguage("vi");
    expect(i18n.t("common.copied", { value: "X" })).toBe("Đã sao chép X");
    expect(i18n.error("BUSY")).toBe(vi["error.BUSY"]);
    expect(i18n.error("SOMETHING_NEW")).toContain("SOMETHING_NEW");
    i18n.setLanguage("en");
  });

  it("keeps placeholders without a value and reports missing codes", () => {
    i18n.setLanguage("en");
    expect(i18n.t("common.copied", {})).toBe("Copied {value}");
    expect(i18n.t("common.copied")).toBe("Copied {value}");
    expect(i18n.error(null)).toBe(i18n.t("error.unknown", { code: "?" }));
    expect(i18n.error(undefined)).toContain("?");
  });

  it("follows the system language and marks the document", () => {
    const languages = mock.spyOn(navigator, "languages", "get");
    languages.mockReturnValue(["vi-VN", "en-US"]);
    i18n.setLanguage("system");
    expect(i18n.locale).toBe("vi");
    expect(document.documentElement.lang).toBe("vi");
    languages.mockReturnValue(["de-DE", "en-GB"]);
    i18n.setLanguage("system");
    expect(i18n.locale).toBe("en");
    expect(document.documentElement.lang).toBe("en");
    languages.mockRestore();
  });
});
