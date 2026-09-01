import { axe } from "vitest-axe";
import { afterEach, describe, expect, it } from "vitest";
import { renderLayer } from "./components/layer";
import { reconcileMinimalReadout } from "./components/minimal-readout";
import { reconcileProviderLayers } from "./components/overlay";
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
  it("has accessible independent provider bubble controls", async () => {
    const host = document.createElement("main");
    const content = document.createElement("div");
    reconcileProviderLayers(content, ["claude", "openai"], {
      snapshots: { claude: snap, openai: { ...snap, windows: [{ ...snap.windows[0], used_percent: 68 }] } },
      previousSnapshots: {},
      now: 1_000_000,
      collapsed: { claude: true, openai: true },
      onAction: () => undefined,
    });
    host.appendChild(content);
    document.body.appendChild(host);

    expect(Array.from(content.querySelectorAll<HTMLButtonElement>(".provider-bubble")).map((button) => button.getAttribute("aria-label")))
      .toEqual(["Expand Claude usage", "Expand ChatGPT usage"]);
    expect(Array.from(content.querySelectorAll<HTMLImageElement>(".provider-bubble img")).every((logo) => logo.alt === "" && logo.getAttribute("aria-hidden") === "true"))
      .toBe(true);
    expect((await axe(host)).violations).toEqual([]);
  });
  it("keeps both controls native and keyboard-focusable in a mixed card and bubble view", async () => {
    const host = document.createElement("main");
    const content = document.createElement("div");
    reconcileProviderLayers(content, ["claude", "openai"], {
      snapshots: { claude: snap, openai: snap },
      previousSnapshots: {},
      now: 1_000_000,
      collapsed: { claude: false, openai: true },
      onAction: () => undefined,
    });
    host.appendChild(content);
    document.body.appendChild(host);

    const minimize = content.querySelector<HTMLButtonElement>('.minimize-control__button[data-provider="claude"]')!;
    const bubble = content.querySelector<HTMLButtonElement>('.provider-bubble[data-provider="openai"]')!;
    expect(minimize.type).toBe("button");
    expect(bubble.type).toBe("button");
    expect(minimize.tabIndex).toBe(0);
    expect(bubble.tabIndex).toBe(0);
    minimize.focus();
    expect(document.activeElement).toBe(minimize);
    bubble.focus();
    expect(document.activeElement).toBe(bubble);
    expect((await axe(host)).violations).toEqual([]);
  });
  it("has an accessible combined Minimal region with concealed dock actions", async () => {
    const host = document.createElement("main");
    const content = document.createElement("div");
    reconcileMinimalReadout(content, ["claude", "openai"], {
      snapshots: { claude: snap, openai: { ...snap, windows: snap.windows.map((window) => ({ ...window, used_percent: window.used_percent + 10 })) } },
      now: 1_000_000,
      meterShape: "ring",
      corner: "bottom-right",
      onAction: () => undefined,
      onGeometryChange: async () => undefined,
    });
    host.appendChild(content);
    document.body.appendChild(host);

    const region = content.querySelector<HTMLElement>('.minimal-readout[role="region"]')!;
    expect(region.getAttribute("aria-label")).toBe("Claude and ChatGPT usage");
    expect(Array.from(region.querySelectorAll('[role="progressbar"]')).map((meter) => meter.getAttribute("aria-label")))
      .toEqual(["Claude 5 hour usage", "Claude Weekly usage", "ChatGPT 5 hour usage", "ChatGPT Weekly usage"]);
    expect(Array.from(region.querySelectorAll<HTMLImageElement>("img")).every((logo) => logo.alt === "" && logo.getAttribute("aria-hidden") === "true"))
      .toBe(true);
    expect(Array.from(region.querySelectorAll<HTMLButtonElement>(".minimal-readout__dock-action")).every((button) => button.tabIndex === -1 && button.getAttribute("aria-hidden") === "true"))
      .toBe(true);

    const trigger = region.querySelector<HTMLButtonElement>(".minimal-readout__dock-handle")!;
    const dock = region.querySelector<HTMLElement>(".minimal-readout__dock")!;
    expect(trigger.getAttribute("aria-expanded")).toBe("false");
    expect(dock.getAttribute("aria-hidden")).toBe("true");
    trigger.focus();
    await Promise.resolve();
    await Promise.resolve();
    expect(trigger.getAttribute("aria-expanded")).toBe("true");
    expect(dock.getAttribute("aria-hidden")).toBe("false");
    expect(Array.from(region.querySelectorAll<HTMLButtonElement>(".minimal-readout__dock-action")).every((button) => button.tabIndex === 0 && button.getAttribute("aria-hidden") === "false"))
      .toBe(true);
    expect((await axe(host)).violations).toEqual([]);
  });
  it("has no violations in the custom settings controls", async () => {
    const config: Config = {
      monitorId: "display-1",
      corner: "bottom-right",
      scale: 1,
      cardOpacity: 0.94,
      theme: "frosted",
      backgroundColor: "#07101f",
      layout: "stacked-compact",
      alwaysOnTop: true,
      offscreenPeek: false,
      launchAtStartup: true,
      pollIntervalSec: 60,
      detectIntervalSec: 5,
    };
    const host = renderSettings(config, [{ id: "display-1", label: "Monitor 1 — 1920×1080" }], { onChange: () => undefined, onClose: () => undefined });
    document.body.appendChild(host);
    expect((await axe(host)).violations).toEqual([]);
  });
});
