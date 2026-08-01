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

export interface GeometryRequestSequencer {
  request(key: string, apply: () => Promise<boolean>): Promise<void>;
  lastAppliedKey(): string;
}

export function createGeometryRequestSequencer(
  onApplied: (key: string) => void = () => undefined,
): GeometryRequestSequencer {
  let latestSequence = 0;
  let lastApplied = "";
  let pending = false;
  let queue: Promise<void> = Promise.resolve();

  return {
    request(key, apply) {
      const sequence = ++latestSequence;
      if (key === lastApplied && !pending) return queue;

      pending = true;
      const operation = queue.then(async () => {
        if (sequence !== latestSequence) return;
        const applied = await apply();
        if (!shouldCommitGeometryRequest(sequence, latestSequence, applied)) return;
        lastApplied = key;
        onApplied(key);
      });
      const completion = operation.finally(() => {
        if (sequence === latestSequence) pending = false;
      });
      queue = completion.catch(() => undefined);
      return queue;
    },
    lastAppliedKey() {
      return lastApplied;
    },
  };
}

export function calculateOverlayGeometry(
  root: RectOrigin,
  cards: MeasuredProviderRect[],
  padding: number,
  radius: number,
  includeCards = true,
): OverlayGeometryMeasurement {
  if (!includeCards || !cards.length) return { regions: [], contentHeight: null };
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

export async function restoreGeometryInTwoSteps(
  applyFallback: () => Promise<void>,
  waitForNextFrame: () => Promise<void>,
  applyMeasured: () => Promise<void>,
): Promise<void> {
  await applyFallback();
  await waitForNextFrame();
  await applyMeasured();
}
