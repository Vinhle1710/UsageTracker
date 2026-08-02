import { describe, expect, it } from "vitest";
import { calculateOverlayGeometry } from "./geometry";

describe("calculateOverlayGeometry", () => {
  it("normalizes card and bubble regions to the visible union", () => {
    const geometry = calculateOverlayGeometry(
      { left: 0, top: 0 },
      [
        { left: 8, top: 8, width: 310, height: 70, right: 318, bottom: 78 },
        { left: 8, top: 87, width: 310, height: 166, right: 318, bottom: 253 },
      ],
      [
        { left: 262, top: 8, width: 48, height: 48, right: 310, bottom: 56 },
      ],
      8,
      14,
    );

    expect(geometry.regions).toEqual([
      { x: 8, y: 8, width: 310, height: 70, radius: 14 },
      { x: 8, y: 87, width: 310, height: 166, radius: 14 },
      { x: 262, y: 8, width: 48, height: 48, radius: 24 },
    ]);
    expect(geometry.contentWidth).toBe(326);
    expect(geometry.contentHeight).toBe(261);
  });

  it("shrinks a bubble-only host to one horizontal row without clipping", () => {
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
      { x: 8, y: 8, width: 48, height: 48, radius: 24 },
      { x: 64, y: 8, width: 48, height: 48, radius: 24 },
    ]);
    expect(geometry.contentWidth).toBe(120);
    expect(geometry.contentHeight).toBe(64);
  });

  it("includes the full bubble row union when a single bubble overlaps a card corner", () => {
    const geometry = calculateOverlayGeometry(
      { left: 0, top: 0 },
      [{ left: 8, top: 8, width: 310, height: 166, right: 318, bottom: 174 }],
      [{ left: 262, top: 8, width: 48, height: 48, right: 310, bottom: 56 }],
      8,
      14,
      24,
      { left: 262, top: 8, width: 48, height: 48, right: 310, bottom: 56 },
    );

    expect(geometry.contentWidth).toBe(326);
    expect(geometry.contentHeight).toBe(182);
  });
});
