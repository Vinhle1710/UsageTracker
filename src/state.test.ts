import { describe, expect, it } from "vitest";
import { clearJustActivated, createProviderState, geometryChanged, initialSnapshots, providerJustActivated, providerSnapshots, readoutShapeChanged, sameSources, updateProviderCollapsed, updateProviderSources, updateProviderUsage, visibleLayers } from "./state";
import type { UsageSnapshot } from "./types";

const snap = (pcts: number[]): UsageSnapshot => ({
  windows: pcts.map((p) => ({ label: "w", used_percent: p, resets_at: 0 })),
  fetched_at: 0,
  state: "fresh",
});


describe("visibleLayers", () => {
  it("shows only claude when only claude is active", () => expect(visibleLayers({ claude: true, openai: false })).toEqual(["claude"]));
  it("shows only openai when only openai is active", () => expect(visibleLayers({ claude: false, openai: true })).toEqual(["openai"]));
  it("shows both when both are active", () => expect(visibleLayers({ claude: true, openai: true })).toEqual(["claude", "openai"]));
  it("shows nothing when neither is active", () => expect(visibleLayers({ claude: false, openai: false })).toEqual([]));
});

describe("change detection", () => {
  const geometry = { monitorId: null, corner: "bottom-right", scale: 1, layout: "stacked-compact" as const, theme: "frosted" as const, backgroundColor: "#07101f", cardOpacity: 0.96 };
  it("recognizes source changes", () => {
    expect(sameSources({ claude: true, openai: false }, { claude: true, openai: false })).toBe(true);
    expect(sameSources({ claude: true, openai: false }, { claude: false, openai: false })).toBe(false);
  });
  it("detects geometry and native-material changes", () => {
    expect(geometryChanged(geometry, geometry)).toBe(false);
    expect(geometryChanged(geometry, { ...geometry, scale: 1.25 })).toBe(true);
    expect(geometryChanged(geometry, { ...geometry, theme: "neon" })).toBe(true);
    expect(geometryChanged(geometry, { ...geometry, backgroundColor: "#203040" })).toBe(true);
    expect(geometryChanged(geometry, { ...geometry, cardOpacity: 0.84 })).toBe(true);
  });
  it("requests an immediate overlay rerender only when the readout shape changes", () => {
    expect(readoutShapeChanged({}, {})).toBe(false);
    expect(readoutShapeChanged({}, { meterShape: "ring" })).toBe(false);
    expect(readoutShapeChanged({ meterShape: "ring" }, { meterShape: "reactor" })).toBe(true);
  });
});

describe("provider usage state", () => {
  it("keeps provider records isolated across activation, close, reopen, polling, and minimize/restore", () => {
    const claude = snap([11]);
    const openai = snap([77]);
    let state = createProviderState({ claude: false, openai: false });

    state = updateProviderSources(state, { claude: true, openai: false });
    state = updateProviderUsage(state, { provider: "claude", snapshot: claude });
    state = updateProviderSources(state, { claude: false, openai: true });
    state = updateProviderUsage(state, { provider: "openai", snapshot: openai });
    state = updateProviderSources(state, { claude: true, openai: true });

    expect(providerSnapshots(state)).toEqual({ claude, openai });
    expect(state.claude.snapshot).toBe(claude);
    expect(state.openai.snapshot).toBe(openai);
  });

  it("retains each provider's collapsed state across close and reopen", () => {
    let state = createProviderState({ claude: true, openai: true });
    state = updateProviderCollapsed(state, "claude", true);
    state = updateProviderSources(state, { claude: false, openai: true });
    state = updateProviderSources(state, { claude: true, openai: true });

    expect(state.claude.collapsed).toBe(true);
    expect(state.openai.collapsed).toBe(false);
  });

  it("never seeds the native overlay with demo usage", () => {
    expect(initialSnapshots(false, 1_000_000)).toEqual({});
  });

  it("keeps browser-only preview data out of the native path", () => {
    expect(Object.keys(initialSnapshots(true, 1_000_000))).toEqual(["claude", "openai"]);
  });

  it("updates only the provider named by the event", () => {
    const claude = snap([11]);
    const openai = snap([77]);
    let state = updateProviderUsage(createProviderState({ claude: true, openai: true }), { provider: "claude", snapshot: claude });
    state = updateProviderUsage(state, { provider: "openai", snapshot: openai });
    expect(state.claude.snapshot).toBe(claude);
    expect(state.openai.snapshot).toBe(openai);
  });

  it("does not let a late response overwrite newer usage", () => {
    const live = { ...snap([77]), fetched_at: 200 };
    const late = { ...snap([11]), fetched_at: 100 };
    let state = updateProviderUsage(createProviderState({ claude: false, openai: true }), { provider: "openai", snapshot: live });
    state = updateProviderUsage(state, { provider: "openai", snapshot: late });
    expect(state.openai.snapshot).toBe(live);
  });
});

describe("provider activation entrance", () => {
  it("starts a newly-activated provider collapsed and flags it as just activated", () => {
    let state = createProviderState({ claude: false, openai: false });
    state = updateProviderSources(state, { claude: false, openai: true });

    expect(state.openai.collapsed).toBe(true);
    expect(state.openai.justActivated).toBe(true);
    expect(state.claude.justActivated).toBe(false);
    expect(providerJustActivated(state)).toEqual({ claude: false, openai: true });
  });

  it("does not re-flag a provider that was already active", () => {
    let state = createProviderState({ claude: true, openai: false });
    state = clearJustActivated(state);
    state = updateProviderSources(state, { claude: true, openai: false });

    expect(state.claude.justActivated).toBe(false);
  });

  it("does not reset a manually-collapsed provider's state on an unrelated source update", () => {
    let state = createProviderState({ claude: true, openai: true });
    state = updateProviderCollapsed(state, "claude", true);
    state = updateProviderSources(state, { claude: true, openai: false });

    expect(state.claude.collapsed).toBe(true);
    expect(state.claude.justActivated).toBe(false);
  });

  it("clears justActivated for both providers once consumed", () => {
    let state = createProviderState({ claude: false, openai: false });
    state = updateProviderSources(state, { claude: true, openai: true });
    state = clearJustActivated(state);

    expect(providerJustActivated(state)).toEqual({ claude: false, openai: false });
  });
});
