#!/usr/bin/env node
// Builds randommac-cli in release mode and copies it to src-tauri/binaries/ with the target-triple
// suffix Tauri expects for `bundle.externalBin` (used by tauri.release.conf.json), so .deb/.rpm
// packages install it next to the app (/usr/bin) and the Windows installer puts it in the app folder.

import { execFileSync } from "node:child_process";
import { copyFileSync, mkdirSync, chmodSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const triple =
  process.argv[2] ?? execFileSync("rustc", ["--print", "host-tuple"], { encoding: "utf8" }).trim();
const exe = triple.includes("windows") ? ".exe" : "";

execFileSync(
  "cargo",
  ["build", "--release", "--locked", "-p", "randommac-cli", "--target", triple],
  {
    cwd: root,
    stdio: "inherit",
  },
);

const from = join(root, "target", triple, "release", `randommac-cli${exe}`);
const outDir = join(root, "src-tauri", "binaries");
mkdirSync(outDir, { recursive: true });
const to = join(outDir, `randommac-cli-${triple}${exe}`);
copyFileSync(from, to);
if (!exe) chmodSync(to, 0o755);
console.log(`sidecar ready: ${to}`);
