import { describe, expect, it, vi } from "vitest";
import {
  reconcileMinimalReadout,
  updateMinimalCountdowns,
  type MinimalReadoutOptions,
} from "./minimal-readout";
import type { MeterShape, SnapshotMap, UsageSnapshot } from "../types";

const NOW = 1_754_575_800;
const WEEKLY_RESET = 1_754_665_800;

function snapshot(fiveHour: number, weekly: number, fetchedAt = 1): UsageSnapshot {
  return {
    windows: [
      { label: "5 hour", used_percent: fiveHour, resets_at: NOW + 3_600 },
      { label: "Weekly", used_percent: weekly, resets_at: WEEKLY_RESET },
    ],
    fetched_at: fetchedAt,
    state: "fresh",
  };
}

function options(
  snapshots: SnapshotMap = { claude: snapshot(21, 48), openai: snapshot(34, 67) },
  meterShape: MeterShape = "ring",
): MinimalReadoutOptions {
  return {
    snapshots,
    now: NOW,
    meterShape,
    corner: "bottom-right",
    onAction: vi.fn(),
    onGeometryChange: vi.fn(async () => undefined),
  };
}

describe("reconcileMinimalReadout", () => {
  it("stacks active providers in one labelled region without cards or bubbles", () => {
    const host = document.createElement("div");

    reconcileMinimalReadout(host, ["claude", "openai"], options());

    const root = host.querySelector<HTMLElement>('.minimal-readout[role="region"]')!;
    expect(root.getAttribute("aria-label")).toBe("Claude and ChatGPT usage");
    expect(Array.from(root.querySelectorAll<HTMLElement>(".minimal-readout__provider")).map((node) => node.dataset.provider))
      .toEqual(["claude", "openai"]);
    expect(root.querySelectorAll('[data-window="five-hour"]')).toHaveLength(2);
    expect(root.querySelectorAll('[data-window="weekly"]')).toHaveLength(2);
    expect(root.querySelector(".layer")).toBeNull();
    expect(root.querySelector(".provider-bubble")).toBeNull();
    expect(root.querySelector(".minimize-control")).toBeNull();
    expect(root.querySelector<HTMLElement>(".minimal-readout__surface")!.tabIndex).toBe(0);
    expect(root.querySelector(".minimal-readout__surface")?.getAttribute("aria-label")).toBe("Show weekly usage");
  });

  it("uses the clipped surface itself as the screen-edge boundary", () => {
    const host = document.createElement("div");
    reconcileMinimalReadout(host, ["claude"], options({ claude: snapshot(21, 48) }));

    const shell = host.querySelector<HTMLElement>(".minimal-readout__surface-shell")!;
    const surface = host.querySelector<HTMLElement>(".minimal-readout__surface")!;
    expect(shell).not.toBeNull();
    expect(surface.parentElement).toBe(shell);
    expect(host.querySelector(".minimal-readout__edge-connector")).toBeNull();
  });

  it("renders one action shell with sibling trigger, blade, and controls", () => {
    const host = document.createElement("div");
    reconcileMinimalReadout(host, ["claude"], options({ claude: snapshot(21, 48) }));

    const shell = host.querySelector<HTMLElement>(".minimal-readout__action-shell")!;
    const trigger = shell?.querySelector<HTMLButtonElement>(".minimal-readout__dock-handle")!;
    const blade = shell?.querySelector<HTMLElement>(".minimal-readout__action-blade")!;
    const dock = shell?.querySelector<HTMLElement>(".minimal-readout__dock")!;
    expect(shell).not.toBeNull();
    expect(Array.from(shell.children)).toEqual(expect.arrayContaining([trigger, blade, dock]));
    expect(trigger.contains(dock)).toBe(false);
    expect(trigger.getAttribute("aria-expanded")).toBe("false");
    expect(dock.getAttribute("aria-hidden")).toBe("true");
  });

  it.each(["ring", "columns", "semicircle"] as const)(
    "centers the provider logo inside the %s meter",
    (meterShape) => {
      const host = document.createElement("div");
      reconcileMinimalReadout(host, ["claude"], options({ claude: snapshot(21, 48) }, meterShape));

      const meter = host.querySelector<HTMLElement>('.minimal-meter[data-window="five-hour"]')!;
      const logo = meter.querySelector<HTMLImageElement>(".minimal-meter__logo")!;
      expect(meter.dataset.shape).toBe(meterShape);
      expect(meter.getAttribute("role")).toBe("progressbar");
      expect(meter.getAttribute("aria-label")).toBe("Claude 5 hour usage");
      expect(logo.parentElement).toBe(meter);
      expect(logo.alt).toBe("");
      expect(logo.getAttribute("aria-hidden")).toBe("true");
    },
  );

  it("keeps compact five-hour content visible and weekly details initially concealed", () => {
    const host = document.createElement("div");
    reconcileMinimalReadout(host, ["claude"], options({ claude: snapshot(21, 48) }));

    expect(host.querySelector<HTMLElement>('[data-window="five-hour"] .minimal-meter__value')?.textContent).toBe("21%");
    const weekly = host.querySelector<HTMLElement>(".minimal-readout__weekly")!;
    expect(weekly.getAttribute("aria-hidden")).toBe("true");
    expect(weekly.querySelector(".minimal-meter__value")?.textContent).toBe("48%");
    expect(weekly.querySelector(".minimal-readout__reset")?.textContent).toContain("Aug 08");
  });

  it("updates usage in place without replacing provider, meter, or logo nodes", () => {
    const host = document.createElement("div");
    const initial = options({ claude: snapshot(21, 48, 1) });
    reconcileMinimalReadout(host, ["claude"], initial);
    const provider = host.querySelector<HTMLElement>(".minimal-readout__provider")!;
    const meter = provider.querySelector<HTMLElement>('[data-window="five-hour"]')!;
    const logo = meter.querySelector<HTMLImageElement>("img")!;

    reconcileMinimalReadout(host, ["claude"], {
      ...initial,
      snapshots: { claude: snapshot(39, 72, 2) },
    });

    expect(host.querySelector(".minimal-readout__provider")).toBe(provider);
    expect(provider.querySelector('[data-window="five-hour"]')).toBe(meter);
    expect(meter.querySelector("img")).toBe(logo);
    expect(meter.querySelector(".minimal-meter__value")?.textContent).toBe("39%");
    expect(provider.querySelector('[data-window="weekly"] .minimal-meter__value')?.textContent).toBe("72%");
  });

  it("shows truthful unavailable states without inventing progress", () => {
    const host = document.createElement("div");
    const missing: UsageSnapshot = {
      windows: [{ label: "Weekly", used_percent: 48, resets_at: WEEKLY_RESET }],
      fetched_at: 1,
      state: "fresh",
    };

    reconcileMinimalReadout(host, ["claude"], options({ claude: missing }));

    const unavailable = host.querySelector<HTMLElement>('[data-window="five-hour"]')!;
    expect(unavailable.textContent).toContain("Unavailable");
    expect(unavailable.getAttribute("role")).not.toBe("progressbar");
    expect(unavailable.textContent).not.toContain("0%");
  });

  it("wires the quiet handle, Settings toggle, and Tuck to distinct actions", () => {
    const host = document.createElement("div");
    const onAction = vi.fn();
    reconcileMinimalReadout(host, ["claude"], { ...options({ claude: snapshot(21, 48) }), onAction });

    const handle = host.querySelector<HTMLButtonElement>(".minimal-readout__dock-handle")!;
    expect(handle.getAttribute("aria-label")).toBe("Show overlay actions");
    host.querySelector<HTMLButtonElement>('[data-action="settings"]')!.click();
    expect(onAction).toHaveBeenLastCalledWith({ action: "toggle-settings" });
    host.querySelector<HTMLButtonElement>('[data-action="tuck"]')!.click();
    expect(onAction).toHaveBeenLastCalledWith({ action: "tuck" });
  });

  it("updates the weekly countdown without rebuilding the surface", () => {
    const host = document.createElement("div");
    reconcileMinimalReadout(host, ["claude"], options({ claude: snapshot(21, 48) }));
    const root = host.querySelector<HTMLElement>(".minimal-readout")!;
    const reset = root.querySelector<HTMLElement>(".minimal-readout__reset")!;

    updateMinimalCountdowns(root, NOW + 100);

    expect(host.querySelector(".minimal-readout")).toBe(root);
    expect(root.querySelector(".minimal-readout__reset")).toBe(reset);
    expect(reset.textContent).toMatch(/^Aug 08 · 1d /);
  });
});
