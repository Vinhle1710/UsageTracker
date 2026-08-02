export interface LogicalCardRegion {
  x: number;
  y: number;
  width: number;
  height: number;
  radius: number;
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

export function calculateOverlayGeometry(
  root: RectOrigin,
  cards: MeasuredRect[],
  bubbles: MeasuredRect[],
  padding: number,
  radius: number,
  bubbleRadius = 24,
  bubbleRow?: MeasuredRect | null,
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
  const horizontalInset = validCards.length ? padding : 0;
  const topInset = validBubbles.length ? 0 : padding;
  const bottomInset = validCards.length ? padding : 0;
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
