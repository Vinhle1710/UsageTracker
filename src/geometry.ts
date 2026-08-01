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

interface MeasuredRect extends RectOrigin {
  width: number;
  height: number;
  right: number;
  bottom: number;
}

export interface OverlayGeometryMeasurement {
  regions: LogicalCardRegion[];
  contentHeight: number | null;
}

export function calculateOverlayGeometry(
  root: RectOrigin,
  cards: MeasuredRect[],
  padding: number,
  radius: number,
): OverlayGeometryMeasurement {
  if (!cards.length) return { regions: [], contentHeight: null };
  const regions = cards.map((card) => ({
    x: card.left - root.left,
    y: card.top - root.top,
    width: card.width,
    height: card.height,
    radius,
  }));
  const bottom = Math.max(...cards.map((card) => card.bottom - root.top));
  return { regions, contentHeight: Math.ceil(bottom + padding) };
}
