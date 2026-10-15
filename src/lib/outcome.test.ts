import { describe, expect, it } from "vitest";
import { describeOutcome } from "./outcome";
import { i18n } from "./i18n/index.svelte";
import type { OpResult } from "./types";

const fmt = { separator: "colon", uppercase: true } as const;

function result(status: OpResult["entry"]["status"], detailCode: string | null = null): OpResult {
  return {
    adapterId: "a",
    adapterName: "Wi-Fi",
    success: status === "applied",
    error: null,
    entry: {
      id: "1",
      timestamp: new Date().toISOString(),
      adapterId: "a",
      adapterName: "Wi-Fi",
      action: "apply",
      source: "ui",
      status,
    },
    report: {
      method: "windowsRegistry",
      status,
      previousMac: "30:56:0F:09:8F:E4",
      requestedMac: "02:11:22:33:44:55",
      observedMac: "02:11:22:33:44:55",
      detailCode,
      nmConnectionUuid: null,
      nmPreviousValue: null,
      temporary: false,
    },
  };
}

describe("describeOutcome", () => {
  i18n.setLanguage("en");

  it("reports success with the formatted address", () => {
    const out = describeOutcome(result("applied"), false, null, fmt, "windows");
    expect(out.kind).toBe("success");
    expect(out.message).toContain("02:11:22:33:44:55");
  });

  it("explains rollbacks on Windows Wi-Fi", () => {
    const wifi = { kind: "wireless" } as Parameters<typeof describeOutcome>[2];
    const out = describeOutcome(
      result("rolledBack", "DRIVER_REJECTED"),
      false,
      wifi,
      fmt,
      "windows",
    );
    expect(out.kind).toBe("error");
    expect(out.message).toContain("Random hardware addresses");
  });

  it("maps restore details", () => {
    expect(
      describeOutcome(result("applied", "NOTHING_TO_RESTORE"), true, null, fmt, "linux").kind,
    ).toBe("info");
    expect(
      describeOutcome(result("unverified", "CHANGED_BY_OS"), true, null, fmt, "windows").kind,
    ).toBe("info");
  });

  it("falls back to error codes when there is no report", () => {
    const r = result("failed");
    r.report = null;
    r.error = { code: "AUTH_CANCELLED", message: "x" };
    expect(describeOutcome(r, false, null, fmt, "linux").message).toBe(
      i18n.t("error.AUTH_CANCELLED"),
    );
  });
});
