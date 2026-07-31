import { describe, expect, it } from "vitest";
import { nextSize, visibleLayers, worstPercent } from "./state";
import type { UsageSnapshot } from "./types";

const snap = (pcts: number[]): UsageSnapshot => ({
  windows: pcts.map((p) => ({ label: "w", used_percent: p, resets_at: 0 })),
  fetched_at: 0,
  state: "fresh",
});

describe("nextSize", () => {
  it("cycles compact to square", () => expect(nextSize("compact")).toBe("square"));
  it("cycles square back to compact", () => expect(nextSize("square")).toBe("compact"));
  it("restores bubble to compact", () => expect(nextSize("bubble")).toBe("compact"));
});

describe("worstPercent", () => {
  it("takes the maximum across providers", () => expect(worstPercent([snap([10, 20]), snap([55])])).toBe(55));
  it("returns null when there are no windows", () => expect(worstPercent([snap([])])).toBeNull());
});

describe("visibleLayers", () => {
  it("shows only claude when only claude is active", () => expect(visibleLayers({ claude: true, openai: false })).toEqual(["claude"]));
  it("shows only openai when only openai is active", () => expect(visibleLayers({ claude: false, openai: true })).toEqual(["openai"]));
  it("shows both when both are active", () => expect(visibleLayers({ claude: true, openai: true })).toEqual(["claude", "openai"]));
  it("shows nothing when neither is active", () => expect(visibleLayers({ claude: false, openai: false })).toEqual([]));
});
