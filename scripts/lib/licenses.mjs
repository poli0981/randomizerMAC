// License policy for the npm packages in package-lock.json — the npm counterpart of the
// `[licenses]` check cargo-deny runs for the Rust crates (deny.toml). npm 7+ lockfiles record each
// package's `license` field, so no download or extra dependency is needed.

/**
 * SPDX identifiers (and `WITH` exceptions) that may appear in the dependency tree: deny.toml's
 * allow-list plus the permissive licenses common on npm. Matching ignores case, as SPDX does.
 */
export const ALLOWED = [
  "0BSD",
  "Apache-2.0",
  "Apache-2.0 WITH LLVM-exception",
  "BlueOak-1.0.0",
  "BSD-2-Clause",
  "BSD-3-Clause",
  "CC0-1.0",
  "ISC",
  "MIT",
  "MIT-0",
  "MPL-2.0",
  "Unicode-3.0",
  "Unlicense",
  "Zlib",
];

const tokenize = (expression) => expression.match(/\(|\)|[^\s()]+/g) ?? [];

/**
 * Evaluates an SPDX license expression (`MIT`, `(MIT OR Apache-2.0)`, `MIT AND ISC`,
 * `Apache-2.0 WITH LLVM-exception`, nested parentheses). `OR` needs one allowed side, `AND` needs
 * both. Anything that does not parse — `UNLICENSED`, `SEE LICENSE IN …`, a dangling operator — is
 * not allowed.
 */
export function isAllowed(expression, allowed = ALLOWED) {
  if (typeof expression !== "string") return false;
  const permitted = new Set(allowed.map((id) => id.toLowerCase()));
  const tokens = tokenize(expression.trim());
  let pos = 0;
  const peek = () => tokens[pos]?.toUpperCase();
  const isOperator = (token) => ["AND", "OR", "WITH"].includes(token?.toUpperCase());

  const term = () => {
    if (peek() === "(") {
      pos += 1;
      const value = or();
      if (peek() !== ")") throw new Error("unbalanced parentheses");
      pos += 1;
      return value;
    }
    const id = tokens[pos];
    if (id === undefined || id === ")" || isOperator(id)) throw new Error("license expected");
    pos += 1;
    let license = id.replace(/\+$/, "");
    if (peek() === "WITH") {
      const exception = tokens[pos + 1];
      if (
        exception === undefined ||
        exception === "(" ||
        exception === ")" ||
        isOperator(exception)
      ) {
        throw new Error("exception expected");
      }
      pos += 2;
      license = `${license} WITH ${exception}`;
    }
    return permitted.has(license.toLowerCase());
  };
  const and = () => {
    let value = term();
    while (peek() === "AND") {
      pos += 1;
      const right = term();
      value = value && right;
    }
    return value;
  };
  const or = () => {
    let value = and();
    while (peek() === "OR") {
      pos += 1;
      const right = and();
      value = value || right;
    }
    return value;
  };

  try {
    const value = or();
    return pos === tokens.length && value;
  } catch {
    return false;
  }
}

/** The `license` field as an SPDX expression: legacy `{ type }` objects and arrays included. */
export function licenseOf(field) {
  if (typeof field === "string") return field.trim() || null;
  if (Array.isArray(field)) {
    const parts = field.map(licenseOf).filter(Boolean);
    return parts.length > 0 ? `(${parts.join(" OR ")})` : null;
  }
  if (field && typeof field === "object" && typeof field.type === "string") {
    return licenseOf(field.type);
  }
  return null;
}

/**
 * Checks every package of a parsed lockfile. Returns the number of packages, the problems
 * (`{ name, version, license }`, sorted by name) and how often each license occurs.
 */
export function checkLock(lock, allowed = ALLOWED) {
  const problems = [];
  const summary = new Map();
  let count = 0;
  for (const [key, entry] of Object.entries(lock?.packages ?? {})) {
    // "" is the project itself; links point to workspace folders, not to downloaded packages.
    if (key === "" || entry.link) continue;
    count += 1;
    const name = entry.name ?? key.slice(key.lastIndexOf("node_modules/") + "node_modules/".length);
    const license = licenseOf(entry.license);
    summary.set(license ?? "(none)", (summary.get(license ?? "(none)") ?? 0) + 1);
    if (!license || !isAllowed(license, allowed)) {
      problems.push({ name, version: entry.version, license });
    }
  }
  problems.sort((a, b) => a.name.localeCompare(b.name));
  return { count, problems, summary };
}
