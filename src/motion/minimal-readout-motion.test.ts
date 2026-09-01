import { describe, expect, it, vi } from "vitest";
import {
  dockBladeClip,
  enhanceMinimalReadout,
  type MinimalMotionAdapters,
  type ReversibleTimeline,
} from "./minimal-readout-motion";

interface NodeProcess {
  getBuiltinModule(name: "fs"): { readFileSync(path: string | URL, encoding: "utf8"): string };
}

const nodeProcess = (globalThis as typeof globalThis & { process: NodeProcess }).process;
const fs = nodeProcess.getBuiltinModule("fs");
const moduleFileUrl = new URL(import.meta.url);
const sourceUrl = (url: URL, relativePath: string): URL => url.protocol === "file:" ? url : new URL(relativePath, moduleFileUrl);
const motionSource = fs.readFileSync(sourceUrl(new URL("./minimal-readout-motion.ts", import.meta.url), "./minimal-readout-motion.ts"), "utf8");

function fixture(): HTMLElement {
  const root = document.createElement("section");
  root.className = "minimal-readout";
  root.innerHTML = `
    <div class="minimal-readout__reserved-bounds">
      <div class="minimal-readout__surface" tabindex="0">
        <div class="minimal-readout__weekly" aria-hidden="true"></div>
      </div>
      <div class="minimal-readout__action-shell">
        <button class="minimal-readout__dock-handle" type="button" aria-expanded="false">Actions</button>
        <span class="minimal-readout__action-blade" aria-hidden="true"></span>
        <div class="minimal-readout__dock" aria-hidden="true">
          <button class="minimal-readout__dock-action" data-action="settings" type="button" tabindex="-1" aria-hidden="true">Settings</button>
          <button class="minimal-readout__dock-action" data-action="tuck" type="button" tabindex="-1" aria-hidden="true">Tuck</button>
        </div>
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
  it("reveals the translucent blade from its anchored edge without scaling its material", () => {
    expect(dockBladeClip("left")).toEqual({ from: "inset(0 48% 0 0)", to: "inset(0 0 0 0)" });
    expect(dockBladeClip("right")).toEqual({ from: "inset(0 0 0 48%)", to: "inset(0 0 0 0)" });
    expect(motionSource).not.toMatch(/\.to\(trigger,\s*\{[^}]*scale:/);
  });

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

  it("does not retain a stale usage hover when the pointer leaves during opening", async () => {
    const root = fixture();
    const events: string[] = [];
    const { value, usage } = adapters(events);
    let finishPlay!: () => void;
    usage.play = vi.fn(() => new Promise<void>((resolve) => { finishPlay = resolve; }));
    enhanceMinimalReadout(root, { adapters: value, onGeometryChange: vi.fn(async () => undefined) });
    const surface = root.querySelector<HTMLElement>(".minimal-readout__surface")!;

    surface.dispatchEvent(new Event("pointerenter"));
    await settle();
    surface.dispatchEvent(new Event("pointerleave"));
    await settle();

    expect(usage.reverse).toHaveBeenCalledOnce();
    expect(root.dataset.reserveUsage).toBe("false");

    finishPlay();
    await settle();
  });

  it("interrupts an in-flight close on pointer re-entry without releasing native geometry", async () => {
    const root = fixture();
    const events: string[] = [];
    const { value, usage } = adapters(events);
    let finishReverse!: () => void;
    usage.reverse = vi.fn(() => new Promise<void>((resolve) => { finishReverse = resolve; }));
    const onGeometryChange = vi.fn(async () => {
      events.push(`geometry:usage:${root.dataset.reserveUsage}`);
    });
    enhanceMinimalReadout(root, { adapters: value, onGeometryChange });
    const surface = root.querySelector<HTMLElement>(".minimal-readout__surface")!;

    surface.dispatchEvent(new Event("pointerenter"));
    await settle();
    surface.dispatchEvent(new Event("pointerleave"));
    await settle();
    surface.dispatchEvent(new Event("pointerenter"));
    await settle();

    expect(usage.play).toHaveBeenCalledTimes(2);
    expect(root.dataset.reserveUsage).toBe("true");
    expect(events).not.toContain("geometry:usage:false");

    finishReverse();
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
      expect(button.dataset.geometryVisible).toBeUndefined();
    }
    expect(root.querySelector(".minimal-readout__dock-handle")?.getAttribute("aria-expanded")).toBe("true");
    expect(root.querySelector(".minimal-readout__dock")?.getAttribute("aria-hidden")).toBe("false");
    expect(root.dataset.usageExpanded).not.toBe("true");
  });

  it("does not retain a stale blade hover when the pointer leaves during opening", async () => {
    const root = fixture();
    const events: string[] = [];
    const { value, dock } = adapters(events);
    let finishPlay!: () => void;
    dock.play = vi.fn(() => new Promise<void>((resolve) => { finishPlay = resolve; }));
    enhanceMinimalReadout(root, { adapters: value, onGeometryChange: vi.fn(async () => undefined) });
    const shell = root.querySelector<HTMLElement>(".minimal-readout__action-shell")!;

    shell.dispatchEvent(new Event("pointerenter"));
    await settle();
    shell.dispatchEvent(new MouseEvent("pointerleave"));
    await settle();

    expect(dock.reverse).toHaveBeenCalledOnce();
    expect(root.dataset.reserveDock).toBe("false");

    finishPlay();
    await settle();
  });

  it("keeps the blade open while the pointer crosses from its trigger to either action", async () => {
    const root = fixture();
    const events: string[] = [];
    const { value, dock } = adapters(events);
    const onGeometryChange = vi.fn(async () => {
      events.push(`geometry:dock:${root.dataset.reserveDock}`);
    });
    enhanceMinimalReadout(root, { adapters: value, onGeometryChange });
    const shell = root.querySelector<HTMLElement>(".minimal-readout__action-shell")!;
    const action = root.querySelector<HTMLButtonElement>('[data-action="settings"]')!;

    shell.dispatchEvent(new Event("pointerenter"));
    await settle();
    expect(events).toEqual(["geometry:dock:true", "dock:play"]);
    events.length = 0;

    shell.dispatchEvent(new MouseEvent("pointerleave", { relatedTarget: action }));
    await settle();

    expect(dock.reverse).not.toHaveBeenCalled();
    expect(root.dataset.dockExpanded).toBe("true");
  });

  it("interrupts an in-flight blade close when the pointer returns to the action shell", async () => {
    const root = fixture();
    const events: string[] = [];
    const { value, dock } = adapters(events);
    let finishReverse!: () => void;
    dock.reverse = vi.fn(() => new Promise<void>((resolve) => { finishReverse = resolve; }));
    const onGeometryChange = vi.fn(async () => {
      events.push(`geometry:dock:${root.dataset.reserveDock}`);
    });
    enhanceMinimalReadout(root, { adapters: value, onGeometryChange });
    const shell = root.querySelector<HTMLElement>(".minimal-readout__action-shell")!;

    shell.dispatchEvent(new Event("pointerenter"));
    await settle();
    shell.dispatchEvent(new MouseEvent("pointerleave"));
    await settle();
    shell.dispatchEvent(new Event("pointerenter"));
    await settle();

    expect(dock.play).toHaveBeenCalledTimes(2);
    expect(root.dataset.reserveDock).toBe("true");
    expect(events).not.toContain("geometry:dock:false");

    finishReverse();
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
