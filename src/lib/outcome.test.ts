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

  it("confirms restores", () => {
    const out = describeOutcome(result("applied"), true, null, fmt, "windows");
    expect(out).toEqual({
      kind: "success",
      message: i18n.t("result.restored", { adapter: "Wi-Fi" }),
    });
  });

  it("warns when the change could not be verified or needs a reboot", () => {
    expect(describeOutcome(result("unverified"), false, null, fmt, "linux")).toEqual({
      kind: "warning",
      message: i18n.t("result.unverified"),
    });
    expect(describeOutcome(result("rebootRequired"), false, null, fmt, "windows")).toEqual({
      kind: "warning",
      message: i18n.t("result.rebootRequired"),
    });
  });

  it("explains rollbacks elsewhere without the Wi-Fi hint", () => {
    const ethernet = { kind: "ethernet" } as Parameters<typeof describeOutcome>[2];
    for (const [adapter, os] of [
      [ethernet, "windows"],
      [null, "windows"],
      [{ kind: "wireless" } as Parameters<typeof describeOutcome>[2], "linux"],
    ] as const) {
      expect(describeOutcome(result("rolledBack"), false, adapter, fmt, os).message).toBe(
        i18n.t("result.rolledBack"),
      );
    }
  });

  it("reports failures with the most specific code", () => {
    const r = result("failed", "VERIFY_TIMEOUT");
    expect(describeOutcome(r, false, null, fmt, "linux")).toEqual({
      kind: "error",
      message: i18n.error("VERIFY_TIMEOUT"),
    });
    r.entry.errorCode = "DRIVER_REJECTED";
    expect(describeOutcome(r, false, null, fmt, "linux").message).toBe(
      i18n.error("DRIVER_REJECTED"),
    );
    r.report = null;
    expect(describeOutcome(r, false, null, fmt, "linux").message).toBe(
      i18n.error("DRIVER_REJECTED"),
    );
  });

  it("formats the requested address when none was observed", () => {
    const r = result("applied");
    r.report!.observedMac = null;
    const lower = { separator: "hyphen", uppercase: false } as const;
    expect(describeOutcome(r, false, null, lower, "linux").message).toContain("02-11-22-33-44-55");
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
