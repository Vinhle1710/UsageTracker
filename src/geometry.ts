export interface LogicalCardRegion {
  x: number;
  y: number;
  width: number;
  height: number;
  radius: number;
  effect_outset?: number;
}

interface RectOrigin {
  left: number;
  top: number;
}

export interface MeasuredRect extends RectOrigin {
  width: number;
  height: number;
  right: number;
  bottom: number;
}

export interface OverlayGeometryMeasurement {
  regions: LogicalCardRegion[];
  contentWidth: number | null;
  contentHeight: number | null;
}

function isValidMeasuredRect(rect: MeasuredRect): boolean {
  return [rect.left, rect.top, rect.right, rect.bottom, rect.width, rect.height]
    .every(Number.isFinite)
    && rect.width > 0
    && rect.height > 0
    && rect.right >= rect.left
    && rect.bottom >= rect.top;
}

/** Transparent slack kept on every side of the content, so an animation can overshoot past the
 *  card/bubble bounds — a sibling's springy slide dipping below its resting place, a burst ring
 *  expanding past its circle — without being cut off by the window's own edge. The window is
 *  clipped to the card shapes in the steady state, so this slack is completely invisible; it only
 *  becomes paintable while a morph temporarily opens the region up. Because the overlay sits in a
 *  screen corner, the window has to overhang the work area by this much to keep the *content*
 *  flush against that corner (see the headroom offset in apply_overlay_geometry). */
export const OVERLAY_HEADROOM = 64;

export function calculateOverlayGeometry(
  root: RectOrigin,
  cards: MeasuredRect[],
  bubbles: MeasuredRect[],
  padding: number,
  radius: number,
  bubbleRadius = 24,
  bubbleRow?: MeasuredRect | null,
  headroom = 0,
): OverlayGeometryMeasurement {
  void root;
  const validCards = cards.filter(isValidMeasuredRect);
  const validBubbles = bubbles.filter(isValidMeasuredRect);
  const measured = [...validCards, ...validBubbles];
  if (!measured.length) return { regions: [], contentWidth: null, contentHeight: null };
  const validBubbleRow = bubbleRow && isValidMeasuredRect(bubbleRow) ? bubbleRow : null;
  const union = validBubbleRow ? [...measured, validBubbleRow] : measured;

  const left = Math.min(...union.map((rect) => rect.left));
  const top = Math.min(...union.map((rect) => rect.top));
  const right = Math.max(...union.map((rect) => rect.right));
  const bottom = Math.max(...union.map((rect) => rect.bottom));
  const horizontalInset = (validCards.length ? padding : 0) + headroom;
  // Which vertical edge is flush (0 inset) is inferred from where the bubble row actually
  // measured, not assumed to always be the top: a bottom-anchored overlay renders its bubble
  // row below the card (see app.css's corner-conditional padding), and hardcoding "bubble is
  // always at the top" here produced a native window region that didn't cover where the bubble
  // actually paints, clipping it.
  const bubbleIsBelowCard = validCards.length > 0 && validBubbles.length > 0
    && Math.min(...validBubbles.map((bubble) => bubble.top)) > Math.min(...validCards.map((card) => card.top));
  const topInset = (!validBubbles.length ? padding : bubbleIsBelowCard ? padding : 0) + headroom;
  const bottomInset = (!validCards.length ? 0 : bubbleIsBelowCard ? 0 : padding) + headroom;
  const region = (rect: MeasuredRect, regionRadius: number): LogicalCardRegion => ({
    x: rect.left - left + horizontalInset,
    y: rect.top - top + topInset,
    width: rect.width,
    height: rect.height,
    radius: regionRadius,
  });

  const contentWidth = Math.ceil(right - left + horizontalInset * 2);
  const contentHeight = Math.ceil(bottom - top + topInset + bottomInset);
  if (!Number.isFinite(contentWidth) || contentWidth <= 0 || !Number.isFinite(contentHeight) || contentHeight <= 0) {
    return { regions: [], contentWidth: null, contentHeight: null };
  }

  return {
    regions: [
      ...validCards.map((card) => region(card, radius)),
      ...validBubbles.map((bubble) => region(bubble, bubbleRadius)),
    ],
    contentWidth,
    contentHeight,
  };
}
