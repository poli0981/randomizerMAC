import { i18n } from "./i18n/index.svelte";
import { formatMac } from "./mac";
import type { Adapter, MacFormat, OpResult } from "./types";
import type { ToastKind } from "./stores/app.svelte";

/** Turns a backend operation result into a user-facing message. */
export function describeOutcome(
  result: OpResult,
  restore: boolean,
  adapter: Adapter | null,
  format: MacFormat,
  os: string | undefined,
): { kind: ToastKind; message: string } {
  const t = i18n.t;
  const report = result.report;
  if (!report) {
    return { kind: "error", message: i18n.error(result.error?.code ?? result.entry.errorCode) };
  }
  const mac = formatMac(report.observedMac ?? report.requestedMac, format);
  switch (report.status) {
    case "applied":
      if (report.detailCode === "NOTHING_TO_RESTORE") {
        return { kind: "info", message: t("result.nothingToRestore") };
      }
      return {
        kind: "success",
        message: restore
          ? t("result.restored", { adapter: result.adapterName })
          : t("result.applied", { adapter: result.adapterName, mac }),
      };
    case "unverified":
      if (report.detailCode === "CHANGED_BY_OS") {
        return { kind: "info", message: t("result.changedByOs") };
      }
      return { kind: "warning", message: t("result.unverified") };
    case "rebootRequired":
      return { kind: "warning", message: t("result.rebootRequired") };
    case "rolledBack":
      return {
        kind: "error",
        message:
          os === "windows" && adapter?.kind === "wireless"
            ? t("result.rolledBackWifi")
            : t("result.rolledBack"),
      };
    case "failed":
    default:
      return { kind: "error", message: i18n.error(result.entry.errorCode ?? report.detailCode) };
  }
}
