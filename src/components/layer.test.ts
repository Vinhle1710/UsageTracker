import { describe, expect, it, vi } from "vitest";
import { progressOffset, renderLayer, renderLoadingLayer, updateLayer, updateMeter } from "./layer";
import type { UsageSnapshot } from "../types";

const snap: UsageSnapshot = {
  windows: [
    { label: "5 hour", used_percent: 12, resets_at: 1_003_600 },
    { label: "Weekly", used_percent: 48, resets_at: 1_259_200 },
  ],
  fetched_at: 999_940,
  state: "fresh",
};

function withExtra(spend: number | null, budget: number | null, balance: number | null = null): UsageSnapshot {
  const money = (minorUnits: number) => ({ minorUnits, currency: "USD" });
  return {
    ...snap,
    details: {
      claude: {
        limits: { value: null, fetchedAt: 1, state: "unavailable", errorCode: null },
        extra: {
          value: {
            spend: spend === null ? undefined : money(spend),
            budget: budget === null ? undefined : money(budget),
            balance: balance === null ? undefined : money(balance),
          },
          fetchedAt: 1,
          state: "fresh",
          errorCode: null,
        },
      },
    },
  };
}

describe("extra credit", () => {
  it("is absent entirely when the provider reports no extra usage", () => {
    const el = renderLayer("Claude", snap, 1_000_000);
    expect(el.querySelector(".extra-credit")).toBeNull();
  });

  it("is absent when the section resolved but carries no amounts", () => {
    const el = renderLayer("Claude", withExtra(null, null), 1_000_000);
    expect(el.querySelector(".extra-credit")).toBeNull();
  });

  it("renders a horizontal bar below the window grid, not inside it", () => {
    const el = renderLayer("Claude", withExtra(1234, 5000), 1_000_000);
    const extra = el.querySelector<HTMLElement>(".extra-credit")!;

    expect(extra.previousElementSibling?.classList.contains("window-grid")).toBe(true);
    expect(extra.querySelector(".extra-credit__fill")).not.toBeNull();
    // $12.34 of $50.00 is 24.68%.
    expect(extra.style.getPropertyValue("--progress-percent")).toBe("24.68%");
    expect(extra.textContent).toContain("$12.34");
    expect(extra.textContent).toContain("$50.00");
  });

  it("carries progressbar semantics so the amount is announced, not just drawn", () => {
    const extra = renderLayer("Claude", withExtra(1234, 5000), 1_000_000).querySelector(".extra-credit")!;
    expect(extra.getAttribute("role")).toBe("progressbar");
    expect(extra.getAttribute("aria-valuemin")).toBe("0");
    expect(extra.getAttribute("aria-valuemax")).toBe("100");
    expect(extra.getAttribute("aria-valuenow")).toBe("25");
    expect(extra.getAttribute("aria-valuetext")).toContain("$12.34 of $50.00");
  });

  it("shows the remaining grant balance when there is no spend limit to fill against", () => {
    const el = renderLayer("Claude", withExtra(null, null, 750), 1_000_000);
    const extra = el.querySelector<HTMLElement>(".extra-credit")!;

    // Nothing to be a percentage *of*, so it reports the balance rather than drawing a
    // fill against an invented denominator.
    expect(extra.querySelector(".extra-credit__fill")).toBeNull();
    expect(extra.textContent).toContain("$7.50");
    expect(extra.getAttribute("role")).not.toBe("progressbar");
  });

  it("never exceeds a full bar when spend has overrun the limit", () => {
    const extra = renderLayer("Claude", withExtra(9000, 5000), 1_000_000).querySelector<HTMLElement>(".extra-credit")!;
    expect(extra.style.getPropertyValue("--progress-percent")).toBe("100%");
    expect(extra.getAttribute("aria-valuenow")).toBe("100");
  });

  it("survives a zero limit without dividing by zero", () => {
    const extra = renderLayer("Claude", withExtra(0, 0), 1_000_000).querySelector<HTMLElement>(".extra-credit")!;
    expect(extra.style.getPropertyValue("--progress-percent")).toBe("0%");
  });

  it("is patched in place by updateLayer rather than forcing a rebuild", () => {
    const el = renderLayer("Claude", withExtra(1234, 5000), 1_000_000);
    const extra = el.querySelector<HTMLElement>(".extra-credit")!;

    expect(updateLayer(el, withExtra(2500, 5000), 1_000_000)).toBe(true);

    expect(el.querySelector(".extra-credit")).toBe(extra);
    expect(extra.style.getPropertyValue("--progress-percent")).toBe("50%");
    expect(extra.textContent).toContain("$25.00");
  });

  it("removes the bar when a later refresh reports extra usage switched off", () => {
    const el = renderLayer("Claude", withExtra(1234, 5000), 1_000_000);
    expect(updateLayer(el, snap, 1_000_000)).toBe(true);
    expect(el.querySelector(".extra-credit")).toBeNull();
  });
});

