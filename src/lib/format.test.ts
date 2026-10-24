import { describe, expect, it } from "vitest";
import {
  formatBytes,
  formatDate,
  formatDateTime,
  formatRelative,
  formatTime,
  methodLabel,
} from "./format";

const iso = "2026-03-04T05:06:07.008Z";

describe("dates", () => {
  it("formats with the locale's conventions", () => {
    const d = new Date(iso);
    expect(formatDateTime(iso, "en")).toBe(
      new Intl.DateTimeFormat("en-US", { dateStyle: "medium", timeStyle: "short" }).format(d),
    );
    expect(formatDateTime(iso, "vi")).toBe(
      new Intl.DateTimeFormat("vi-VN", { dateStyle: "medium", timeStyle: "short" }).format(d),
    );
    expect(formatDate(iso, "en")).toBe(
      new Intl.DateTimeFormat("en-US", { dateStyle: "long" }).format(d),
    );
    expect(formatDate(iso, "vi")).toContain("2026");
  });

  it("shows a dash for missing values and keeps unparsable ones", () => {
    for (const f of [formatDateTime, formatDate, formatRelative]) {
      expect(f(null, "en")).toBe("—");
      expect(f(undefined, "vi")).toBe("—");
      expect(f("", "en")).toBe("—");
      expect(f("not a date", "en")).toBe("not a date");
    }
    expect(formatTime("not a date")).toBe("not a date");
  });

  it("formats relative times in every unit", () => {
    const now = Date.parse(iso);
    const at = (seconds: number) => new Date(now + seconds * 1000).toISOString();
    expect(formatRelative(at(-10), "en", now)).toBe("now");
    expect(formatRelative(at(-5 * 60), "en", now)).toBe("5 minutes ago");
    expect(formatRelative(at(3 * 3600), "en", now)).toBe("in 3 hours");
    expect(formatRelative(at(-86400), "en", now)).toBe("yesterday");
    expect(formatRelative(at(-40 * 86400), "en", now)).toBe(formatDate(at(-40 * 86400), "en"));
    expect(formatRelative(at(-2 * 86400), "vi", now)).toBe(
      new Intl.RelativeTimeFormat("vi-VN", { numeric: "auto" }).format(-2, "day"),
    );
  });

  it("formats local clock times with milliseconds", () => {
    const local = new Date(2026, 0, 2, 3, 4, 5, 6).toISOString();
    expect(formatTime(local)).toBe("03:04:05.006");
  });
});

describe("formatBytes", () => {
  it("picks the unit", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(1023)).toBe("1023 B");
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(5 * 1024 * 1024)).toBe("5.0 MB");
  });
});

describe("methodLabel", () => {
  it("names the known methods and passes others through", () => {
    expect(methodLabel("windowsRegistry")).toBe("Windows (NetworkAddress)");
    expect(methodLabel("networkManager")).toBe("NetworkManager");
    expect(methodLabel("iproute2")).toBe("iproute2");
    expect(methodLabel("mock")).toBe("Demo");
    expect(methodLabel("future")).toBe("future");
  });
});
