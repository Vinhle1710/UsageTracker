import { describe, expect, it, vi } from "vitest";
import { generateConfetti, spawnCelebration } from "./celebration";

describe("generateConfetti", () => {
  it("spreads pieces evenly around the circle with deterministic jitter from the injected rand", () => {
    const rand = () => 0.5;
    const pieces = generateConfetti(4, rand);

    expect(pieces).toHaveLength(4);
    expect(pieces[0].angle).toBeCloseTo(0);
    expect(pieces[1].angle).toBeCloseTo(90);
    expect(pieces.every((piece) => piece.distance > 0)).toBe(true);
    expect(pieces.every((piece) => piece.delayMs >= 0)).toBe(true);
  });

  it("cycles through the color palette by index", () => {
    const pieces = generateConfetti(6, () => 0);
    expect(pieces[0].colorVar).toBe("--claude");
    expect(pieces[4].colorVar).toBe(pieces[0].colorVar);
  });
});

describe("spawnCelebration", () => {
  it("scopes the burst to the given card only, never a sibling or the document", () => {
    vi.useFakeTimers();
    const card = document.createElement("div");
    const sibling = document.createElement("div");
    document.body.append(card, sibling);

    spawnCelebration(card, generateConfetti(5, () => 0.5));

    expect(card.querySelector(".celebration-burst")).not.toBeNull();
    expect(card.querySelectorAll(".celebration-piece")).toHaveLength(5);
    expect(sibling.querySelector(".celebration-burst")).toBeNull();
    expect(document.body.querySelector(":scope > .celebration-burst")).toBeNull();

    vi.useRealTimers();
  });

  it("removes the burst from the DOM after it finishes playing", () => {
    vi.useFakeTimers();
    const card = document.createElement("div");
    spawnCelebration(card, generateConfetti(3, () => 0.5));

    expect(card.querySelector(".celebration-burst")).not.toBeNull();
    vi.advanceTimersByTime(1200);
    expect(card.querySelector(".celebration-burst")).toBeNull();

    vi.useRealTimers();
  });
});
