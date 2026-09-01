import { describe, expect, it, vi } from "vitest";
import { reconcileOverlayLayout, reconcileProviderLayers, reconcileTuckControl, TUCK_REGION_SELECTOR } from "./overlay";
import type { SnapshotMap, UsageSnapshot } from "../types";

const snapshot = (used: number): UsageSnapshot => ({
  windows: [{ label: "Weekly", used_percent: used, resets_at: 1_200_000 }],
  fetched_at: 1_000_000 + used,
  state: "fresh",
});

const minimalSnapshot = (fiveHour: number, weekly: number): UsageSnapshot => ({
  windows: [
    { label: "5 hour", used_percent: fiveHour, resets_at: 1_100_000 },
    { label: "Weekly", used_percent: weekly, resets_at: 1_200_000 },
  ],
  fetched_at: 1_000_000 + fiveHour,
  state: "fresh",
});

describe("reconcileOverlayLayout", () => {
  it("switches between card and Minimal renderers without leaving stale controls", () => {
    const content = document.createElement("div");
    const base = {
      snapshots: { claude: minimalSnapshot(21, 48), openai: minimalSnapshot(34, 67) },
      previousSnapshots: {},
      now: 1_000_000,
      meterShape: "ring" as const,
      corner: "bottom-right",
      onAction: vi.fn(),
      onGeometryChange: vi.fn(async () => undefined),
    };

    reconcileOverlayLayout(content, ["claude", "openai"], { ...base, layout: "stacked-compact" });
    expect(content.querySelectorAll(".layer")).toHaveLength(2);

    reconcileOverlayLayout(content, ["claude", "openai"], { ...base, layout: "minimal" });
    expect(content.querySelectorAll(".minimal-readout__provider")).toHaveLength(2);
    expect(content.querySelector(".layer")).toBeNull();
    expect(content.querySelector(".provider-bubble")).toBeNull();
    expect(content.querySelector(".minimize-control")).toBeNull();

    reconcileOverlayLayout(content, ["claude", "openai"], { ...base, layout: "provider-columns" });
    expect(content.querySelectorAll(".layer")).toHaveLength(2);
    expect(content.querySelector(".minimal-readout")).toBeNull();
  });
});

