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
  it("renders one row per window", () => expect(renderLayer("Claude", snap, 1_000_000).querySelectorAll('[role="progressbar"]').length).toBe(2));
  it("gives each bar an accessible value and description", () => {
    const bar = renderLayer("Claude", snap, 1_000_000).querySelector('[role="progressbar"]')!;
    expect(bar.getAttribute("aria-valuenow")).toBe("12");
    expect(bar.getAttribute("aria-valuemin")).toBe("0");
    expect(bar.getAttribute("aria-valuemax")).toBe("100");
    expect(bar.getAttribute("aria-valuetext")).toBe("12 percent used, resets in 1h");
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
});
