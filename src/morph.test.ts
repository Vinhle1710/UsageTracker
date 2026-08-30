import { afterEach, describe, expect, it, vi } from "vitest";
import { crossfadeKeyframes, flipDelta, flipKeyframes, isNegligibleFlipDelta, MORPH_DURATION_MS, morphKeyframes, prefersReducedMotion, supportsElementAnimate, toAnchoredRect } from "./morph";

const viewport = { width: 400, height: 300 };

describe("toAnchoredRect", () => {
  it("expresses the rect from left/top for a top-left anchored corner", () => {
    const anchored = toAnchoredRect({ left: 10, top: 20, width: 300, height: 90 }, 14, "top-left", viewport);
    expect(anchored).toEqual({ xSide: "left", ySide: "top", x: 10, y: 20, width: 300, height: 90, borderRadius: 14 });
  });

  it("expresses the rect from right/bottom for a bottom-right anchored corner", () => {
    // rect right edge = 10+300=310, distance from viewport right (400) = 90
    // rect bottom edge = 20+90=110, distance from viewport bottom (300) = 190
    const anchored = toAnchoredRect({ left: 10, top: 20, width: 300, height: 90 }, 14, "bottom-right", viewport);
    expect(anchored).toEqual({ xSide: "right", ySide: "bottom", x: 90, y: 190, width: 300, height: 90, borderRadius: 14 });
  });

  it("mixes sides independently for top-right and bottom-left corners", () => {
    const topRight = toAnchoredRect({ left: 10, top: 20, width: 300, height: 90 }, 0, "top-right", viewport);
    expect(topRight.xSide).toBe("right");
    expect(topRight.ySide).toBe("top");

    const bottomLeft = toAnchoredRect({ left: 10, top: 20, width: 300, height: 90 }, 0, "bottom-left", viewport);
    expect(bottomLeft.xSide).toBe("left");
    expect(bottomLeft.ySide).toBe("bottom");
  });

  it("stays correct when the viewport itself changes size, unlike a left/top measurement would", () => {
    // Same physical position relative to the bottom-right corner, before and after a resize
    // that shrinks the viewport but keeps that corner pinned.
    const beforeResize = toAnchoredRect({ left: 10, top: 20, width: 300, height: 90 }, 0, "bottom-right", { width: 400, height: 300 });
    const afterResize = toAnchoredRect({ left: 0, top: 0, width: 300, height: 90 }, 0, "bottom-right", { width: 390, height: 280 });
    expect(afterResize.x).toBe(beforeResize.x);
    expect(afterResize.y).toBe(beforeResize.y);
  });
});

describe("morphKeyframes", () => {
  it("produces exactly a start and end keyframe using the anchored side's own property", () => {
    const from = toAnchoredRect({ left: 8, top: 8, width: 310, height: 171 }, 14, "bottom-right", viewport);
    const to = toAnchoredRect({ left: 262, top: 0, width: 48, height: 48 }, 24, "bottom-right", viewport);

    const frames = morphKeyframes(from, to);

    expect(frames).toHaveLength(2);
    expect(frames[0]).toMatchObject({ right: `${from.x}px`, bottom: `${from.y}px`, width: "310px", height: "171px", borderRadius: "14px" });
    expect(frames[1]).toMatchObject({ right: `${to.x}px`, bottom: `${to.y}px`, width: "48px", height: "48px", borderRadius: "24px" });
    expect(frames[0]).not.toHaveProperty("left");
    expect(frames[0]).not.toHaveProperty("top");
  });

  it("is a no-op pair of identical keyframes when from and to are the same rect", () => {
    const rect = toAnchoredRect({ left: 5, top: 5, width: 100, height: 100 }, 12, "top-left", viewport);
    const frames = morphKeyframes(rect, rect);
    expect(frames[0]).toEqual(frames[1]);
  });
});

describe("crossfadeKeyframes", () => {
  it("fades an outgoing ghost from opaque to transparent, holding until the last quarter", () => {
    const frames = crossfadeKeyframes("out");
    expect(frames[0]).toMatchObject({ opacity: 1, offset: 0 });
    expect(frames[frames.length - 1]).toMatchObject({ opacity: 0, offset: 1 });
  });

  it("fades an incoming element from transparent to opaque, holding until the last quarter", () => {
    const frames = crossfadeKeyframes("in");
    expect(frames[0]).toMatchObject({ opacity: 0, offset: 0 });
    expect(frames[frames.length - 1]).toMatchObject({ opacity: 1, offset: 1 });
  });
});

describe("flipDelta", () => {
  it("computes how far an element must appear offset to hide a layout jump", () => {
    expect(flipDelta({ left: 100, top: 50, width: 10, height: 10 }, { left: 100, top: 107, width: 10, height: 10 }))
      .toEqual({ dx: 0, dy: -57 });
  });
});

describe("isNegligibleFlipDelta", () => {
  it("treats sub-pixel deltas as no movement", () => {
    expect(isNegligibleFlipDelta({ dx: 0.1, dy: -0.2 })).toBe(true);
    expect(isNegligibleFlipDelta({ dx: 1, dy: 0 })).toBe(false);
    expect(isNegligibleFlipDelta({ dx: 0, dy: 57 })).toBe(false);
  });
});

describe("flipKeyframes", () => {
  it("starts offset by the inverted delta and settles at no transform", () => {
    const frames = flipKeyframes({ dx: 0, dy: -57 });
    expect(frames[0]).toMatchObject({ transform: "translate(0px, -57px)" });
    expect(frames[1]).toMatchObject({ transform: "translate(0px, 0px)" });
  });
});

describe("MORPH_DURATION_MS", () => {
  it("is 50% of the original 1200ms duration per explicit feedback", () => {
    expect(MORPH_DURATION_MS).toBe(600);
  });
});

describe("prefersReducedMotion", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("reflects the matchMedia result", () => {
    vi.stubGlobal("matchMedia", (query: string) => ({ matches: query.includes("reduce") }));
    expect(prefersReducedMotion()).toBe(true);
  });

  it("defaults to false when matchMedia reports no preference", () => {
    vi.stubGlobal("matchMedia", () => ({ matches: false }));
    expect(prefersReducedMotion()).toBe(false);
  });
});

describe("supportsElementAnimate", () => {
  it("detects the Web Animations API on Element.prototype", () => {
    expect(supportsElementAnimate()).toBe(typeof Element.prototype.animate === "function");
  });
});
