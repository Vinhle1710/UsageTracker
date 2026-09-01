import { describe, expect, it, vi } from "vitest";
import {
  enhanceMinimalReadout,
  type MinimalMotionAdapters,
  type ReversibleTimeline,
} from "./minimal-readout-motion";

function fixture(): HTMLElement {
  const root = document.createElement("section");
  root.className = "minimal-readout";
  root.innerHTML = `
    <div class="minimal-readout__reserved-bounds">
      <div class="minimal-readout__surface" tabindex="0">
        <div class="minimal-readout__weekly" aria-hidden="true"></div>
      </div>
      <button class="minimal-readout__dock-handle" type="button">Actions</button>
      <div class="minimal-readout__dock">
        <button class="minimal-readout__dock-action" type="button" tabindex="-1" aria-hidden="true">Settings</button>
        <button class="minimal-readout__dock-action" type="button" tabindex="-1" aria-hidden="true">Tuck</button>
      </div>
    </div>`;
  return root;
}

function timeline(name: string, events: string[]): ReversibleTimeline {
  let currentProgress = 0;
  return {
    play: vi.fn(async () => { events.push(`${name}:play`); currentProgress = 1; }),
    reverse: vi.fn(async () => { events.push(`${name}:reverse`); currentProgress = 0; }),
    progress: vi.fn(() => currentProgress),
    finish: vi.fn((open) => { events.push(`${name}:finish:${open}`); currentProgress = open ? 1 : 0; }),
    kill: vi.fn(() => events.push(`${name}:kill`)),
  };
}

function adapters(events: string[]): { value: MinimalMotionAdapters; usage: ReversibleTimeline; dock: ReversibleTimeline } {
  const usage = timeline("usage", events);
  const dock = timeline("dock", events);
  return {
    usage,
    dock,
    value: {
      createUsageTimeline: vi.fn(() => usage),
      createDockTimeline: vi.fn(() => dock),
      reducedMotion: vi.fn(() => false),
    },
  };
}

async function settle(): Promise<void> {
  await Promise.resolve();
  await Promise.resolve();
  await Promise.resolve();
}

describe("enhanceMinimalReadout", () => {
  it("reserves native usage geometry before playing the reveal", async () => {
    const root = fixture();
    const events: string[] = [];
    const { value } = adapters(events);
    const onGeometryChange = vi.fn(async () => {
      events.push(`geometry:usage:${root.dataset.reserveUsage}`);
    });
    enhanceMinimalReadout(root, { adapters: value, onGeometryChange });

    root.querySelector<HTMLElement>(".minimal-readout__surface")!
      .dispatchEvent(new Event("pointerenter"));
    await settle();

    expect(events).toEqual(["geometry:usage:true", "usage:play"]);
    expect(root.dataset.usageExpanded).toBe("true");
    expect(root.querySelector(".minimal-readout__weekly")?.getAttribute("aria-hidden")).toBe("false");
  });

  it("reverses the reveal before releasing compact geometry", async () => {
    const root = fixture();
    const events: string[] = [];
    const { value } = adapters(events);
    const onGeometryChange = vi.fn(async () => {
      events.push(`geometry:usage:${root.dataset.reserveUsage}`);
    });
    enhanceMinimalReadout(root, { adapters: value, onGeometryChange });
    const surface = root.querySelector<HTMLElement>(".minimal-readout__surface")!;

    surface.dispatchEvent(new Event("pointerenter"));
    await settle();
    events.length = 0;
    surface.dispatchEvent(new Event("pointerleave"));
    await settle();

    expect(events).toEqual(["usage:reverse", "geometry:usage:false"]);
    expect(root.dataset.usageExpanded).toBe("false");
    expect(root.querySelector(".minimal-readout__weekly")?.getAttribute("aria-hidden")).toBe("true");
  });

  it("opens the action dock from handle focus and exposes only its two buttons", async () => {
    const root = fixture();
    const events: string[] = [];
    const { value } = adapters(events);
    enhanceMinimalReadout(root, { adapters: value, onGeometryChange: vi.fn(async () => undefined) });

    root.querySelector<HTMLButtonElement>(".minimal-readout__dock-handle")!
      .dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
    await settle();

    expect(events).toContain("dock:play");
    for (const button of root.querySelectorAll<HTMLButtonElement>(".minimal-readout__dock-action")) {
      expect(button.tabIndex).toBe(0);
      expect(button.getAttribute("aria-hidden")).toBe("false");
      expect(button.dataset.geometryVisible).toBe("true");
    }
    expect(root.dataset.usageExpanded).not.toBe("true");
  });

  it("Escape closes both states without activating an action", async () => {
    const root = fixture();
    const events: string[] = [];
    const { value } = adapters(events);
    enhanceMinimalReadout(root, { adapters: value, onGeometryChange: vi.fn(async () => undefined) });
    root.querySelector<HTMLElement>(".minimal-readout__surface")!.dispatchEvent(new Event("pointerenter"));
    root.querySelector<HTMLButtonElement>(".minimal-readout__dock-handle")!
      .dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
    await settle();

    root.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    await settle();

    expect(events).toContain("usage:reverse");
    expect(events).toContain("dock:reverse");
    expect(root.dataset.usageExpanded).toBe("false");
    expect(root.dataset.dockExpanded).toBe("false");
  });

  it("creates one reusable timeline per state and kills both during cleanup", async () => {
    const root = fixture();
    const events: string[] = [];
    const { value, usage, dock } = adapters(events);
    const cleanup = enhanceMinimalReadout(root, { adapters: value, onGeometryChange: vi.fn(async () => undefined) });
    const surface = root.querySelector<HTMLElement>(".minimal-readout__surface")!;

    surface.dispatchEvent(new Event("pointerenter"));
    surface.dispatchEvent(new Event("pointerenter"));
    await settle();
    expect(value.createUsageTimeline).toHaveBeenCalledOnce();
    expect(value.createDockTimeline).toHaveBeenCalledOnce();

    cleanup();
    cleanup();
    expect(usage.kill).toHaveBeenCalledOnce();
    expect(dock.kill).toHaveBeenCalledOnce();
  });

  it("jumps timelines to settled states when reduced motion is requested", async () => {
    const root = fixture();
    const events: string[] = [];
    const { value, usage } = adapters(events);
    vi.mocked(value.reducedMotion).mockReturnValue(true);
    enhanceMinimalReadout(root, { adapters: value, onGeometryChange: vi.fn(async () => undefined) });

    root.querySelector<HTMLElement>(".minimal-readout__surface")!.dispatchEvent(new Event("pointerenter"));
    await settle();

    expect(usage.finish).toHaveBeenCalledWith(true);
    expect(usage.play).not.toHaveBeenCalled();
  });
});
