import { describe, expect, it } from "vitest";
import { calculateOverlayGeometry } from "./geometry";

describe("calculateOverlayGeometry", () => {
  it("uses the rendered card rectangles instead of splitting the window evenly", () => {
    const geometry = calculateOverlayGeometry(
      { left: 0, top: 0 },
      [
        { left: 8, top: 8, width: 310, height: 70, right: 318, bottom: 78 },
        { left: 8, top: 87, width: 310, height: 166, right: 318, bottom: 253 },
      ],
      8,
      14,
    );

    expect(geometry.regions).toEqual([
      { x: 8, y: 8, width: 310, height: 70, radius: 14 },
      { x: 8, y: 87, width: 310, height: 166, radius: 14 },
    ]);
    expect(geometry.contentHeight).toBe(261);
  });
});
