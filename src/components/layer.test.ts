import { describe, expect, it } from "vitest";
import { renderLayer } from "./layer";
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
  it("centers a lone weekly meter when five-hour usage is unavailable", () => {
    const grid = renderLayer(
      "Claude",
      { ...snap, windows: [{ label: "Weekly", used_percent: 48, resets_at: 1_259_200 }] },
      1_000_000,
    ).querySelector<HTMLElement>(".window-grid")!;
    expect(grid.dataset.singleWindow).toBe("true");
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
});
