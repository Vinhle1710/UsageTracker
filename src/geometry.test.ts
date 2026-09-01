import { describe, expect, it } from "vitest";
import {
  calculateOverlayGeometry,
  expandMeasuredRectHorizontally,
  overlayEdgePadding,
} from "./geometry";

describe("minimal overlay geometry", () => {
  it("removes decorative edge padding so the connector reaches the native screen margin", () => {
    expect(overlayEdgePadding("minimal", 1)).toBe(0);
    expect(overlayEdgePadding("stacked-compact", 1)).toBe(8);
    expect(overlayEdgePadding("provider-columns", 1.25)).toBe(10);
  });

  it("makes the reserved horizontal span paintable without adding the dock reserve vertically", () => {
    const surface = { left: 100, top: 20, width: 52, height: 180, right: 152, bottom: 200 };
    const reserve = { left: -14, top: 20, width: 166, height: 276, right: 152, bottom: 296 };

    expect(expandMeasuredRectHorizontally(surface, reserve)).toEqual({
      left: -14,
      top: 20,
      width: 166,
      height: 180,
      right: 152,
      bottom: 200,
    });
  });
});

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

  it("gives an extra its own clip region without disturbing the card insets", () => {
    // The tuck tab rides on the card's outer edge but is neither a card nor a bubble: feeding
    // it through either list would flip the padding heuristics that place every other region.
    const cards = [{ left: 8, top: 8, width: 310, height: 70, right: 318, bottom: 78 }];
    const tab = { left: 305, top: 25, width: 13, height: 34, right: 318, bottom: 59 };
    const withoutTab = calculateOverlayGeometry({ left: 0, top: 0 }, cards, [], 8, 14);
    const withTab = calculateOverlayGeometry({ left: 0, top: 0 }, cards, [], 8, 14, 24, null, 0, [tab]);

    expect(withTab.regions[0]).toEqual(withoutTab.regions[0]);
    expect(withTab.contentWidth).toBe(withoutTab.contentWidth);
    expect(withTab.contentHeight).toBe(withoutTab.contentHeight);
    expect(withTab.regions).toHaveLength(2);
    expect(withTab.regions[1]).toEqual({ x: 305, y: 25, width: 13, height: 34, radius: 8 });
  });

  it("never widens the host for an extra, so the cards keep their place at the corner", () => {
    // The host is anchored to the screen corner. An extra that grew it would push the cards
    // inward by exactly what it gained, leaving the tab no closer to the edge than it started.
    const cards = [{ left: 8, top: 8, width: 310, height: 70, right: 318, bottom: 78 }];
    const tab = { left: 318, top: 25, width: 13, height: 34, right: 331, bottom: 59 };
    const withoutTab = calculateOverlayGeometry({ left: 0, top: 0 }, cards, [], 8, 14);
    const geometry = calculateOverlayGeometry({ left: 0, top: 0 }, cards, [], 8, 14, 24, null, 0, [tab]);

    expect(geometry.contentWidth).toBe(withoutTab.contentWidth);
    expect(geometry.contentHeight).toBe(withoutTab.contentHeight);
    // Still gets a region of its own, sitting in the padding the host already had.
    expect(geometry.regions).toContainEqual({ x: 318, y: 25, width: 13, height: 34, radius: 8 });
  });

  it("uses reserved bounds to size an animation without making the reserve clickable", () => {
    const surface = { left: 100, top: 20, width: 52, height: 180, right: 152, bottom: 200 };
    const action = { left: 60, top: 204, width: 36, height: 36, right: 96, bottom: 240 };
    const reserve = { left: 0, top: 20, width: 152, height: 220, right: 152, bottom: 240 };

    const geometry = calculateOverlayGeometry(
      { left: 0, top: 0 },
      [surface],
      [],
      0,
      14,
      24,
      null,
      0,
      [action],
      [reserve],
    );

    expect(geometry.contentWidth).toBe(152);
    expect(geometry.contentHeight).toBe(220);
    expect(geometry.regions).toEqual([
      { x: 100, y: 0, width: 52, height: 180, radius: 14 },
      { x: 60, y: 184, width: 36, height: 36, radius: 8 },
    ]);
  });

  it("ignores a degenerate extra rather than collapsing the whole measurement", () => {
    const cards = [{ left: 8, top: 8, width: 310, height: 70, right: 318, bottom: 78 }];
    const bad = { left: 8, top: 8, width: 0, height: NaN, right: 8, bottom: NaN };
    const geometry = calculateOverlayGeometry({ left: 0, top: 0 }, cards, [], 8, 14, 24, null, 0, [bad]);
    expect(geometry.regions).toHaveLength(1);
    expect(geometry.contentWidth).toBe(326);
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

  it("places a mixed card below the 48px bubble row and 9px gap for a bottom-anchored overlay", () => {
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

  it("places a mixed card above the 48px bubble row for a top-anchored overlay", () => {
    // Mirror image of the bottom-anchored case above, and inferred purely from where the bubble
    // measured relative to the card rather than a hardcoded "bubble is always on top" — so the
    // native region follows whichever side app.css actually rendered the row on. Getting this
    // wrong clipped the bubble into a cropped, see-through arc.
    const geometry = calculateOverlayGeometry(
      { left: 0, top: 0 },
      [{ left: 8, top: 8, width: 310, height: 166, right: 318, bottom: 174 }],
      [{ left: 270, top: 183, width: 48, height: 48, right: 318, bottom: 231 }],
      8,
      14,
      24,
      { left: 270, top: 183, width: 48, height: 48, right: 318, bottom: 231 },
    );

    expect(geometry.regions).toEqual([
      { x: 8, y: 8, width: 310, height: 166, radius: 14 },
      { x: 270, y: 183, width: 48, height: 48, radius: 24 },
    ]);
    expect(geometry.contentWidth).toBe(326);
    expect(geometry.contentHeight).toBe(231);
  });

  it("adds headroom to every side, growing the window and insetting the regions equally", () => {
    // The slack has to appear on all four sides, including the anchored one — an overshoot
    // toward the screen corner is exactly what was getting clipped by the work area edge.
    const card = { left: 8, top: 8, width: 310, height: 70, right: 318, bottom: 78 };
    const plain = calculateOverlayGeometry({ left: 0, top: 0 }, [card], [], 8, 14);
    const padded = calculateOverlayGeometry({ left: 0, top: 0 }, [card], [], 8, 14, 24, null, 64);

    expect(padded.regions[0]).toEqual({ ...plain.regions[0], x: 8 + 64, y: 8 + 64 });
    expect(padded.contentWidth).toBe(plain.contentWidth! + 128);
    expect(padded.contentHeight).toBe(plain.contentHeight! + 128);
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
