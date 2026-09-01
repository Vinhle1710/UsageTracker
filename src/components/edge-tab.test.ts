import { describe, expect, it, vi } from "vitest";
import { renderEdgeTab, renderTuckControl, setSettingsButtonOpen, TUCK_MOTION_MS, tuckKeyframes } from "./edge-tab";

describe("renderEdgeTab", () => {
  it("renders a labelled pull button, not a decorative sliver", () => {
    const onRestore = vi.fn();
    const tab = renderEdgeTab("bottom-right", onRestore);
    const button = tab.querySelector<HTMLButtonElement>(".usage-tab__button")!;

    expect(button.getAttribute("aria-label")).toBe("Show usage overlay");
    expect(button.type).toBe("button");
    button.click();
    expect(onRestore).toHaveBeenCalledOnce();
  });

  it("points its chevron back toward the screen, away from the edge it clings to", () => {
    // A right-edge tab has to be grabbed leftward, and vice versa — a chevron pointing into
    // the bezel would tell the user the wrong direction.
    expect(renderEdgeTab("bottom-right", vi.fn()).dataset.edge).toBe("right");
    expect(renderEdgeTab("top-right", vi.fn()).dataset.edge).toBe("right");
    expect(renderEdgeTab("bottom-left", vi.fn()).dataset.edge).toBe("left");
    expect(renderEdgeTab("top-left", vi.fn()).dataset.edge).toBe("left");
  });

  it("keeps a full-height hit target even though the tab is drawn narrow", () => {
    const button = renderEdgeTab("bottom-right", vi.fn()).querySelector<HTMLButtonElement>(".usage-tab__button")!;
    expect(button.className).toContain("usage-tab__button");
  });

  it("opens settings without restoring the overlay", () => {
    const onRestore = vi.fn();
    const onOpenSettings = vi.fn();
    const tab = renderEdgeTab("bottom-right", onRestore, onOpenSettings);
    const button = tab.querySelector<HTMLButtonElement>(".usage-tab__settings-button")!;

    expect(button.type).toBe("button");
    expect(button.getAttribute("aria-label")).toBe("Open settings");
    expect(button.title).toBe("Open settings");
    expect(button.querySelector("svg")?.getAttribute("aria-hidden")).toBe("true");
    button.click();

    expect(onOpenSettings).toHaveBeenCalledOnce();
    expect(onRestore).not.toHaveBeenCalled();
  });

  it("renders and updates the authoritative settings visibility state", () => {
    const tab = renderEdgeTab("bottom-right", vi.fn(), vi.fn(), true);
    const button = tab.querySelector<HTMLButtonElement>(".usage-tab__settings-button")!;

    expect(button.getAttribute("aria-pressed")).toBe("true");
    expect(button.dataset.settingsOpen).toBe("true");
    expect(button.getAttribute("aria-label")).toBe("Close settings");
    expect(button.title).toBe("Close settings");

    setSettingsButtonOpen(tab, false);

    expect(button.getAttribute("aria-pressed")).toBe("false");
    expect(button.dataset.settingsOpen).toBe("false");
    expect(button.getAttribute("aria-label")).toBe("Open settings");
    expect(button.title).toBe("Open settings");
  });
});

