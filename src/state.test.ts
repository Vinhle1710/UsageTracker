import { describe, expect, it } from "vitest";
import { applyUsageEvent, geometryChanged, initialSnapshots, mergeBootstrap, nextLayout, nextSize, sameSources, visibleLayers, worstPercent } from "./state";
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
  const geometry = { monitorId: null, corner: "bottom-right", scale: 1, layout: "stacked-compact" as const, theme: "acrylic" as const, backgroundColor: "#07101f", cardOpacity: 0.96 };
  it("recognizes source changes", () => {
    expect(sameSources({ claude: true, openai: false }, { claude: true, openai: false })).toBe(true);
    expect(sameSources({ claude: true, openai: false }, { claude: false, openai: false })).toBe(false);
  });
  it("detects geometry and native-material changes", () => {
    expect(geometryChanged(geometry, geometry)).toBe(false);
    expect(geometryChanged(geometry, { ...geometry, scale: 1.25 })).toBe(true);
    expect(geometryChanged(geometry, { ...geometry, theme: "blur" })).toBe(true);
    expect(geometryChanged(geometry, { ...geometry, backgroundColor: "#203040" })).toBe(true);
    expect(geometryChanged(geometry, { ...geometry, cardOpacity: 0.84 })).toBe(true);
  });
});

describe("provider usage state", () => {
  it("never seeds the native overlay with demo usage", () => {
    expect(initialSnapshots(false, 1_000_000)).toEqual({});
  });

  it("keeps browser-only preview data out of the native path", () => {
    expect(Object.keys(initialSnapshots(true, 1_000_000))).toEqual(["claude", "openai"]);
  });

  it("updates only the provider named by the event", () => {
    const claude = snap([11]);
    const openai = snap([77]);
    const result = applyUsageEvent({ claude }, { provider: "openai", snapshot: openai });
    expect(result.claude).toBe(claude);
    expect(result.openai).toBe(openai);
  });

  it("does not let a late bootstrap response overwrite newer events", () => {
    const live = { ...snap([77]), fetched_at: 200 };
    const boot = { ...snap([11]), fetched_at: 100 };
    expect(mergeBootstrap({ openai: live }, [{ provider: "openai", snapshot: boot }]).openai).toBe(live);
  });
});