describe("meter shape", () => {
  it("defaults to the ring, keeping the SVG readout", () => {
    const meter = renderLayer("Claude", snap, 1_000_000).querySelector<HTMLElement>(".meter")!;
    expect(meter.dataset.shape).toBe("ring");
    expect(meter.querySelector(".meter__ring")).not.toBeNull();
    expect(meter.querySelector(".meter__charge")).toBeNull();
  });

  it("renders Charge as a compact horizontal gauge", () => {
    const meter = renderLayer("Claude", snap, 1_000_000, undefined, undefined, "charge").querySelector<HTMLElement>(".meter")!;
    expect(meter.dataset.shape).toBe("charge");
    expect(meter.querySelector(".meter__ring")).toBeNull();
    expect(meter.querySelector(".meter__charge-fill")).not.toBeNull();
    expect(meter.style.getPropertyValue("--progress-percent")).toBe("12%");
  });

  it("renders Reactor as a segmented arc that updates in place", () => {
    const meter = renderLayer("Claude", snap, 1_000_000, undefined, undefined, "reactor").querySelector<HTMLElement>(".meter")!;
    expect(meter.querySelector(".meter__reactor-fill")).toBeNull();
    expect(meter.querySelectorAll(".meter__reactor-segment")).toHaveLength(16);
    expect(meter.querySelectorAll(".meter__reactor-segment.is-active")).toHaveLength(2);

    updateMeter(meter, "Claude", { ...snap.windows[0], used_percent: 77 }, 1_000_000);
    expect(meter.querySelectorAll(".meter__reactor-segment.is-active")).toHaveLength(13);
  });

  it("renders Columns as a solid vertical column", () => {
    const meter = renderLayer("Claude", snap, 1_000_000, undefined, undefined, "columns").querySelector<HTMLElement>(".meter")!;
    expect(meter.querySelector(".meter__columns-fill")).not.toBeNull();
  });

  it("renders Line as vertically stacked telemetry rows with the same progressbar semantics", () => {
    const layer = renderLayer("Claude", snap, 1_000_000, undefined, undefined, "line");
    const meter = layer.querySelector<HTMLElement>(".meter")!;
    expect(meter.dataset.shape).toBe("line");
    expect(meter.querySelector(".meter__line-fill")).not.toBeNull();
    expect(layer.querySelector<HTMLElement>(".window-grid")?.dataset.shape).toBe("line");
    expect(meter.getAttribute("role")).toBe("progressbar");
    expect(meter.getAttribute("aria-valuenow")).toBe("12");
    expect(meter.getAttribute("aria-valuetext")).toContain("12 percent used");
  });

  it("renders Semi Circle as a separate open-bottom loading gauge", () => {
    const meter = renderLayer("Claude", snap, 1_000_000, undefined, undefined, "semicircle").querySelector<HTMLElement>(".meter")!;
    expect(meter.dataset.shape).toBe("semicircle");
    expect(meter.querySelector(".meter__semicircle-track")).not.toBeNull();
    expect(meter.querySelector(".meter__semicircle-progress")).not.toBeNull();
    expect(meter.querySelector(".meter__reactor-segment")).toBeNull();
  });

  it("keeps every shape updatable in place, so a shape change is the only thing that rebuilds", () => {
    const el = renderLayer("Claude", snap, 1_000_000, undefined, undefined, "charge");
    const meter = el.querySelector<HTMLElement>('[data-label="5 hour"]')!;

    expect(updateLayer(el, { ...snap, windows: [{ ...snap.windows[0], used_percent: 77 }, snap.windows[1]] }, 1_000_000, undefined, "charge")).toBe(true);

    // A different shape cannot be patched in place; it has to rebuild.
    expect(updateLayer(el, snap, 1_000_000, undefined, "ring")).toBe(false);

    expect(meter.style.getPropertyValue("--progress-percent")).toBe("77%");
    expect(meter.getAttribute("aria-valuenow")).toBe("77");
  });
});