describe("renderTuckControl", () => {
  it("names the destination rather than the mechanism", () => {
    const onTuck = vi.fn();
    const control = renderTuckControl(onTuck);
    const button = control.querySelector<HTMLButtonElement>(".usage-tab__button")!;

    expect(button.getAttribute("aria-label")).toBe("Tuck usage to the screen edge");
    expect(button.title).toBe("Tuck usage to the screen edge");
    button.click();
    expect(onTuck).toHaveBeenCalledOnce();
  });

  it("is keyboard-operable on the same keys as the other overlay controls", () => {
    const onTuck = vi.fn();
    const button = renderTuckControl(onTuck).querySelector<HTMLButtonElement>(".usage-tab__button")!;

    button.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
    button.dispatchEvent(new KeyboardEvent("keydown", { key: " ", bubbles: true, cancelable: true }));

    expect(onTuck).toHaveBeenCalledTimes(2);
  });

  it("ignores keys that are not activation keys", () => {
    const onTuck = vi.fn();
    const button = renderTuckControl(onTuck).querySelector<HTMLButtonElement>(".usage-tab__button")!;
    button.dispatchEvent(new KeyboardEvent("keydown", { key: "a", bubbles: true, cancelable: true }));
    expect(onTuck).not.toHaveBeenCalled();
  });

  it("opens settings from the active overlay without tucking it", () => {
    const onTuck = vi.fn();
    const onOpenSettings = vi.fn();
    const button = renderTuckControl(onTuck, "bottom-right", onOpenSettings)
      .querySelector<HTMLButtonElement>(".usage-tab__settings-button")!;

    button.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
    button.dispatchEvent(new KeyboardEvent("keydown", { key: " ", bubbles: true, cancelable: true }));

    expect(onOpenSettings).toHaveBeenCalledTimes(2);
    expect(onTuck).not.toHaveBeenCalled();
  });

  it("shares the same settings visibility state in the active overlay", () => {
    const control = renderTuckControl(vi.fn(), "bottom-right", vi.fn(), true);
    const button = control.querySelector<HTMLButtonElement>(".usage-tab__settings-button")!;

    expect(button.getAttribute("aria-pressed")).toBe("true");
    expect(button.dataset.settingsOpen).toBe("true");
  });

  it("points at the vertical edge the overlay is anchored to, not always rightward", () => {
    // The tab appears on the corner's own edge, so a left-anchored overlay tucking rightward
    // would send the user looking at the wrong side of the screen.
    expect(renderTuckControl(vi.fn(), "bottom-left").dataset.edge).toBe("left");
    expect(renderTuckControl(vi.fn(), "top-left").dataset.edge).toBe("left");
    expect(renderTuckControl(vi.fn(), "bottom-right").dataset.edge).toBe("right");
    expect(renderTuckControl(vi.fn(), "top-right").dataset.edge).toBe("right");
  });

  it("is the same button as the edge tab, with only the chevron turned around", () => {
    // Tucking must not look like the handle became a different control, so the two share a
    // renderer; the only thing that may differ between them is which way the arrow points.
    const open = renderTuckControl(vi.fn(), "bottom-right").querySelector(".usage-tab__button")!;
    const closed = renderEdgeTab("bottom-right", vi.fn()).querySelector(".usage-tab__button")!;

    expect(open.className).toBe(closed.className);
    expect(open.querySelector("svg")!.innerHTML).not.toBe(closed.querySelector("svg")!.innerHTML);
    // Open points into the edge it disappears into; closed points back at the screen.
    expect(open.querySelector(".usage-tab__button path")!.getAttribute("d")).toBe("m6 3.5 4.5 4.5-4.5 4.5");
    expect(closed.querySelector(".usage-tab__button path")!.getAttribute("d")).toBe("m10 3.5-4.5 4.5 4.5 4.5");
    // ...and mirrored wholesale on the other anchor.
    expect(renderTuckControl(vi.fn(), "bottom-left").querySelector(".usage-tab__button path")!.getAttribute("d"))
      .toBe("m10 3.5-4.5 4.5 4.5 4.5");
    expect(renderEdgeTab("bottom-left", vi.fn()).querySelector(".usage-tab__button path")!.getAttribute("d"))
      .toBe("m6 3.5 4.5 4.5-4.5 4.5");
  });
});

describe("tuckKeyframes", () => {
  const offsetOf = (frames: Keyframe[], index: number) =>
    Number(/translateX\((-?[\d.]+)px\)/.exec(String(frames[index].transform))?.[1]);

  it("travels toward the anchored edge on the way out and back off it on the way in", () => {
    // The overlay disappears into the edge it is anchored to, so the sign of the travel is the
    // whole point: mirrored, the surface would slide across the screen away from its own tab.
    expect(offsetOf(tuckKeyframes("right", "out"), 1)).toBeGreaterThan(0);
    expect(offsetOf(tuckKeyframes("left", "out"), 1)).toBeLessThan(0);
    expect(offsetOf(tuckKeyframes("right", "in"), 0)).toBeGreaterThan(0);
    expect(offsetOf(tuckKeyframes("left", "in"), 0)).toBeLessThan(0);
  });

  it("ends resting and opaque going in, and starts that way going out", () => {
    const out = tuckKeyframes("right", "out");
    const into = tuckKeyframes("right", "in");
    expect(out[0]).toMatchObject({ opacity: 1 });
    expect(out[1]).toMatchObject({ opacity: 0 });
    expect(into[0]).toMatchObject({ opacity: 0 });
    expect(into[1]).toMatchObject({ opacity: 1 });
    expect(offsetOf(out, 0)).toBe(0);
    expect(offsetOf(into, 1)).toBe(0);
  });

  it("is short enough to stay ahead of the native window swap", () => {
    expect(TUCK_MOTION_MS).toBeLessThanOrEqual(260);
  });
});
