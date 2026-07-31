import { axe } from "vitest-axe";
import { afterEach, describe, expect, it } from "vitest";
import { renderLayer } from "./components/layer";
import { renderSettings } from "./components/settings";
import type { Config, UsageSnapshot } from "./types";

const snap: UsageSnapshot = {
  windows: [
    { label: "5 hour", used_percent: 12, resets_at: 1_003_600 },
    { label: "Weekly", used_percent: 48, resets_at: 1_259_200 },
  ],
  fetched_at: 999_940,
  state: "fresh",
};

describe("accessibility", () => {
  afterEach(() => {
    document.body.innerHTML = "";
  });

  for (const state of ["fresh", "stale", "error"] as const) {
    it(`has no violations in the ${state} state`, async () => {
      const host = document.createElement("main");
      host.appendChild(renderLayer("Claude", { ...snap, state }, 1_000_000));
      document.body.appendChild(host);
      expect((await axe(host)).violations).toEqual([]);
    });
  }
  it("has no violations when a provider reports no windows", async () => {
    const host = document.createElement("main");
    host.appendChild(renderLayer("Codex", { ...snap, windows: [] }, 1_000_000));
    document.body.appendChild(host);
    expect((await axe(host)).violations).toEqual([]);
  });
  it("has no violations in the custom settings controls", async () => {
    const config: Config = {
      monitorId: "display-1",
      corner: "bottom-right",
      scale: 1,
      cardOpacity: 0.94,
      theme: "acrylic",
      backgroundColor: "#07101f",
      layout: "stacked-compact",
      alwaysOnTop: true,
      offscreenPeek: false,
      pollIntervalSec: 60,
      detectIntervalSec: 5,
    };
    const host = renderSettings(config, [{ id: "display-1", label: "Monitor 1 — 1920×1080" }], { onChange: () => undefined, onClose: () => undefined });
    document.body.appendChild(host);
    expect((await axe(host)).violations).toEqual([]);
  });
});