describe("renderLayer", () => {
  it("renders one circular meter per usage window", () => {
    const el = renderLayer("Claude", snap, 1_000_000);
    expect(el.querySelectorAll('[role="progressbar"]')).toHaveLength(2);
    expect(el.querySelectorAll(".window-grid")).toHaveLength(1);
  });
  it("keeps five-hour and weekly meters side by side", () => {
    const grid = renderLayer("Claude", snap, 1_000_000).querySelector(".window-grid")!;
    expect(grid.children).toHaveLength(2);
    expect(grid.textContent).toContain("5 hour");
    expect(grid.textContent).toContain("Weekly");
  });
  it("uses provider identity instead of card position for styling", () => {
    expect(renderLayer("Claude", snap, 1_000_000).dataset.provider).toBe("claude");
    expect(renderLayer("ChatGPT", snap, 1_000_000).dataset.provider).toBe("openai");
  });
  it("uses the supplied provider logo assets", () => {
    const claude = renderLayer("Claude", snap, 1_000_000);
    const chatgpt = renderLayer("ChatGPT", snap, 1_000_000);
    expect(claude.querySelector<HTMLImageElement>(".provider-mark img")?.src).toContain("claude-logo.png");
    expect(chatgpt.querySelector<HTMLImageElement>(".provider-mark img")?.src).toContain("chatgpt-logo.png");
  });
  it("updates usage in place so the existing progress ring can animate", () => {
    const el = renderLayer("Claude", snap, 1_000_000);
    const meter = el.querySelector<HTMLElement>('[data-label="5 hour"]')!;
    const progress = meter.querySelector(".meter__progress");

    const updated = updateLayer(el, { ...snap, windows: [{ ...snap.windows[0], used_percent: 35 }, snap.windows[1]] }, 1_000_010);

    expect(updated).toBe(true);
    expect(el.querySelector('[data-label="5 hour"]')).toBe(meter);
    expect(el.querySelector('[data-label="5 hour"] .meter__progress')).toBe(progress);
    expect(meter.style.getPropertyValue("--progress-offset")).toBe(String(276.46 * 0.65));
    expect(meter.querySelector(".meter__value")?.textContent).toBe("35%");
    expect(meter.getAttribute("aria-valuenow")).toBe("35");
  });
  it("marks usage increases and decreases for animated feedback", () => {
    const increase = renderLayer(
      "Claude",
      { ...snap, windows: [{ ...snap.windows[0], used_percent: 24 }] },
      1_000_000,
      snap,
    );
    const decrease = renderLayer(
      "ChatGPT",
      { ...snap, windows: [{ ...snap.windows[0], used_percent: 4 }] },
      1_000_000,
      snap,
    );
    expect(increase.querySelector(".meter")?.getAttribute("data-usage-change")).toBe("increase");
    expect(decrease.querySelector(".meter")?.getAttribute("data-usage-change")).toBe("decrease");
  });
  it("centers a lone weekly meter when five-hour usage is unavailable", () => {
    const grid = renderLayer(
      "Claude",
      { ...snap, windows: [{ label: "Weekly", used_percent: 48, resets_at: 1_259_200 }] },
      1_000_000,
    ).querySelector<HTMLElement>(".window-grid")!;
    expect(grid.dataset.singleWindow).toBe("true");
    expect(grid.querySelector<HTMLElement>(".window-card")?.querySelector<HTMLElement>(".window-card__reset")?.dataset.resetsAt).toBe("1259200");
  });
  it("gives each circular meter accessible usage semantics", () => {
    const meter = renderLayer("Claude", snap, 1_000_000).querySelector('[role="progressbar"]')!;
    expect(meter.getAttribute("aria-valuenow")).toBe("12");
    expect(meter.getAttribute("aria-valuemin")).toBe("0");
    expect(meter.getAttribute("aria-valuemax")).toBe("100");
    expect(meter.getAttribute("aria-valuetext")).toContain("12 percent used");
  });
  it("renders a zero-percent window rather than hiding it", () => {
    const el = renderLayer("Claude", { ...snap, windows: [{ label: "5 hour", used_percent: 0, resets_at: 1_003_600 }] }, 1_000_000);
    expect(el.querySelectorAll('[role="progressbar"]').length).toBe(1);
    expect(el.textContent).toContain("0%");
  });
  it("does not confuse unavailable usage with provider detection", () => {
    const stale = renderLayer("Claude", { ...snap, windows: [], state: "stale" }, 1_000_000);
    expect(stale.textContent).toContain("Usage temporarily unavailable");
    expect(stale.textContent).not.toContain("No active window");
  });
  it("reads as still checking, not unavailable, before any usage has arrived", () => {
    const pending = renderLayer("ChatGPT", { ...snap, windows: [], state: "pending" }, 1_000_000);
    expect(pending.textContent).toContain("Checking usage");
    expect(pending.textContent).not.toContain("Usage temporarily unavailable");
    expect(pending.querySelector('[role="progressbar"]')).toBeNull();
  });
  it("never invents a usage figure for a state that carries no windows", () => {
    for (const state of ["pending", "stale", "error"] as const) {
      const el = renderLayer("ChatGPT", { ...snap, windows: [], state }, 1_000_000);
      expect(el.querySelector(".meter__value")).toBeNull();
      expect(el.textContent).not.toContain("100%");
    }
  });
  it("keeps previously fetched numbers visible when a refresh goes stale", () => {
    const el = renderLayer("ChatGPT", { ...snap, state: "stale" }, 1_000_000);
    expect(el.querySelectorAll('[role="progressbar"]').length).toBe(snap.windows.length);
    expect(el.dataset.state).toBe("stale");
  });
  it("marks the layer stale without blanking values", () => {
    const el = renderLayer("Claude", { ...snap, state: "stale" }, 1_000_000);
    expect(el.dataset.state).toBe("stale");
    expect(el.textContent).toContain("48%");
  });
  it("directs Claude re-authentication to Claude Code", () => expect(renderLayer("Claude", { ...snap, state: "error" }, 1_000_000).textContent).toContain("Claude Code"));
  it("tells a never-signed-in user to sign in rather than to sign in again", () => {
    const el = renderLayer("Claude", { ...snap, windows: [], state: "signed-out" }, 1_000_000);
    expect(el.textContent).toContain("Not signed in");
    expect(el.textContent).toContain("Run claude to sign in");
    expect(el.textContent).not.toContain("Usage temporarily unavailable");
  });
  it("sends Codex's hint to its CLI, not to Settings", () => {
    expect(renderLayer("ChatGPT", { ...snap, windows: [], state: "signed-out" }, 1_000_000).textContent).toContain("codex");
  });
  it("swaps the hint when an existing card transitions between signed-out and re-auth", () => {
    const el = renderLayer("Claude", { ...snap, state: "signed-out" }, 1_000_000);
    expect(updateLayer(el, { ...snap, state: "error" }, 1_000_000)).toBe(true);
    expect(el.querySelectorAll(".layer__hint").length).toBe(1);
    expect(el.textContent).toContain("Re-authenticate in Claude Code");
  });
  it("sends Claude's hint to the Claude Code CLI", () => {
    const onAction = vi.fn();
    const el = renderLayer("Claude", { ...snap, state: "error" }, 1_000_000, undefined, onAction);
    el.querySelector<HTMLButtonElement>(".layer__hint")!.click();
    expect(onAction).toHaveBeenCalledWith({ action: "open-cli", provider: "claude" });
  });
  it("names the openai provider for the ChatGPT hint button", () => {
    const onAction = vi.fn();
    const el = renderLayer("ChatGPT", { ...snap, windows: [], state: "signed-out" }, 1_000_000, undefined, onAction);
    el.querySelector<HTMLButtonElement>(".layer__hint")!.click();
    expect(onAction).toHaveBeenCalledWith({ action: "open-cli", provider: "openai" });
  });
  it("wires the click handler on a hint created during an update, not just on initial render", () => {
    const onAction = vi.fn();
    const el = renderLayer("Claude", { ...snap, state: "signed-out" }, 1_000_000);
    updateLayer(el, { ...snap, state: "error" }, 1_000_000, onAction);
    el.querySelector<HTMLButtonElement>(".layer__hint")!.click();
    expect(onAction).toHaveBeenCalledWith({ action: "open-cli", provider: "claude" });
  });
  it("renders the hint as a real button so it is keyboard and screen-reader actionable", () => {
    const el = renderLayer("Claude", { ...snap, state: "error" }, 1_000_000);
    expect(el.querySelector(".layer__hint")!.tagName).toBe("BUTTON");
  });
  it("does not render the removed updated footer", () => expect(renderLayer("Claude", snap, 1_000_000).textContent).not.toContain("Updated"));
  it("shows a provider-specific loading card without invented usage", () => {
    const el = renderLoadingLayer("ChatGPT");
    expect(el.dataset.provider).toBe("openai");
    expect(el.textContent).toContain("Loading usage");
    expect(el.querySelector('[role="progressbar"]')).toBeNull();
  });
});

