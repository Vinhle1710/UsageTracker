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
  const measured = [...cards, ...bubbles];
  const union = bubbleRow ? [...measured, bubbleRow] : measured;
  if (!union.length) return { regions: [], contentWidth: null, contentHeight: null };

  const left = Math.min(...union.map((rect) => rect.left));
  const top = Math.min(...union.map((rect) => rect.top));
  const right = Math.max(...union.map((rect) => rect.right));
  const bottom = Math.max(...union.map((rect) => rect.bottom));
  const region = (rect: MeasuredRect, regionRadius: number): LogicalCardRegion => ({
    x: rect.left - left + padding,
    y: rect.top - top + padding,
    width: rect.width,
    height: rect.height,
    radius: regionRadius,
  });

  return {
    regions: [
      ...cards.map((card) => region(card, radius)),
      ...bubbles.map((bubble) => region(bubble, bubbleRadius)),
    ],
    contentWidth: Math.ceil(right - left + padding * 2),
    contentHeight: Math.ceil(bottom - top + padding * 2),
  };
}
