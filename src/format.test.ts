import { describe, expect, it } from "vitest";
import { formatCountdown, formatPercent, formatReset, formatWeeklyReset, getFunPlaceholder , formatMicros } from "./format";

describe("formatPercent", () => {
  it("rounds to a whole number", () => expect(formatPercent(25.4)).toBe("25%"));
  it("renders zero without hiding it", () => expect(formatPercent(0)).toBe("0%"));
});

describe("formatReset", () => {
  const now = 1_000_000;
  it("uses a live countdown for five-hour windows", () => expect(formatReset("5 hour", now + 7384, now)).toBe("resets in 02:03:04"));
  it("uses a date and a live countdown for weekly windows", () => {
    const resetsAt = 1_754_665_800; // Aug 08, 2025 (fixed reference, matches formatWeeklyReset tests below)
    const testNow = resetsAt - 90000; // 1 day + 1 hour before reset
    expect(formatReset("Weekly", resetsAt, testNow)).toMatch(/^Aug 08 · 1d \d{2}:\d{2}:\d{2}$/);
  });
  it("shows a short countdown for weekly windows under 24 hours", () => {
    const resetsAt = 1_754_665_800;
    const testNow = resetsAt - 7200; // 2 hours before reset
    expect(formatReset("Weekly", resetsAt, testNow)).toBe("Aug 08 · 02:00:00");
  });
  it("shows a day-prefixed countdown for weekly windows over 24 hours", () => {
    const resetsAt = 1_754_665_800;
    const testNow = resetsAt - 259200; // 3 days before reset
    expect(formatReset("Weekly", resetsAt, testNow)).toBe("Aug 08 · 3d 00:00:00");
  });
  it("reports an elapsed reset as due", () => expect(formatReset("5 hour", now - 10, now)).toBe("resets in 00:00:00"));
  it("does not invent a 1970 reset when the provider omits the time", () => {
    expect(formatReset("5 hour", 0, now)).toBe("reset time unavailable");
    expect(formatReset("Weekly", 0, now)).toBe("reset time unavailable");
  });
});

describe("formatCountdown", () => {
  it("pads hours, minutes, and seconds", () => expect(formatCountdown(7384)).toBe("02:03:04"));
  it("clamps an expired countdown to zero", () => expect(formatCountdown(-1)).toBe("00:00:00"));
});

describe("formatWeeklyReset", () => {
  it("includes only the reset month and day, no time-of-day", () => {
    const reset = new Date(2025, 7, 8, 14, 30).getTime() / 1000;
    expect(formatWeeklyReset(reset)).toBe("Aug 08");
  });
});

describe("getFunPlaceholder", () => {
  it("returns a non-empty string from the fun message pool", () => {
    const message = getFunPlaceholder();
    expect(typeof message).toBe("string");
    expect(message.length).toBeGreaterThan(0);
  });

  it("returns more than one distinct message across many calls", () => {
    const messages = new Set<string>();
    for (let i = 0; i < 50; i += 1) messages.add(getFunPlaceholder());
    expect(messages.size).toBeGreaterThan(1);
  });
});

describe("formatMicros", () => {
  it("renders micro-units as ordinary money", () => {
    expect(formatMicros(1_000_000)).toContain("1.00");
    expect(formatMicros(2_500_000)).toContain("2.50");
  });

  it("keeps sub-cent API pricing from rounding away to nothing", () => {
    expect(formatMicros(123_456)).toContain("0.1235");
    expect(formatMicros(400)).toContain("0.0004");
  });

  it("does not pad round amounts out to four decimals", () => {
    expect(formatMicros(3_000_000)).not.toContain("3.0000");
  });

  it("uses the currency the provider reported", () => {
    expect(formatMicros(1_000_000, "EUR")).toMatch(/€|EUR/);
  });

  it("renders zero as zero money rather than an empty value", () => {
    expect(formatMicros(0)).toContain("0.00");
  });
});
