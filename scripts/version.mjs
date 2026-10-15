#!/usr/bin/env node
// Single source of truth for the RandomMac version.
//
//   node scripts/version.mjs get                 print the workspace version
//   node scripts/version.mjs set 2.1.0           update Cargo.toml, package.json, package-lock.json, Cargo.lock
//   node scripts/version.mjs check [--tag v2.1.0] verify every file agrees (and matches the tag),
//                                                that docs/RELEASE_v<version>.md and a CHANGELOG
//                                                section exist when a tag is given.

import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const read = (p) => readFileSync(join(root, p), "utf8");
const write = (p, s) => writeFileSync(join(root, p), s);
const SEMVER = /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/;

function cargoVersion() {
  const toml = read("Cargo.toml");
  const section = toml.split("[workspace.package]")[1] ?? "";
  const m = section.match(/^version\s*=\s*"([^"]+)"/m);
  if (!m) throw new Error("workspace version not found in Cargo.toml");
  return m[1];
}

function lockVersions() {
  const lock = read("Cargo.lock");
  const out = {};
  for (const name of ["randommac", "randommac-core", "randommac-net", "randommac-cli"]) {
    const m = lock.match(new RegExp(`name = "${name}"\\nversion = "([^"]+)"`));
    out[name] = m?.[1];
  }
  return out;
}

function packageVersions() {
  const pkg = JSON.parse(read("package.json"));
  const lock = JSON.parse(read("package-lock.json"));
  return { pkg: pkg.version, lock: lock.version, lockRoot: lock.packages?.[""]?.version };
}

function tauriConfVersion() {
  return JSON.parse(read("src-tauri/tauri.conf.json")).version;
}

const [cmd, arg, ...rest] = process.argv.slice(2);

if (cmd === "get") {
  console.log(cargoVersion());
} else if (cmd === "set") {
  if (!arg || !SEMVER.test(arg)) {
    console.error("usage: node scripts/version.mjs set <semver>");
    process.exit(2);
  }
  const toml = read("Cargo.toml").replace(
    /(\[workspace\.package\][\s\S]*?^version\s*=\s*")[^"]+(")/m,
    `$1${arg}$2`,
  );
  write("Cargo.toml", toml);
  const pkg = JSON.parse(read("package.json"));
  pkg.version = arg;
  write("package.json", JSON.stringify(pkg, null, 2) + "\n");
  const lock = JSON.parse(read("package-lock.json"));
  lock.version = arg;
  if (lock.packages?.[""]) lock.packages[""].version = arg;
  write("package-lock.json", JSON.stringify(lock, null, 2) + "\n");
  try {
    execFileSync("cargo", ["update", "--workspace", "--offline"], { cwd: root, stdio: "inherit" });
  } catch {
    console.warn("cargo update --workspace failed; run it manually to refresh Cargo.lock");
  }
  console.log(`version set to ${arg}`);
} else if (cmd === "check") {
  const errors = [];
  const version = cargoVersion();
  if (!SEMVER.test(version)) errors.push(`Cargo.toml version ${version} is not semver`);
  const { pkg, lock, lockRoot } = packageVersions();
  for (const [name, v] of Object.entries({
    "package.json": pkg,
    "package-lock.json": lock,
    "package-lock.json (root)": lockRoot,
  })) {
    if (v !== version) errors.push(`${name} has ${v}, expected ${version}`);
  }
  for (const [name, v] of Object.entries(lockVersions())) {
    if (v !== version) errors.push(`Cargo.lock ${name} has ${v}, expected ${version}`);
  }
  if (tauriConfVersion() !== undefined) {
    errors.push("src-tauri/tauri.conf.json must not set `version` (it is read from Cargo.toml)");
  }
  const tagIndex = [arg, ...rest].indexOf("--tag");
  const tag = tagIndex >= 0 ? [arg, ...rest][tagIndex + 1] : undefined;
  if (tag !== undefined) {
    if (tag !== `v${version}`) errors.push(`tag ${tag} does not match version v${version}`);
    if (!existsSync(join(root, "docs", `RELEASE_v${version}.md`))) {
      errors.push(`docs/RELEASE_v${version}.md is missing`);
    }
    if (!read("CHANGELOG.md").includes(`## [${version}]`)) {
      errors.push(`CHANGELOG.md has no "## [${version}]" section`);
    }
    if (/-/.test(version) && !/^\d+\.\d+\.\d+-\d+$/.test(version)) {
      errors.push("MSI only supports numeric pre-release identifiers (e.g. 2.1.0-1)");
    }
  }
  if (errors.length) {
    for (const e of errors) console.error(`✗ ${e}`);
    process.exit(1);
  }
  console.log(`✓ version ${version} is consistent${tag ? ` with ${tag}` : ""}`);
} else {
  console.error("usage: node scripts/version.mjs <get|set <version>|check [--tag vX.Y.Z]>");
  process.exit(2);
}
