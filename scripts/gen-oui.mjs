#!/usr/bin/env node
// Generates the curated OUI vendor table used by randommac-core from the IEEE MA-L registry.
//
// Usage:
//   node scripts/gen-oui.mjs <path/to/oui.csv> [--source-date YYYY-MM-DD]
//
// The CSV is the official IEEE registry export (https://standards-oui.ieee.org/oui/oui.csv,
// also shipped by Debian/Ubuntu as /usr/share/ieee-data/oui.csv in the `ieee-data` package).
// Output:
//   crates/randommac-core/src/oui/vendors.rs  — vendor table (id, display name, imitable flag)
//   crates/randommac-core/src/oui/index.bin   — sorted 5-byte records: OUI (3 bytes) + vendor index (u16 LE)

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const outDir = join(root, "crates", "randommac-core", "src", "oui");

// id, display name, "imitable" (offered in the "imitate vendor" generator mode), org-name patterns.
const VENDORS = [
  ["apple", "Apple", true, [/^Apple, Inc\.$/]],
  ["samsung", "Samsung", true, [/^Samsung Electronics Co\.,\s?Ltd/i]],
  ["google", "Google", true, [/^Google, Inc\.$/]],
  ["microsoft", "Microsoft", true, [/^Microsoft/]],
  ["intel", "Intel", true, [/^Intel Corporat(e|ion)$/]],
  ["realtek", "Realtek", true, [/^REALTEK SEMICONDUCTOR/i]],
  ["qualcomm", "Qualcomm", true, [/^Qualcomm/i]],
  ["broadcom", "Broadcom", true, [/^Broadcom/]],
  ["mediatek", "MediaTek", true, [/^MediaTek/]],
  ["xiaomi", "Xiaomi", true, [/Xiaomi/]],
  ["huawei", "Huawei", true, [/^HUAWEI TECHNOLOGIES CO\.,LTD$/, /^Huawei Device Co/]],
  ["honor", "Honor", true, [/^Honor Device Co/]],
  ["oppo", "OPPO", true, [/^GUANGDONG OPPO MOBILE/]],
  ["vivo", "vivo", true, [/^vivo Mobile Communication/]],
  ["oneplus", "OnePlus", true, [/^OnePlus/]],
  ["sony", "Sony", true, [/^Sony Corporation$/, /^Sony Interactive Entertainment/]],
  ["lg", "LG Electronics", true, [/^LG Electronics/i]],
  ["motorola", "Motorola", true, [/^Motorola Mobility LLC, a Lenovo Company$/]],
  ["lenovo", "Lenovo", true, [/^Lenovo/]],
  ["dell", "Dell", true, [/^Dell/]],
  ["hp", "HP", true, [/^Hewlett Packard$/, /^HP Inc\.$/, /^Hewlett Packard Enterprise$/]],
  ["asus", "ASUS", true, [/^ASUSTek COMPUTER/]],
  ["amazon", "Amazon", true, [/^Amazon Technologies Inc\.$/]],
  ["nintendo", "Nintendo", true, [/^Nintendo Co\./]],
  ["tp-link", "TP-Link", true, [/^TP-LINK TECHNOLOGIES/i, /^TP-Link Systems/]],
  ["netgear", "NETGEAR", false, [/^NETGEAR$/]],
  ["d-link", "D-Link", false, [/^D-Link/i]],
  ["cisco", "Cisco", false, [/^Cisco Systems/]],
  ["linksys", "Linksys", false, [/^Cisco-Linksys/, /^The Linksys Group/, /^Linksys/]],
  ["zte", "ZTE", false, [/^zte corporation$/]],
  ["nokia", "Nokia", false, [/^Nokia( Corporation)?$/]],
  ["htc", "HTC", false, [/^HTC Corporation$/]],
  ["meta", "Meta", false, [/^Meta Platforms/, /^Facebook Inc/]],
  ["roku", "Roku", false, [/^Roku/]],
  ["sonos", "Sonos", false, [/^Sonos,? Inc/]],
  ["garmin", "Garmin", false, [/^Garmin International$/]],
  ["raspberry-pi", "Raspberry Pi", false, [/^Raspberry Pi/]],
  ["espressif", "Espressif", false, [/^Espressif/]],
  ["ubiquiti", "Ubiquiti", false, [/^Ubiquiti/]],
  ["mikrotik", "MikroTik", false, [/^Routerboard\.com$/]],
  ["zyxel", "Zyxel", false, [/^Zyxel Communications/]],
  ["belkin", "Belkin", false, [/^Belkin International/]],
  ["juniper", "Juniper Networks", false, [/^Juniper Networks$/]],
  ["arista", "Arista", false, [/^Arista Network/]],
  ["fortinet", "Fortinet", false, [/^Fortinet/]],
  ["synology", "Synology", false, [/^Synology/]],
  ["qnap", "QNAP", false, [/^QNAP Systems/]],
  ["murata", "Murata", false, [/^Murata Manufacturing/]],
  ["liteon", "Lite-On", false, [/^Liteon Technology Corporation$/]],
  ["azurewave", "AzureWave", false, [/^AzureWave Technolog/]],
  ["foxconn", "Hon Hai (Foxconn)", false, [/^Hon Hai Precision/i]],
  ["gigabyte", "GIGABYTE", false, [/^GIGA-BYTE TECHNOLOGY/i]],
  ["msi", "MSI", false, [/^Micro-Star/i]],
  ["asrock", "ASRock", false, [/^ASRock/]],
  ["nvidia", "NVIDIA", false, [/^NVIDIA/]],
  ["vmware", "VMware", false, [/^VMware, Inc\.$/]],
  ["virtualbox", "Oracle VirtualBox", false, [/^PCS Systemtechnik GmbH$/]],
  ["parallels", "Parallels", false, [/^Parallels, Inc\.$/]],
  ["xen", "Xen", false, [/^Xensource, Inc\.$/i]],
];

