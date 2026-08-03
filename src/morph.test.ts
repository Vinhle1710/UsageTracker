import { afterEach, describe, expect, it, vi } from "vitest";
import { crossfadeKeyframes, MORPH_DURATION_MS, morphKeyframes, prefersReducedMotion, supportsElementAnimate, toMorphRect } from "./morph";

describe("toMorphRect", () => {
  it("carries the measured rect and the requested border radius", () => {
    expect(toMorphRect({ left: 10, top: 20, width: 300, height: 90 }, 14)).toEqual({
      left: 10,
      top: 20,
      width: 300,
      height: 90,
      borderRadius: 14,
    });
  });
});

describe("morphKeyframes", () => {
  it("produces exactly a start and end keyframe with px/units", () => {
    const from = toMorphRect({ left: 0, top: 0, width: 48, height: 48 }, 24);
    const to = toMorphRect({ left: 8, top: 8, width: 280, height: 96 }, 14);

    const frames = morphKeyframes(from, to);

    expect(frames).toHaveLength(2);
    expect(frames[0]).toMatchObject({ left: "0px", top: "0px", width: "48px", height: "48px", borderRadius: "24px" });
    expect(frames[1]).toMatchObject({ left: "8px", top: "8px", width: "280px", height: "96px", borderRadius: "14px" });
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

describe("MORPH_DURATION_MS", () => {
  it("sits within the requested 1 to 1.5 second range", () => {
    expect(MORPH_DURATION_MS).toBeGreaterThanOrEqual(1000);
    expect(MORPH_DURATION_MS).toBeLessThanOrEqual(1500);
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
