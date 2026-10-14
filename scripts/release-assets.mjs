#!/usr/bin/env node
// Release helpers used by .github/workflows/release.yml.
//
//   node scripts/release-assets.mjs verify <dir>
//       Checks that exactly the expected installers, their updater signatures and the CLI
//       archives are present.
//   node scripts/release-assets.mjs latest-json <dir> <tag> <repo> <notes-file>
//       Writes <dir>/latest.json for tauri-plugin-updater with tag-based download URLs.
//       (Generated once in the finalize job instead of being merged by parallel build jobs.)
//   node scripts/release-assets.mjs sums <dir>
//       Writes <dir>/SHA256SUMS.txt covering every asset (and nothing else).

import { createHash, createPublicKey, verify as verifySignature } from "node:crypto";
import { readdirSync, readFileSync, writeFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");

const PATTERNS = {
  nsis: /_x64-setup\.exe$/,
  msi: /_x64_en-US\.msi$/,
  deb: /_amd64\.deb$/,
  rpm: /\.x86_64\.rpm$/,
  appimage: /_amd64\.AppImage$/,
  cliWindows: /^randommac-cli-.*-x86_64-pc-windows-msvc\.zip$/,
  cliLinux: /^randommac-cli-.*-x86_64-unknown-linux-gnu\.tar\.gz$/,
};
const SIGNED = ["nsis", "msi", "deb", "rpm", "appimage"];

function files(dir) {
  return readdirSync(dir).filter((f) => statSync(join(dir, f)).isFile());
}

function find(dir) {
  const all = files(dir);
  const found = {};
  const errors = [];
  for (const [kind, re] of Object.entries(PATTERNS)) {
    const matches = all.filter((f) => re.test(f));
    if (matches.length !== 1) {
      errors.push(
        `${kind}: expected exactly 1 file, found ${matches.length} (${matches.join(", ")})`,
      );
    } else {
      found[kind] = matches[0];
    }
  }
  for (const kind of SIGNED) {
    if (found[kind] && !all.includes(`${found[kind]}.sig`)) {
      errors.push(`${kind}: signature ${found[kind]}.sig is missing`);
    }
  }
  return { found, errors };
}

// minisign helpers: tauri-cli only warns when the signing key does not belong to the public key
// built into the app, which would publish updates no installed copy can verify.
const decodeMinisign = (text) => Buffer.from(text.trim(), "base64").toString("utf8").split("\n");

function updaterPublicKey() {
  const conf = JSON.parse(readFileSync(join(root, "src-tauri", "tauri.conf.json"), "utf8"));
  const raw = Buffer.from(decodeMinisign(conf.plugins.updater.pubkey)[1].trim(), "base64");
  const key = createPublicKey({
    key: { kty: "OKP", crv: "Ed25519", x: raw.subarray(10, 42).toString("base64url") },
    format: "jwk",
  });
  return { id: raw.subarray(2, 10), key };
}

/** Returns why `file` does not carry a valid signature by the app's updater key, or null. */
function signatureProblem(file, sigFile, pub) {
  const lines = decodeMinisign(readFileSync(sigFile, "utf8"));
  const blob = Buffer.from(lines[1]?.trim() ?? "", "base64");
  if (blob.length !== 74) return "malformed signature";
  if (!blob.subarray(2, 10).equals(pub.id)) return "signed with a different key";
  const sig = blob.subarray(10, 74);
  const data = readFileSync(file);
  const prehashed = blob.subarray(0, 2).toString("latin1") === "ED";
  const message = prehashed ? createHash("blake2b512").update(data).digest() : data;
  if (!verifySignature(null, message, pub.key, sig)) return "signature does not match the file";
  const trusted = Buffer.from((lines[2] ?? "").replace(/^trusted comment: /, ""));
  const global = Buffer.from(lines[3]?.trim() ?? "", "base64");
  if (!verifySignature(null, Buffer.concat([sig, trusted]), pub.key, global)) {
    return "invalid trusted comment signature";
  }
  return null;
}

const [cmd, dir, ...rest] = process.argv.slice(2);
if (!dir) {
  console.error("usage: node scripts/release-assets.mjs <verify|latest-json|sums> <dir> ...");
  process.exit(2);
}

if (cmd === "verify") {
  const { found, errors } = find(dir);
  const pub = updaterPublicKey();
  for (const kind of SIGNED) {
    if (!found[kind] || !files(dir).includes(`${found[kind]}.sig`)) continue;
    const problem = signatureProblem(join(dir, found[kind]), join(dir, `${found[kind]}.sig`), pub);
    if (problem) errors.push(`${kind}: ${found[kind]}.sig: ${problem}`);
  }
  if (errors.length) {
    for (const e of errors) console.error(`✗ ${e}`);
    process.exit(1);
  }
  for (const [k, f] of Object.entries(found)) console.log(`✓ ${k.padEnd(10)} ${f}`);
} else if (cmd === "latest-json") {
  const [tag, repo, notesFile] = rest;
  if (!tag || !repo) {
    console.error("usage: latest-json <dir> <tag> <owner/repo> [notes-file]");
    process.exit(2);
  }
  const { found, errors } = find(dir);
  if (errors.length) {
    for (const e of errors) console.error(`✗ ${e}`);
    process.exit(1);
  }
  const url = (f) => `https://github.com/${repo}/releases/download/${tag}/${encodeURIComponent(f)}`;
  const entry = (kind) => ({
    signature: readFileSync(join(dir, `${found[kind]}.sig`), "utf8").trim(),
    url: url(found[kind]),
  });
  const notes = notesFile ? readFileSync(notesFile, "utf8").trim() : "";
  const latest = {
    version: tag.replace(/^v/, ""),
    notes,
    pub_date: new Date().toISOString().replace(/\.\d{3}Z$/, "Z"),
    platforms: {
      "windows-x86_64-nsis": entry("nsis"),
      "windows-x86_64-msi": entry("msi"),
      "windows-x86_64": entry("nsis"),
      "linux-x86_64-appimage": entry("appimage"),
      "linux-x86_64-deb": entry("deb"),
      "linux-x86_64-rpm": entry("rpm"),
      "linux-x86_64": entry("appimage"),
    },
  };
  writeFileSync(join(dir, "latest.json"), JSON.stringify(latest, null, 2) + "\n");
  console.log(
    `latest.json written for ${tag} (${Object.keys(latest.platforms).length} platform keys)`,
  );
} else if (cmd === "sums") {
  const lines = files(dir)
    .filter((f) => f !== "SHA256SUMS.txt")
    .sort()
    .map(
      (f) =>
        `${createHash("sha256")
          .update(readFileSync(join(dir, f)))
          .digest("hex")}  ${f}`,
    );
  writeFileSync(join(dir, "SHA256SUMS.txt"), lines.join("\n") + "\n");
  console.log(`SHA256SUMS.txt: ${lines.length} files`);
} else {
  console.error(`unknown command ${cmd}`);
  process.exit(2);
}
