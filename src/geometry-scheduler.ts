export interface GeometryRequestCounts {
  expandedProviderCount: number;
  bubbleCount: number;
}

export interface MeasuredGeometryRequest extends GeometryRequestCounts {
  regions: readonly unknown[];
}

function hasActiveContentWithoutRegions(request: MeasuredGeometryRequest): boolean {
  return request.expandedProviderCount + request.bubbleCount > 0 && request.regions.length === 0;
}

/**
 * DOM replacement and native focus can briefly expose an active overlay before its measurable
 * card node has completed layout. Sending that empty snapshot would make the native layer fall
 * back to the compact base window and clip an in-progress reveal. Give layout one frame to settle;
 * if it is still empty, keep the last valid native geometry instead.
 */
export async function measureGeometryWhenReady<T extends MeasuredGeometryRequest>(
  measure: () => T,
  waitForLayout: () => Promise<void>,
): Promise<T | null> {
  let request = measure();
  if (!hasActiveContentWithoutRegions(request)) return request;

  await waitForLayout();
  request = measure();
  return hasActiveContentWithoutRegions(request) ? null : request;
}

export class GeometryRequestScheduler<T extends GeometryRequestCounts> {
  private pending: { request: T; key: string; revision: number } | null = null;
  private running: Promise<void> | null = null;
  private revision = 0;
  private appliedGeometry = "";

  constructor(private readonly apply: (request: T) => Promise<unknown>) {}

  get lastGeometry(): string {
    return this.appliedGeometry;
  }

  enqueue(request: T): Promise<void> {
    this.revision += 1;
    if (request.expandedProviderCount === 0 && request.bubbleCount === 0) {
      this.pending = null;
      return this.running ?? Promise.resolve();
    }

    const key = JSON.stringify(request);
    if (!this.running && key === this.appliedGeometry) return Promise.resolve();
    this.pending = { request, key, revision: this.revision };
    if (!this.running) this.startDrain();
    return this.waitUntilIdle();
  }

  private startDrain(): void {
    const tracked = this.drain().finally(() => {
      if (this.running !== tracked) return;
      this.running = null;
      // An enqueue can land after drain() observes an empty queue but before this finally
      // callback runs. Restart here so that request cannot remain stranded indefinitely.
      if (this.pending) this.startDrain();
    });
    this.running = tracked;
  }

  private async waitUntilIdle(): Promise<void> {
    while (this.running) {
      await this.running;
    }
  }

  private async drain(): Promise<void> {
    while (this.pending) {
      const next = this.pending;
      this.pending = null;
      try {
        await this.apply(next.request);
      } catch {
        continue;
      }
      if (!this.pending && next.revision === this.revision) {
        this.appliedGeometry = next.key;
      }
    }
  }
}