describe("reconcileProviderLayers", () => {
  it("collapses and restores each provider independently with stable bubble order", () => {
    const content = document.createElement("div");
    const onAction = vi.fn();
    const options = {
      snapshots: { claude: snapshot(20), openai: snapshot(40) },
      previousSnapshots: {},
      now: 1_000_000,
      collapsed: { claude: false, openai: true },
      onAction,
    };

    reconcileProviderLayers(content, ["claude", "openai"], options);

    expect(content.querySelectorAll('.layer[data-provider]')).toHaveLength(1);
    expect(content.querySelector('[data-provider="claude"]')?.textContent).toContain("20%");
    expect(content.querySelector('.layer[data-provider="openai"]')).toBeNull();
    expect(Array.from(content.querySelectorAll<HTMLButtonElement>(".provider-bubble")).map((bubble) => bubble.dataset.provider))
      .toEqual(["openai"]);
    expect(content.querySelector<HTMLButtonElement>('.provider-bubble[data-provider="openai"]')?.getAttribute("aria-label"))
      .toBe("Expand ChatGPT usage");

    reconcileProviderLayers(content, ["claude", "openai"], {
      ...options,
      collapsed: { claude: false, openai: false },
    });
    expect(content.querySelectorAll('.layer[data-provider]')).toHaveLength(2);
    expect(content.querySelectorAll(".provider-bubble")).toHaveLength(0);

    reconcileProviderLayers(content, ["claude", "openai"], {
      ...options,
      collapsed: { claude: true, openai: true },
    });
    expect(content.querySelectorAll('.layer[data-provider]')).toHaveLength(0);
    expect(Array.from(content.querySelectorAll<HTMLButtonElement>(".provider-bubble")).map((bubble) => bubble.dataset.provider))
      .toEqual(["claude", "openai"]);

    content.querySelector<HTMLButtonElement>('.provider-bubble[data-provider="claude"]')!.click();
    expect(onAction).toHaveBeenCalledWith({ action: "restore", provider: "claude" });
  });

  it("keeps the other card identity and provider logo/data isolated while one provider is minimized", () => {
    const content = document.createElement("div");
    const options = {
      snapshots: { claude: snapshot(20), openai: snapshot(40) },
      previousSnapshots: {},
      now: 1_000_000,
      collapsed: { claude: false, openai: false },
      onAction: vi.fn(),
    };
    reconcileProviderLayers(content, ["claude", "openai"], options);
    const openai = content.querySelector<HTMLElement>('[data-provider="openai"]')!;
    const openaiLogo = openai.querySelector<HTMLImageElement>(".provider-mark img")!;

    reconcileProviderLayers(content, ["claude", "openai"], {
      ...options,
      collapsed: { claude: true, openai: false },
      snapshots: { claude: snapshot(91), openai: snapshot(44) },
    });

    expect(content.querySelector('.layer[data-provider="claude"]')).toBeNull();
    expect(content.querySelector('[data-provider="openai"]')).toBe(openai);
    expect(openai.textContent).toContain("44%");
    expect(openai.textContent).not.toContain("91%");
    expect(openaiLogo.src).toContain("chatgpt-logo.png");
    expect(content.querySelector<HTMLImageElement>('.provider-bubble[data-provider="claude"] img')?.src)
      .toContain("claude-logo.png");
  });

  it("focuses the new bubble or restored provider control by provider key", async () => {
    const content = document.createElement("div");
    document.body.appendChild(content);
    const options = {
      snapshots: { claude: snapshot(20), openai: snapshot(40) },
      previousSnapshots: {},
      now: 1_000_000,
      collapsed: { claude: false, openai: false },
      onAction: vi.fn(),
    };

    reconcileProviderLayers(content, ["claude", "openai"], { ...options, focusProvider: "claude" });
    await Promise.resolve();
    expect(document.activeElement).toBe(content.querySelector<HTMLButtonElement>('[data-provider="claude"] .minimize-control__button'));

    reconcileProviderLayers(content, ["claude", "openai"], {
      ...options,
      collapsed: { claude: true, openai: false },
      focusProvider: "claude",
    });
    await Promise.resolve();
    expect(document.activeElement).toBe(content.querySelector<HTMLButtonElement>('.provider-bubble[data-provider="claude"]'));
  });

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

  it("plays the burst-entrance animation only on a freshly-created bubble for a just-activated provider", () => {
    const content = document.createElement("div");
    const options = {
      snapshots: { openai: snapshot(40) },
      previousSnapshots: {},
      now: 1_000_000,
      collapsed: { claude: false, openai: true },
      burstProviders: { claude: false, openai: true },
      onAction: vi.fn(),
    };

    reconcileProviderLayers(content, ["openai"], options);
    const bubble = content.querySelector<HTMLButtonElement>('.provider-bubble[data-provider="openai"]')!;
    expect(bubble.classList.contains("provider-bubble--burst")).toBe(true);

    reconcileProviderLayers(content, ["openai"], { ...options, burstProviders: { claude: false, openai: false } });
    expect(content.querySelector('.provider-bubble[data-provider="openai"]')).toBe(bubble);
    expect(bubble.classList.contains("provider-bubble--burst")).toBe(true);
  });

  it("does not burst a bubble created by manual minimize", () => {
    const content = document.createElement("div");
    const options = {
      snapshots: { openai: snapshot(40) },
      previousSnapshots: {},
      now: 1_000_000,
      collapsed: { claude: false, openai: true },
      onAction: vi.fn(),
    };

    reconcileProviderLayers(content, ["openai"], options);
    const bubble = content.querySelector<HTMLButtonElement>('.provider-bubble[data-provider="openai"]')!;
    expect(bubble.classList.contains("provider-bubble--burst")).toBe(false);
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
    expect(resolved.textContent).toContain("Re-authenticate in Claude Code");
  });

  it("announces a signed-out provider as not signed in rather than as a sign-in failure", () => {
    const content = document.createElement("div");

    reconcileProviderLayers(content, ["claude"], {
      snapshots: { claude: { windows: [], fetched_at: 1_000_000, state: "signed-out" } },
      previousSnapshots: {},
      now: 1_000_000,
      onAction: vi.fn(),
    });

    const status = content.querySelector<HTMLElement>(".overlay-status")!;
    expect(status.textContent).toContain("Claude status: Not signed in");
    expect(status.textContent).not.toContain("Sign-in required");
  });

  it("keeps the tuck tab out of the stack so nothing it renders can move the tab", () => {
    // The stack is what the tuck animation slides away and what changes box when a card
    // collapses. A tab parented into it inherits both, and the tab must never move.
    const host = document.createElement("div");
    const content = document.createElement("div");
    content.className = "layers";
    host.appendChild(content);

    reconcileProviderLayers(content, ["claude", "openai"], {
      snapshots: { claude: snapshot(20), openai: snapshot(40) },
      previousSnapshots: {},
      now: 1_000_000,
      collapsed: { claude: true, openai: true },
      onAction: vi.fn(),
    });
    reconcileTuckControl(host, "bottom-right", vi.fn());

    expect(content.querySelector(".tuck-control")).toBeNull();
    expect(host.querySelector(".tuck-control")?.parentElement).toBe(host);
    expect(content.querySelectorAll(".provider-bubble-row .provider-bubble")).toHaveLength(2);
  });

  it("offers the tuck tab whether or not anything is collapsed", () => {
    const host = document.createElement("div");
    const onTuck = vi.fn();

    reconcileTuckControl(host, "bottom-right", onTuck);
    const button = host.querySelector<HTMLButtonElement>(".tuck-control .usage-tab__button")!;
    expect(button).not.toBeNull();
    button.click();
    expect(onTuck).toHaveBeenCalledOnce();

    reconcileTuckControl(host, "bottom-right", undefined);
    expect(host.querySelector(".tuck-control")).toBeNull();
  });

  it("opens settings from the edge controls without tucking the overlay", () => {
    const host = document.createElement("div");
    const onTuck = vi.fn();
    const onOpenSettings = vi.fn();

    reconcileTuckControl(host, "bottom-right", onTuck, onOpenSettings);
    host.querySelector<HTMLButtonElement>(".usage-tab__settings-button")!.click();

    expect(onOpenSettings).toHaveBeenCalledOnce();
    expect(onTuck).not.toHaveBeenCalled();
  });

  it("re-points the tuck tab when the overlay moves to the other screen edge", () => {
    const host = document.createElement("div");

    reconcileTuckControl(host, "bottom-right", vi.fn());
    expect(host.querySelector<HTMLElement>(".tuck-control")?.dataset.edge).toBe("right");

    reconcileTuckControl(host, "top-left", vi.fn());
    expect(host.querySelectorAll(".tuck-control")).toHaveLength(1);
    expect(host.querySelector<HTMLElement>(".tuck-control")?.dataset.edge).toBe("left");
  });

  it("leaves the tab node untouched when nothing about it changed", () => {
    // Re-appending it on every render would restart any animation it carries and, once the
    // overlay is animating around it, is exactly how a "fixed" element starts drifting.
    const host = document.createElement("div");
    reconcileTuckControl(host, "bottom-right", vi.fn());
    const first = host.querySelector(".tuck-control");
    reconcileTuckControl(host, "bottom-right", vi.fn());
    expect(host.querySelector(".tuck-control")).toBe(first);
  });

  it("counts the tuck tab among the shapes the native window is clipped to", () => {
    // The window region is the union of the measured rects; anything painted outside it is
    // clipped away by the OS, so a control missing from this selector renders and stays unseen.
    const host = document.createElement("div");
    reconcileTuckControl(host, "bottom-right", vi.fn());
    const measured = Array.from(host.querySelectorAll(TUCK_REGION_SELECTOR));
    expect(measured).toEqual([host.querySelector(".tuck-control .usage-tab__button")]);
    // Never matches a bubble: counting one as an "extra" flips geometry.ts's padding heuristics.
    expect(TUCK_REGION_SELECTOR).not.toContain(".provider-bubble");
  });

  it("mirrors the in-card minimize chevron onto the anchored edge", () => {
    const content = document.createElement("div");
    const options = {
      snapshots: { claude: snapshot(20) },
      previousSnapshots: {},
      now: 1_000_000,
      collapsed: { claude: false, openai: false },
      onAction: vi.fn(),
      corner: "bottom-right",
    };

    reconcileProviderLayers(content, ["claude"], options);
    expect(content.querySelector<HTMLElement>(".minimize-control")?.dataset.edge).toBe("right");

    reconcileProviderLayers(content, ["claude"], { ...options, corner: "bottom-left" });
    expect(content.querySelectorAll(".minimize-control")).toHaveLength(1);
    expect(content.querySelector<HTMLElement>(".minimize-control")?.dataset.edge).toBe("left");
  });

  it("keeps provider identity through close, reopen, polling, and minimized restore", () => {
    const content = document.createElement("div");
    const claude = snapshot(21);
    const openai = snapshot(74);
    const options = { snapshots: { claude, openai }, previousSnapshots: {}, now: 1_000_000, onAction: vi.fn() };

    reconcileProviderLayers(content, ["claude", "openai"], options);
    const openaiNode = content.querySelector<HTMLElement>('[data-provider="openai"]')!;
    const openaiLogo = openaiNode.querySelector<HTMLImageElement>(".provider-mark img")!;

    reconcileProviderLayers(content, ["openai"], options);
    reconcileProviderLayers(content, ["claude", "openai"], {
      ...options,
      snapshots: { openai },
    });
    reconcileProviderLayers(content, ["claude", "openai"], {
      ...options,
      snapshots: { claude: snapshot(31), openai },
    });

    content.replaceChildren();
    reconcileProviderLayers(content, ["claude", "openai"], {
      ...options,
      snapshots: { claude: snapshot(31), openai },
    });

    const claudeNode = content.querySelector<HTMLElement>('[data-provider="claude"]')!;
    const restoredOpenaiNode = content.querySelector<HTMLElement>('[data-provider="openai"]')!;
    expect(claudeNode.dataset.provider).toBe("claude");
    expect(claudeNode.querySelector<HTMLImageElement>(".provider-mark img")?.src).toContain("claude-logo.png");
    expect(claudeNode.textContent).toContain("31%");
    expect(restoredOpenaiNode.dataset.provider).toBe("openai");
    expect(restoredOpenaiNode.querySelector<HTMLImageElement>(".provider-mark img")).not.toBe(openaiLogo);
    expect(restoredOpenaiNode.querySelector<HTMLImageElement>(".provider-mark img")?.src).toContain("chatgpt-logo.png");
    expect(restoredOpenaiNode.textContent).toContain("74%");
  });
});
