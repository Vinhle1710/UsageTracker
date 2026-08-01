import type { Provider } from "./types";

export interface LogicalCardRegion {
  provider: Provider;
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

export interface MeasuredProviderRect extends MeasuredRect {
  provider: Provider;
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

export function shouldCommitGeometryRequest(
  sequence: number,
  latestSequence: number,
  applied: boolean,
): boolean {
  return applied && sequence === latestSequence;
}

export function calculateOverlayGeometry(
  root: RectOrigin,
  cards: MeasuredProviderRect[],
  padding: number,
  radius: number,
): OverlayGeometryMeasurement {
  if (!cards.length) return { regions: [], contentHeight: null };
  const regions = cards.map((card) => ({
    provider: card.provider,
    x: card.left - root.left,
    y: card.top - root.top,
    width: card.width,
    height: card.height,
    radius,
  }));
  const bottom = Math.max(...cards.map((card) => card.bottom - root.top));
  return { regions, contentHeight: Math.ceil(bottom + padding) };
}
