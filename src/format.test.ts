import { describe, expect, it } from "vitest";
import { formatAge, formatPercent, formatReset } from "./format";

describe("formatPercent", () => {
  it("rounds to a whole number", () => expect(formatPercent(25.4)).toBe("25%"));
  it("renders zero without hiding it", () => expect(formatPercent(0)).toBe("0%"));
});

describe("formatReset", () => {
  const now = 1_000_000;
  it("renders minutes under an hour", () => expect(formatReset(now + 1800, now)).toBe("resets in 30m"));
  it("renders hours under a day", () => expect(formatReset(now + 7200, now)).toBe("resets in 2h"));
  it("renders days beyond 24 hours", () => expect(formatReset(now + 259200, now)).toBe("resets in 3d"));
  it("reports an elapsed reset as due", () => expect(formatReset(now - 10, now)).toBe("resetting"));
});

describe("formatAge", () => {
  const now = 1_000_000;
  it("treats the last minute as just now", () => expect(formatAge(now - 5, now)).toBe("just now"));
  it("renders whole minutes", () => expect(formatAge(now - 120, now)).toBe("2m ago"));
  it("renders hours", () => expect(formatAge(now - 7200, now)).toBe("2h ago"));
});
