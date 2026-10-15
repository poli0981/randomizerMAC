import { describe, expect, it } from "vitest";
import vectors from "../../crates/randommac-core/tests/vectors/mac_parse.json";
import { canonical, formatMac, isLocallyAdministered, isMulticast, parseMac, sameMac } from "./mac";

describe("parseMac (shared vectors with the Rust core)", () => {
  it.each(vectors.valid)("accepts $input", ({ input, canonical: expected }) => {
    expect(canonical(input)).toBe(expected);
  });

  it.each(vectors.invalid.map((input) => ({ input })))("rejects $input", ({ input }) => {
    expect(parseMac(input)).toBeNull();
  });
});

describe("formatMac", () => {
  const mac = "02:AB:0C:D0:0E:FF";
  it("formats with every separator", () => {
    expect(formatMac(mac, { separator: "colon", uppercase: true })).toBe("02:AB:0C:D0:0E:FF");
    expect(formatMac(mac, { separator: "hyphen", uppercase: false })).toBe("02-ab-0c-d0-0e-ff");
    expect(formatMac(mac, { separator: "none", uppercase: true })).toBe("02AB0CD00EFF");
    expect(formatMac(mac, { separator: "dot", uppercase: false })).toBe("02ab.0cd0.0eff");
  });
  it("handles missing values", () => {
    expect(formatMac(null, { separator: "colon", uppercase: true })).toBe("—");
    expect(formatMac("garbage", { separator: "colon", uppercase: true })).toBe("garbage");
  });
});

describe("bit helpers", () => {
  it("detects locally administered and multicast addresses", () => {
    expect(isLocallyAdministered("02:00:00:00:00:01")).toBe(true);
    expect(isLocallyAdministered("00:1B:21:00:00:01")).toBe(false);
    expect(isMulticast("01:00:5E:00:00:01")).toBe(true);
    expect(sameMac("02-ab-0c-d0-0e-ff", "02:AB:0C:D0:0E:FF")).toBe(true);
    expect(sameMac(null, "02:AB:0C:D0:0E:FF")).toBe(false);
  });
});
