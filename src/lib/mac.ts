// Client-side MAC helpers for live input feedback. The Rust core remains authoritative; both
// parsers are tested against crates/randommac-core/tests/vectors/mac_parse.json.
import type { MacFormat } from "./types";

/** Parses the shapes accepted by the Rust core; returns 6 bytes or null. */
export function parseMac(input: string): number[] | null {
  const s = input.trim();
  const chars = [...s];
  let hex: string[];
  if (chars.length === 12) {
    hex = chars;
  } else if (chars.length === 14) {
    if (chars[4] !== "." || chars[9] !== ".") return null;
    hex = chars.filter((_, i) => i !== 4 && i !== 9);
  } else if (chars.length === 17) {
    const sep = chars[2];
    if (sep !== ":" && sep !== "-") return null;
    for (let i = 2; i < 17; i += 3) {
      if (chars[i] !== sep) return null;
    }
    hex = chars.filter((_, i) => i % 3 !== 2);
  } else {
    return null;
  }
  if (!hex.every((c) => /^[0-9a-fA-F]$/.test(c))) return null;
  const bytes: number[] = [];
  for (let i = 0; i < 12; i += 2) {
    bytes.push(parseInt(hex[i]! + hex[i + 1]!, 16));
  }
  return bytes;
}

export function canonical(input: string): string | null {
  const b = parseMac(input);
  return b ? b.map((x) => x.toString(16).padStart(2, "0").toUpperCase()).join(":") : null;
}

/** Formats a canonical/parsable MAC with the user's display preferences. */
export function formatMac(mac: string | null | undefined, format: MacFormat): string {
  if (!mac) return "—";
  const b = parseMac(mac);
  if (!b) return mac;
  let parts = b.map((x) => x.toString(16).padStart(2, "0"));
  if (format.uppercase) parts = parts.map((p) => p.toUpperCase());
  switch (format.separator) {
    case "colon":
      return parts.join(":");
    case "hyphen":
      return parts.join("-");
    case "none":
      return parts.join("");
    case "dot":
      return [0, 2, 4].map((i) => parts[i]! + parts[i + 1]!).join(".");
  }
}

export function isLocallyAdministered(mac: string): boolean {
  const b = parseMac(mac);
  return !!b && (b[0]! & 0x02) !== 0;
}

export function isMulticast(mac: string): boolean {
  const b = parseMac(mac);
  return !!b && (b[0]! & 0x01) !== 0;
}

export function sameMac(a: string | null | undefined, b: string | null | undefined): boolean {
  if (!a || !b) return false;
  const x = canonical(a);
  return x !== null && x === canonical(b);
}
