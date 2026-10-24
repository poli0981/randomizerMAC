#!/usr/bin/env node
// Rewrites the source paths (`SF:` lines) of LCOV reports to repository-relative paths with
// forward slashes, so the reports of the Windows and Linux CI runners can be merged by
// SonarQube Cloud (cargo-llvm-cov writes absolute paths, e.g. `D:\a\…\crates\…\mac.rs`).
//
//   node scripts/lcov-paths.mjs coverage/rust-Linux.info [more.info ...]

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, isAbsolute, join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const files = process.argv.slice(2);
if (files.length === 0) {
  console.error("usage: node scripts/lcov-paths.mjs <lcov file>...");
  process.exit(2);
}

for (const file of files) {
  let count = 0;
  const lcov = readFileSync(file, "utf8").replace(/^SF:(.+?)\r?$/gm, (_, path) => {
    count += 1;
    const rel = isAbsolute(path) ? relative(root, path) : path;
    return `SF:${rel.split(sep).join("/")}`;
  });
  writeFileSync(file, lcov);
  console.log(`${file}: ${count} source files`);
}
