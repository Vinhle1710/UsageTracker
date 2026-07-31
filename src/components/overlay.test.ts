import { describe, expect, it, vi } from "vitest";
import { reconcileProviderLayers } from "./overlay";
import type { SnapshotMap, UsageSnapshot } from "../types";

const snapshot = (used: number): UsageSnapshot => ({
  windows: [{ label: "Weekly", used_percent: used, resets_at: 1_200_000 }],
  fetched_at: 1_000_000 + used,
  state: "fresh",
});

describe("reconcileProviderLayers", () => {
  it("preserves an unchanged provider card when another provider closes", () => {
    const content = document.createElement("div");
    const snapshots: SnapshotMap = { claude: snapshot(20), openai: snapshot(40) };
    const options = { snapshots, previousSnapshots: {}, now: 1_000_000, onAction: vi.fn() };
    reconcileProviderLayers(content, ["claude", "openai"], options);
    const openai = content.querySelector<HTMLElement>('[data-provider="openai"]')!;

    reconcileProviderLayers(content, ["openai"], options);

    expect(content.querySelector('[data-provider="claude"]')).toBeNull();
    expect(content.querySelector('[data-provider="openai"]')).toBe(openai);
    expect(openai.textContent).toContain("40%");
  });

  it("updates progress in place without replacing the card or ring", () => {
    const content = document.createElement("div");
    const options = { snapshots: { openai: snapshot(40) }, previousSnapshots: {}, now: 1_000_000, onAction: vi.fn() };
    reconcileProviderLayers(content, ["openai"], options);
    const layer = content.querySelector<HTMLElement>('[data-provider="openai"]')!;
    const ring = layer.querySelector(".meter__progress");

    reconcileProviderLayers(content, ["openai"], { ...options, snapshots: { openai: snapshot(55) } });

    expect(content.querySelector('[data-provider="openai"]')).toBe(layer);
    expect(layer.querySelector(".meter__progress")).toBe(ring);
    expect(layer.textContent).toContain("55%");
  });

  it("replaces the loading shell even when the resolved snapshot is empty", () => {
    const content = document.createElement("div");
    const base = { snapshots: {}, previousSnapshots: {}, now: 1_000_000, onAction: vi.fn() };
    reconcileProviderLayers(content, ["claude"], base);
    const loading = content.querySelector<HTMLElement>('[data-provider="claude"]')!;

    reconcileProviderLayers(content, ["claude"], {
      ...base,
      snapshots: {
        claude: { windows: [], fetched_at: 1_000_001, state: "error" },
      },
    });

    const resolved = content.querySelector<HTMLElement>('[data-provider="claude"]')!;
    expect(resolved).not.toBe(loading);
    expect(resolved.textContent).toContain("No active window");
    expect(resolved.textContent).toContain("Re-authenticate in the CLI");
  });
});
