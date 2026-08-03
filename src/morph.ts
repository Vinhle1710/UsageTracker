/** 50% faster than the original 1200ms per explicit feedback. */
export const MORPH_DURATION_MS = 600;
/** Smooth, monotonic ease-out for the shape/logo morph: settles directly on the destination's
 *  exact geometry with no overshoot, so the ghost never reads as bigger than the source or
 *  smaller than the destination mid-flight. */
export const MORPH_EASING = "cubic-bezier(.22,.8,.35,1)";
/** Kept bouncy for sibling position-only slides (see flipKeyframes) — those never change size,
 *  so a little overshoot there doesn't create the "wrong shape" artifact the shape/logo morph
 *  had with this same curve. */
export const FLIP_EASING = "cubic-bezier(.34,1.56,.64,1)";

interface RectLike {
  left: number;
  top: number;
  width: number;
  height: number;
}

interface ViewportLike {
  width: number;
  height: number;
}

/** "top-left" | "top-right" | "bottom-left" | "bottom-right" — matches Config.corner. */
export type Corner = string;

export interface AnchoredRect {
  xSide: "left" | "right";
  ySide: "top" | "bottom";
  x: number;
  y: number;
  width: number;
  height: number;
  borderRadius: number;
}

/** Expresses a measured rect as an offset from whichever edges the overlay is actually
 *  anchored to (its corner), instead of always left/top. A native window resize that keeps
 *  the anchored corner pinned leaves that offset correct automatically — there is no window
 *  size or resize-timing assumption baked in, unlike positioning via left/top, which is only
 *  ever correct relative to a fixed top-left origin that a corner-anchored resize does not
 *  preserve. */
export function toAnchoredRect(rect: RectLike, borderRadius: number, corner: Corner, viewport: ViewportLike): AnchoredRect {
  const xSide: "left" | "right" = corner.endsWith("left") ? "left" : "right";
  const ySide: "top" | "bottom" = corner.startsWith("top") ? "top" : "bottom";
  return {
    xSide,
    ySide,
    x: xSide === "left" ? rect.left : viewport.width - rect.left - rect.width,
    y: ySide === "top" ? rect.top : viewport.height - rect.top - rect.height,
    width: rect.width,
    height: rect.height,
    borderRadius,
  };
}

function anchoredRectStyle(rect: AnchoredRect): Keyframe {
  return {
    [rect.xSide]: `${rect.x}px`,
    [rect.ySide]: `${rect.y}px`,
    width: `${rect.width}px`,
    height: `${rect.height}px`,
    borderRadius: `${rect.borderRadius}px`,
  } as Keyframe;
}

/** Direct pixel-value interpolation of the box's own geometry (its anchored offset, plus
 *  width/height/radius) from `from` to `to` — an exact shape-to-shape blend with no
 *  independent-axis scaling, so there's no intermediate ellipse distortion and no border-width
 *  stretching (unlike animating via `transform: scale()`, where a fixed-width border visually
 *  thickens or thins with it). `from` and `to` must share the same corner (it doesn't change
 *  mid-animation), so they always animate the same pair of CSS properties. */
export function morphKeyframes(from: AnchoredRect, to: AnchoredRect): Keyframe[] {
  return [anchoredRectStyle(from), anchoredRectStyle(to)];
}

/** Cross-fades the ghost out and the real destination element in during the final quarter
 *  of the morph, so the shape settles before the content swap becomes visible. */
export function crossfadeKeyframes(direction: "out" | "in"): Keyframe[] {
  return direction === "out"
    ? [{ opacity: 1, offset: 0 }, { opacity: 1, offset: 0.75 }, { opacity: 0, offset: 1 }]
    : [{ opacity: 0, offset: 0 }, { opacity: 0, offset: 0.75 }, { opacity: 1, offset: 1 }];
}

export interface FlipDelta {
  dx: number;
  dy: number;
}

/** How far an element that stayed in the DOM (e.g. a sibling card that didn't move
 *  intentionally) needs to travel to appear un-moved, before sliding back to `after`. Real
 *  elements reflow normally (unlike the fixed-position ghost above), so a plain viewport-pixel
 *  delta is safe here regardless of any window resize in between. */
export function flipDelta(before: RectLike, after: RectLike): FlipDelta {
  return { dx: before.left - after.left, dy: before.top - after.top };
}

const FLIP_NEGLIGIBLE_PX = 0.5;

export function isNegligibleFlipDelta(delta: FlipDelta): boolean {
  return Math.abs(delta.dx) < FLIP_NEGLIGIBLE_PX && Math.abs(delta.dy) < FLIP_NEGLIGIBLE_PX;
}

export function flipKeyframes(delta: FlipDelta): Keyframe[] {
  return [{ transform: `translate(${delta.dx}px, ${delta.dy}px)` }, { transform: "translate(0px, 0px)" }];
}

export function prefersReducedMotion(): boolean {
  return typeof window !== "undefined" && (window.matchMedia?.("(prefers-reduced-motion: reduce)").matches ?? false);
}

export function supportsElementAnimate(): boolean {
  return typeof Element !== "undefined" && typeof Element.prototype.animate === "function";
}
