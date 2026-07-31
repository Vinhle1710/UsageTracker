import { describe, expect, it } from "vitest";
import { renderLayer, renderLoadingLayer, updateLayer } from "./layer";
import type { UsageSnapshot } from "../types";

const snap: UsageSnapshot = {
  windows: [
    { label: "5 hour", used_percent: 12, resets_at: 1_003_600 },
    { label: "Weekly", used_percent: 48, resets_at: 1_259_200 },
  ],
  fetched_at: 999_940,
  state: "fresh",
};

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
  it("shows a no-window message when the provider reports none", () => expect(renderLayer("Claude", { ...snap, windows: [] }, 1_000_000).textContent).toContain("No active window"));
  it("marks the layer stale without blanking values", () => {
    const el = renderLayer("Claude", { ...snap, state: "stale" }, 1_000_000);
    expect(el.dataset.state).toBe("stale");
    expect(el.textContent).toContain("48%");
  });
  it("shows a re-auth hint in the error state", () => expect(renderLayer("Claude", { ...snap, state: "error" }, 1_000_000).textContent).toContain("Re-authenticate"));
  it("does not render the removed updated footer", () => expect(renderLayer("Claude", snap, 1_000_000).textContent).not.toContain("Updated"));
  it("shows a provider-specific loading card without invented usage", () => {
    const el = renderLoadingLayer("ChatGPT");
    expect(el.dataset.provider).toBe("openai");
    expect(el.textContent).toContain("Loading usage");
    expect(el.querySelector('[role="progressbar"]')).toBeNull();
  });
});
