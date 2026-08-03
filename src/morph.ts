/** Mid-point of the requested 1–1.5s range: long enough to read as a deliberate morph,
 *  short enough not to make minimize/restore feel sluggish. */
export const MORPH_DURATION_MS = 1200;
export const MORPH_EASING = "cubic-bezier(.16,1,.3,1)";

export interface MorphRect {
  left: number;
  top: number;
  width: number;
  height: number;
  borderRadius: number;
}

interface RectLike {
  left: number;
  top: number;
  width: number;
  height: number;
}

export function toMorphRect(rect: RectLike, borderRadius: number): MorphRect {
  return { left: rect.left, top: rect.top, width: rect.width, height: rect.height, borderRadius };
}

function rectStyle(rect: MorphRect): Keyframe {
  return {
    left: `${rect.left}px`,
    top: `${rect.top}px`,
    width: `${rect.width}px`,
    height: `${rect.height}px`,
    borderRadius: `${rect.borderRadius}px`,
  };
}

/** Two-keyframe shape tween the browser interpolates natively via the Web Animations API. */
export function morphKeyframes(from: MorphRect, to: MorphRect): Keyframe[] {
  return [rectStyle(from), rectStyle(to)];
}

/** Cross-fades the ghost out and the real destination element in during the final quarter
 *  of the morph, so the shape settles before the content swap becomes visible. */
export function crossfadeKeyframes(direction: "out" | "in"): Keyframe[] {
  return direction === "out"
    ? [{ opacity: 1, offset: 0 }, { opacity: 1, offset: 0.75 }, { opacity: 0, offset: 1 }]
    : [{ opacity: 0, offset: 0 }, { opacity: 0, offset: 0.75 }, { opacity: 1, offset: 1 }];
}

export function prefersReducedMotion(): boolean {
  return typeof window !== "undefined" && (window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false);
}

export function supportsElementAnimate(): boolean {
  return typeof Element !== "undefined" && typeof Element.prototype.animate === "function";
}
