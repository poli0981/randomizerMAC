import { describe, expect, it } from "vitest";
import { ALLOWED, checkLock, isAllowed, licenseOf } from "./licenses.mjs";

describe("isAllowed", () => {
  it.each([
    "MIT",
    "mit",
    "Apache-2.0",
    "(MIT OR Apache-2.0)",
    "Apache-2.0 OR MIT",
    "MIT OR GPL-3.0-only",
    "MIT AND ISC",
    "(MPL-2.0 OR Apache-2.0)",
    "((MIT))",
    "(MIT AND ISC) OR GPL-3.0-only",
    "Apache-2.0 WITH LLVM-exception",
    "apache-2.0 with llvm-exception",
    "MPL-2.0+",
  ])("allows %s", (expression) => {
    expect(isAllowed(expression)).toBe(true);
  });

  it.each([
    "GPL-3.0-only",
    "MIT AND GPL-3.0-only",
    "GPL-2.0 WITH Classpath-exception-2.0",
    "MIT WITH LLVM-exception",
    "UNLICENSED",
    "SEE LICENSE IN LICENSE.txt",
    "",
    "MIT OR",
    "OR MIT",
    "(MIT",
    "MIT)",
    "MIT WITH",
    "()",
  ])("refuses %j", (expression) => {
    expect(isAllowed(expression)).toBe(false);
  });

  it("refuses values that are not strings", () => {
    expect(isAllowed(undefined)).toBe(false);
    expect(isAllowed(null)).toBe(false);
    expect(isAllowed(42)).toBe(false);
  });

  it("uses the given allow-list", () => {
    expect(isAllowed("GPL-3.0-only", ["GPL-3.0-only"])).toBe(true);
    expect(isAllowed("MIT", ["ISC"])).toBe(false);
  });

  it("allows every identifier on its own list", () => {
    for (const id of ALLOWED) expect(isAllowed(id), id).toBe(true);
  });
});

describe("licenseOf", () => {
  it("reads strings, legacy objects and arrays", () => {
    expect(licenseOf(" MIT ")).toBe("MIT");
    expect(licenseOf({ type: "ISC", url: "https://example.org" })).toBe("ISC");
    expect(licenseOf(["MIT", { type: "Apache-2.0" }])).toBe("(MIT OR Apache-2.0)");
    expect(licenseOf("")).toBeNull();
    expect(licenseOf([])).toBeNull();
    expect(licenseOf({ url: "x" })).toBeNull();
    expect(licenseOf(undefined)).toBeNull();
  });
});

describe("checkLock", () => {
  const lock = {
    lockfileVersion: 3,
    packages: {
      "": { name: "randommac", license: "MIT" },
      "node_modules/a": { version: "1.0.0", license: "MIT" },
      "node_modules/@scope/b": { version: "2.0.0", license: "(MIT OR Apache-2.0)" },
      "node_modules/a/node_modules/c": { version: "3.0.0", license: "GPL-3.0-only" },
      "node_modules/d": { version: "4.0.0" },
      "node_modules/e": { version: "5.0.0", license: { type: "ISC" } },
      "node_modules/linked": { resolved: "packages/linked", link: true },
      "node_modules/alias": { name: "real-name", version: "6.0.0", license: "BUSL-1.1" },
    },
  };

  it("reports packages whose license is missing or not allowed", () => {
    const { count, problems } = checkLock(lock);
    expect(count).toBe(6);
    expect(problems).toEqual([
      { name: "c", version: "3.0.0", license: "GPL-3.0-only" },
      { name: "d", version: "4.0.0", license: null },
      { name: "real-name", version: "6.0.0", license: "BUSL-1.1" },
    ]);
  });

  it("counts each license", () => {
    const { summary } = checkLock(lock);
    expect(summary.get("MIT")).toBe(1);
    expect(summary.get("(none)")).toBe(1);
    expect(summary.get("ISC")).toBe(1);
  });

  it("accepts an empty or odd lockfile", () => {
    expect(checkLock({}).count).toBe(0);
    expect(checkLock(null).problems).toEqual([]);
  });
});
