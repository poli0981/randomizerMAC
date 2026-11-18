// History page logic: filtering MAC and DNS entries, and the "change" column.
import { labelText, summaryText } from "./dns";
import { formatMac } from "./mac";
import type { ChangeStatus, DnsCatalog, HistoryEntry, HistoryKind, MacFormat } from "./types";

export interface HistoryFilters {
  query: string;
  adapterId: string;
  status: "" | "success" | "failed";
  kind: "" | HistoryKind;
}

/** MAC entries written before 2.1 have no kind. */
export function entryKind(h: HistoryEntry): HistoryKind {
  return h.kind ?? "mac";
}

export function isSuccess(status: ChangeStatus): boolean {
  return status === "applied" || status === "unverified" || status === "rebootRequired";
}

function macMatches(h: HistoryEntry, hex: string): boolean {
  return [h.previousMac, h.newMac, h.requestedMac].some((m) =>
    m
      ?.replace(/[^0-9a-f]/gi, "")
      .toLowerCase()
      .includes(hex),
  );
}

/** The texts a DNS entry is found by: its servers and the provider or preset name. */
function dnsTexts(h: HistoryEntry, catalog: DnsCatalog | null): string[] {
  const d = h.dns;
  if (!d) return [];
  return [
    summaryText(d.previous),
    summaryText(d.requested),
    summaryText(d.observed),
    labelText(d.label, catalog),
  ].map((s) => s.toLowerCase());
}

export function filterHistory(
  entries: HistoryEntry[],
  f: HistoryFilters,
  catalog: DnsCatalog | null = null,
): HistoryEntry[] {
  const q = f.query.trim().toLowerCase();
  const hex = /^[0-9a-f:.\- ]+$/i.test(q) ? q.replace(/[^0-9a-f]/gi, "") : null;
  return entries.filter((h) => {
    if (f.adapterId && h.adapterId !== f.adapterId) return false;
    if (f.kind && entryKind(h) !== f.kind) return false;
    if (f.status === "success" && !isSuccess(h.status)) return false;
    if (f.status === "failed" && isSuccess(h.status)) return false;
    if (!q || h.adapterName.toLowerCase().includes(q)) return true;
    if (entryKind(h) === "dns") return dnsTexts(h, catalog).some((s) => s.includes(q));
    return hex !== null && hex !== "" && macMatches(h, hex);
  });
}

/** "before → after" for the change column. */
export function changeText(h: HistoryEntry, format: MacFormat): string {
  if (entryKind(h) === "dns") {
    const d = h.dns;
    return `${summaryText(d?.previous)} → ${summaryText(d?.observed ?? d?.requested)}`;
  }
  return `${formatMac(h.previousMac, format)} → ${formatMac(h.newMac ?? h.requestedMac, format)}`;
}