describe("progressOffset", () => {
  it("returns the full ring length at 0%", () => expect(progressOffset(0)).toBe("276.46"));
  it("returns zero offset at 100%", () => expect(progressOffset(100)).toBe("0"));
  it("clamps values below 0", () => expect(progressOffset(-10)).toBe("276.46"));
  it("clamps values above 100", () => expect(progressOffset(150)).toBe("0"));
});

describe("updateMeter cache clearing", () => {
  function buildResetCard(cachedMessage: string): { meter: HTMLElement; reset: HTMLElement } {
    const card = document.createElement("div");
    card.className = "window-card";

    const meter = document.createElement("div");
    meter.className = "meter";
    meter.dataset.label = "5 hour";
    meter.dataset.resetsAt = "2000000";
    const value = document.createElement("span");
    value.className = "meter__value";
    meter.appendChild(value);

    const reset = document.createElement("span");
    reset.className = "window-card__reset";
    reset.dataset.label = "5 hour";
    reset.dataset.resetsAt = "2000000"; // stale value from before the reset
    reset.dataset.cachedMessage = cachedMessage;
    reset.textContent = cachedMessage;

    card.append(meter, reset);
    return { meter, reset };
  }

  it("clears a cached fun message once a new resets_at value arrives", () => {
    const { meter, reset } = buildResetCard("Recharging the quota…");

    updateMeter(meter, "Claude", { label: "5 hour", used_percent: 4, resets_at: 2_100_000 }, 1_000_000);

    expect(reset.dataset.cachedMessage).toBeUndefined();
    expect(reset.dataset.resetsAt).toBe("2100000");
  });

  it("keeps the cached fun message when resets_at has not changed yet", () => {
    const { meter, reset } = buildResetCard("Recharging the quota…");

    // Backend still reports the same (now-elapsed) resets_at — no fresh data yet.
    updateMeter(meter, "Claude", { label: "5 hour", used_percent: 0, resets_at: 2_000_000 }, 2_500_000);

    expect(reset.dataset.cachedMessage).toBe("Recharging the quota…");
  });
});
