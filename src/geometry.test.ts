import { describe, expect, it } from "vitest";
import {
  calculateOverlayGeometry,
  createGeometryRequestSequencer,
  shouldCommitGeometryRequest,
} from "./geometry";

function deferred(): { promise: Promise<void>; resolve: () => void } {
  let resolve!: () => void;
  const promise = new Promise<void>((finish) => {
    resolve = finish;
  });
  return { promise, resolve };
}

describe("geometry request sequencing", () => {
  it("rejects a stale completion from updating the applied geometry", () => {
    expect(shouldCommitGeometryRequest(1, 2, true)).toBe(false);
    expect(shouldCommitGeometryRequest(2, 2, false)).toBe(false);
    expect(shouldCommitGeometryRequest(2, 2, true)).toBe(true);
  });

  it("reapplies a duplicate key after a newer request is already in flight", async () => {
    const applied: string[] = [];
    const committed: string[] = [];
    const aReady = deferred();
    const bReady = deferred();
    const sequencer = createGeometryRequestSequencer((key) => committed.push(key));

    const requestA = sequencer.request("A", async () => {
      applied.push("A");
      await aReady.promise;
      return true;
    });
    await Promise.resolve();

    const requestB = sequencer.request("B", async () => {
      applied.push("B");
      await bReady.promise;
      return true;
    });
    aReady.resolve();
    await requestA;
    await Promise.resolve();
    expect(applied).toEqual(["A", "B"]);

    const duplicateA = sequencer.request("A", async () => {
      applied.push("A");
      return true;
    });
    bReady.resolve();
    await Promise.all([requestB, duplicateA]);

    expect(applied).toEqual(["A", "B", "A"]);
    expect(committed).toEqual(["A"]);
    expect(sequencer.lastAppliedKey()).toBe("A");
  });
});

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
