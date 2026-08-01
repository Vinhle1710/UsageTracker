import { describe, expect, it } from "vitest";
import { calculateOverlayGeometry } from "./geometry";

describe("calculateOverlayGeometry", () => {
  it("uses the rendered card rectangles instead of splitting the window evenly", () => {
    const geometry = calculateOverlayGeometry(
      { left: 0, top: 0 },
      [
        {
          provider: "claude", left: 8, top: 8, width: 310, height: 70, right: 318, bottom: 78,
        },
        {
          provider: "openai", left: 8, top: 87, width: 310, height: 166, right: 318, bottom: 253,
        },
      ],
      8,
      14,
    );

    expect(geometry.regions).toEqual([
      { provider: "claude", x: 8, y: 8, width: 310, height: 70, radius: 14 },
      { provider: "openai", x: 8, y: 87, width: 310, height: 166, radius: 14 },
    ]);
    expect(geometry.contentHeight).toBe(261);
  });

  it("keeps each measured provider attached to its output region", () => {
    const geometry = calculateOverlayGeometry(
      { left: 100, top: 50 },
      [
        {
          provider: "claude", left: 108, top: 58, width: 310, height: 70, right: 418, bottom: 128,
        },
        {
          provider: "openai", left: 108, top: 137, width: 310, height: 166, right: 418, bottom: 303,
        },
      ],
      8,
      14,
    );

    expect(geometry.regions).toEqual([
      { provider: "claude", x: 8, y: 8, width: 310, height: 70, radius: 14 },
      { provider: "openai", x: 8, y: 87, width: 310, height: 166, radius: 14 },
    ]);
  });
});
