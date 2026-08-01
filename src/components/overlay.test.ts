import { describe, expect, it, vi } from "vitest";
import { reconcileProviderLayers } from "./overlay";
import type { SnapshotMap, UsageSnapshot } from "../types";

const snapshot = (used: number): UsageSnapshot => ({
  windows: [{ label: "Weekly", used_percent: used, resets_at: 1_200_000 }],
  fetched_at: 1_000_000 + used,
  state: "fresh",
});

describe("reconcileProviderLayers", () => {
  it("never moves Claude data or styling into the ChatGPT node", () => {
    const content = document.createElement("div");
    const claude = { windows: [{ label: "Weekly", used_percent: 91, resets_at: 2_000_000 }], fetched_at: 1, state: "fresh" as const };
    const openai = { windows: [{ label: "Weekly", used_percent: 37, resets_at: 2_000_000 }], fetched_at: 1, state: "fresh" as const };
    const options = { snapshots: { claude, openai }, previousSnapshots: {}, now: 1_000_000, onAction: () => undefined };

    reconcileProviderLayers(content, ["claude", "openai"], options);
    const openaiNode = content.querySelector<HTMLElement>('[data-provider="openai"]')!;
    const openaiLogo = openaiNode.querySelector<HTMLImageElement>(".provider-mark img")!;
    const openaiMeter = openaiNode.querySelector<HTMLElement>(".meter")!;

    reconcileProviderLayers(content, ["claude", "openai"], {
      ...options,
      snapshots: { claude: { ...claude, windows: [{ ...claude.windows[0], used_percent: 94 }] }, openai },
    });
    reconcileProviderLayers(content, ["openai"], options);

    expect(content.querySelector('[data-provider="openai"]')).toBe(openaiNode);
    expect(openaiNode.textContent).toContain("37%");
    expect(openaiNode.textContent).not.toContain("94%");
    expect(openaiNode.dataset.provider).toBe("openai");
    expect(openaiMeter.dataset.provider).toBe("openai");
    expect(openaiLogo.src).toContain("/assets/chatgpt-logo.png");
    expect(content.querySelector('[data-provider="claude"]')).toBeNull();
  });

  it("does not move provider nodes when their order is unchanged", () => {
    const content = document.createElement("div");
    const options = {
      snapshots: { claude: snapshot(20), openai: snapshot(40) },
      previousSnapshots: {},
      now: 1_000_000,
      onAction: vi.fn(),
    };
    reconcileProviderLayers(content, ["claude", "openai"], options);
    const before = Array.from(content.querySelectorAll<HTMLElement>(".layer[data-provider]"));
    const appendSpy = vi.spyOn(content, "appendChild");
    const insertSpy = vi.spyOn(content, "insertBefore");

    reconcileProviderLayers(content, ["claude", "openai"], options);

    const after = Array.from(content.querySelectorAll<HTMLElement>(".layer[data-provider]"));
    expect(appendSpy).not.toHaveBeenCalled();
    expect(insertSpy).not.toHaveBeenCalled();
    expect(after).toEqual(before);
    expect(after[0].dataset.provider).toBe("claude");
    expect(after[1].dataset.provider).toBe("openai");
  });

  it("announces meaningful provider updates without repeating unchanged snapshots", () => {
    const content = document.createElement("div");
    const options = { snapshots: { claude: snapshot(20) }, previousSnapshots: {}, now: 1_000_000, onAction: vi.fn() };

    reconcileProviderLayers(content, ["claude"], options);

    const status = content.querySelector<HTMLElement>(".overlay-status")!;
    expect(status.getAttribute("aria-live")).toBe("polite");
    expect(status.textContent).toContain("Claude usage updated");
    const setText = vi.spyOn(status, "textContent", "set");

    reconcileProviderLayers(content, ["claude"], options);
    expect(setText).not.toHaveBeenCalled();

    reconcileProviderLayers(content, ["claude"], {
      ...options,
      snapshots: { claude: snapshot(25) },
    });
    expect(status.textContent).toContain("25 percent used");
    expect(setText).toHaveBeenCalledTimes(1);
  });

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
    expect(resolved.textContent).toContain("Sign-in required");
    expect(resolved.textContent).not.toContain("No active window");
    expect(resolved.textContent).toContain("Re-authenticate in the CLI");
  });
});
