import { describe, expect, it, vi } from "vitest";
import { renderEdgeTab, renderTuckControl } from "./edge-tab";

describe("renderEdgeTab", () => {
  it("is a single labelled button, not a decorative sliver", () => {
    const onRestore = vi.fn();
    const tab = renderEdgeTab("bottom-right", onRestore);
    const button = tab.querySelector<HTMLButtonElement>("button")!;

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
    const button = renderEdgeTab("bottom-right", vi.fn()).querySelector<HTMLButtonElement>("button")!;
    expect(button.className).toContain("edge-tab__button");
  });
});

describe("renderTuckControl", () => {
  it("names the destination rather than the mechanism", () => {
    const onTuck = vi.fn();
    const control = renderTuckControl(onTuck);
    const button = control.querySelector<HTMLButtonElement>("button")!;

    expect(button.getAttribute("aria-label")).toBe("Tuck usage to the screen edge");
    expect(button.title).toBe("Tuck usage to the screen edge");
    button.click();
    expect(onTuck).toHaveBeenCalledOnce();
  });

  it("is keyboard-operable on the same keys as the other overlay controls", () => {
    const onTuck = vi.fn();
    const button = renderTuckControl(onTuck).querySelector<HTMLButtonElement>("button")!;

    button.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true, cancelable: true }));
    button.dispatchEvent(new KeyboardEvent("keydown", { key: " ", bubbles: true, cancelable: true }));

    expect(onTuck).toHaveBeenCalledTimes(2);
  });

  it("ignores keys that are not activation keys", () => {
    const onTuck = vi.fn();
    const button = renderTuckControl(onTuck).querySelector<HTMLButtonElement>("button")!;
    button.dispatchEvent(new KeyboardEvent("keydown", { key: "a", bubbles: true, cancelable: true }));
    expect(onTuck).not.toHaveBeenCalled();
  });
});