function parseCsv(text) {
  const rows = [];
  let row = [];
  let field = "";
  let quoted = false;
  for (let i = 0; i < text.length; i++) {
    const ch = text[i];
    if (quoted) {
      if (ch === '"') {
        if (text[i + 1] === '"') {
          field += '"';
          i++;
        } else {
          quoted = false;
        }
      } else {
        field += ch;
      }
    } else if (ch === '"') {
      quoted = true;
    } else if (ch === ",") {
      row.push(field);
      field = "";
    } else if (ch === "\n" || ch === "\r") {
      if (ch === "\r" && text[i + 1] === "\n") i++;
      row.push(field);
      rows.push(row);
      row = [];
      field = "";
    } else {
      field += ch;
    }
  }
  if (field.length > 0 || row.length > 0) {
    row.push(field);
    rows.push(row);
  }
  return rows;
}

const args = process.argv.slice(2);
const csvPath = args.find((a) => !a.startsWith("--"));
if (!csvPath) {
  console.error("usage: node scripts/gen-oui.mjs <oui.csv> [--source-date YYYY-MM-DD]");
  process.exit(2);
}
const dateIdx = args.indexOf("--source-date");
const sourceDate = dateIdx >= 0 ? args[dateIdx + 1] : new Date().toISOString().slice(0, 10);

const rows = parseCsv(readFileSync(csvPath, "utf8"));
const header = rows.shift();
if (!header || header[0] !== "Registry" || header[1] !== "Assignment") {
  console.error("unexpected CSV header:", header);
  process.exit(1);
}

const records = [];
const counts = new Array(VENDORS.length).fill(0);
for (const r of rows) {
  if (r.length < 3 || r[0] !== "MA-L") continue;
  const hex = r[1].trim().toUpperCase();
  if (!/^[0-9A-F]{6}$/.test(hex)) continue;
  const first = parseInt(hex.slice(0, 2), 16);
  if (first & 0x03) continue; // never index multicast or locally administered prefixes
  const org = r[2].trim();
  const idx = VENDORS.findIndex(([, , , pats]) => pats.some((p) => p.test(org)));
  if (idx < 0) continue;
  records.push([parseInt(hex, 16), idx]);
  counts[idx]++;
}
records.sort((a, b) => a[0] - b[0]);
for (let i = 1; i < records.length; i++) {
  if (records[i][0] === records[i - 1][0])
    throw new Error(`duplicate OUI ${records[i][0].toString(16)}`);
}
VENDORS.forEach(([id], i) => {
  if (counts[i] === 0) throw new Error(`vendor ${id} matched no OUIs — check its patterns`);
});

const bin = Buffer.alloc(records.length * 5);
records.forEach(([oui, idx], i) => {
  bin[i * 5] = (oui >> 16) & 0xff;
  bin[i * 5 + 1] = (oui >> 8) & 0xff;
  bin[i * 5 + 2] = oui & 0xff;
  bin.writeUInt16LE(idx, i * 5 + 3);
});
writeFileSync(join(outDir, "index.bin"), bin);

const esc = (s) => s.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
const lines = VENDORS.map(
  ([id, name, imitable], i) =>
    `    Vendor { id: "${esc(id)}", name: "${esc(name)}", imitable: ${imitable}, oui_count: ${counts[i]} },`,
);
const rs = `// @generated by scripts/gen-oui.mjs — do not edit by hand.
// Source: IEEE MA-L registry (oui.csv), snapshot ${sourceDate}; ${records.length} OUIs for ${VENDORS.length} vendors.

use super::Vendor;

pub(super) const SOURCE_DATE: &str = "${sourceDate}";

pub(super) static VENDORS: &[Vendor] = &[
${lines.join("\n")}
];
`;
writeFileSync(join(outDir, "vendors.rs"), rs);
console.log(`wrote ${records.length} OUIs for ${VENDORS.length} vendors (snapshot ${sourceDate})`);
