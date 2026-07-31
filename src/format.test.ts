import { describe, expect, it } from "vitest";
import { formatCountdown, formatPercent, formatReset, formatWeeklyReset } from "./format";

describe("formatPercent", () => {
  it("rounds to a whole number", () => expect(formatPercent(25.4)).toBe("25%"));
  it("renders zero without hiding it", () => expect(formatPercent(0)).toBe("0%"));
});

describe("formatReset", () => {
  const now = 1_000_000;
  it("uses a live countdown for five-hour windows", () => expect(formatReset("5 hour", now + 7384, now)).toBe("resets in 02:03:04"));
  it("uses a date and time for weekly windows", () => expect(formatReset("Weekly", 1_754_665_800, now)).toContain("Aug"));
  it("reports an elapsed reset as due", () => expect(formatReset("5 hour", now - 10, now)).toBe("resets in 00:00:00"));
});

describe("formatCountdown", () => {
  it("pads hours, minutes, and seconds", () => expect(formatCountdown(7384)).toBe("02:03:04"));
  it("clamps an expired countdown to zero", () => expect(formatCountdown(-1)).toBe("00:00:00"));
});

describe("formatWeeklyReset", () => {
  it("includes the reset month, day, and local time", () => {
    const reset = new Date(2025, 7, 8, 14, 30).getTime() / 1000;
    expect(formatWeeklyReset(reset)).toBe("Aug 08 · 14:30");
  });
});
