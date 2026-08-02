import { describe, expect, it } from "vitest";
import { calculateOverlayGeometry } from "./geometry";

describe("calculateOverlayGeometry", () => {
  it("keeps card-only regions inset from the host", () => {
    const geometry = calculateOverlayGeometry(
      { left: 0, top: 0 },
      [
        { left: 8, top: 8, width: 310, height: 70, right: 318, bottom: 78 },
        { left: 8, top: 87, width: 310, height: 166, right: 318, bottom: 253 },
      ],
      [],
      8,
      14,
    );

    expect(geometry.regions).toEqual([
      { x: 8, y: 8, width: 310, height: 70, radius: 14 },
      { x: 8, y: 87, width: 310, height: 166, radius: 14 },
    ]);
    expect(geometry.contentWidth).toBe(326);
    expect(geometry.contentHeight).toBe(261);
  });

  it("shrinks one bubble to an inset-free 48px host", () => {
    const geometry = calculateOverlayGeometry(
      { left: 0, top: 0 },
      [],
      [{ left: 270, top: 8, width: 48, height: 48, right: 318, bottom: 56 }],
      8,
      14,
      24,
      { left: 270, top: 8, width: 48, height: 48, right: 318, bottom: 56 },
    );

    expect(geometry.regions).toEqual([
      { x: 0, y: 0, width: 48, height: 48, radius: 24 },
    ]);
    expect(geometry.contentWidth).toBe(48);
    expect(geometry.contentHeight).toBe(48);
  });

  it("shrinks two bubbles to an inset-free 104px horizontal row", () => {
    const geometry = calculateOverlayGeometry(
      { left: 0, top: 0 },
      [],
      [
        { left: 214, top: 8, width: 48, height: 48, right: 262, bottom: 56 },
        { left: 270, top: 8, width: 48, height: 48, right: 318, bottom: 56 },
      ],
      8,
      14,
      24,
      { left: 214, top: 8, width: 104, height: 48, right: 318, bottom: 56 },
    );

    expect(geometry.regions).toEqual([
      { x: 0, y: 0, width: 48, height: 48, radius: 24 },
      { x: 56, y: 0, width: 48, height: 48, radius: 24 },
    ]);
    expect(geometry.contentWidth).toBe(104);
    expect(geometry.contentHeight).toBe(48);
  });

  it("places a mixed card below the 48px bubble row and 9px gap", () => {
    const geometry = calculateOverlayGeometry(
      { left: 0, top: 0 },
      [{ left: 8, top: 57, width: 310, height: 166, right: 318, bottom: 223 }],
      [{ left: 270, top: 0, width: 48, height: 48, right: 318, bottom: 48 }],
      8,
      14,
      24,
      { left: 270, top: 0, width: 48, height: 48, right: 318, bottom: 48 },
    );

    expect(geometry.regions).toEqual([
      { x: 8, y: 57, width: 310, height: 166, radius: 14 },
      { x: 270, y: 0, width: 48, height: 48, radius: 24 },
    ]);
    expect(geometry.contentWidth).toBe(326);
    expect(geometry.contentHeight).toBe(231);
  });

  it("drops malformed or nonpositive DOM rectangles from the measured union", () => {
    const valid = { left: 8, top: 8, width: 310, height: 70, right: 318, bottom: 78 };
    const invalid = [
      { ...valid, left: Number.NaN },
      { ...valid, top: Number.POSITIVE_INFINITY },
      { ...valid, right: Number.NaN },
      { ...valid, bottom: Number.NEGATIVE_INFINITY },
      { ...valid, width: Number.NaN },
      { ...valid, height: Number.POSITIVE_INFINITY },
      { ...valid, width: 0 },
      { ...valid, height: -1 },
      { ...valid, right: valid.left - 1 },
      { ...valid, bottom: valid.top - 1 },
    ];

    const geometry = calculateOverlayGeometry(
      { left: 0, top: 0 },
      [valid, ...invalid],
      [],
      8,
      14,
    );

    expect(geometry.regions).toEqual([
      { x: 8, y: 8, width: 310, height: 70, radius: 14 },
    ]);
    expect(geometry.contentWidth).toBe(326);
    expect(geometry.contentHeight).toBe(86);
  });

  it("returns empty fallback geometry when no measured region rectangle is valid", () => {
    const geometry = calculateOverlayGeometry(
      { left: 0, top: 0 },
      [{ left: 8, top: 8, width: 0, height: 70, right: 8, bottom: 78 }],
      [{ left: 0, top: 0, width: 48, height: Number.NaN, right: 48, bottom: 48 }],
      8,
      14,
      24,
      { left: 0, top: 0, width: 48, height: 48, right: 48, bottom: 48 },
    );

    expect(geometry).toEqual({
      regions: [],
      contentWidth: null,
      contentHeight: null,
    });
  });
});
