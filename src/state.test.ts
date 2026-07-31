import { describe, expect, it } from "vitest";
import { geometryChanged, nextLayout, nextSize, sameSources, visibleLayers, worstPercent } from "./state";
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

describe("nextLayout", () => {
  it("switches compact stack to provider columns", () => expect(nextLayout("stacked-compact")).toBe("provider-columns"));
  it("switches provider columns back to compact stack", () => expect(nextLayout("provider-columns")).toBe("stacked-compact"));
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

describe("change detection", () => {
  const geometry = { monitorId: null, corner: "bottom-right", scale: 1, layout: "stacked-compact" as const };
  it("recognizes source changes", () => {
    expect(sameSources({ claude: true, openai: false }, { claude: true, openai: false })).toBe(true);
    expect(sameSources({ claude: true, openai: false }, { claude: false, openai: false })).toBe(false);
  });
  it("ignores non-geometry config changes", () => {
    expect(geometryChanged(geometry, geometry)).toBe(false);
    expect(geometryChanged(geometry, { ...geometry, scale: 1.25 })).toBe(true);
  });
});
