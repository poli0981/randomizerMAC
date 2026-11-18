#!/usr/bin/env node
// Checks the license of every npm package in package-lock.json against the allow-list in
// scripts/lib/licenses.mjs — the npm counterpart of `cargo deny check licenses` (deny.toml).
// CI runs it in the "Dependency audit" job; it reads only the repository's lockfile.
//
//   node scripts/check-licenses.mjs

import { readFileSync } from "node:fs";
import { checkLock } from "./lib/licenses.mjs";

const lockfile = new URL("../package-lock.json", import.meta.url);
const { count, problems, summary } = checkLock(JSON.parse(readFileSync(lockfile, "utf8")));

for (const [license, n] of [...summary].sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]))) {
  console.log(`${String(n).padStart(5)}  ${license}`);
}

if (problems.length > 0) {
  console.error(`\n${problems.length} of ${count} packages have a license that is not allowed:`);
  for (const p of problems) {
    console.error(`  ${p.name}@${p.version}: ${p.license ?? "no license field"}`);
  }
  console.error("\nAllow the license in scripts/lib/licenses.mjs only after checking its terms.");
  process.exit(1);
}
console.log(`\n${count} packages, every license allowed.`);
