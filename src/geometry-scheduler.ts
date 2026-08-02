export interface GeometryRequestCounts {
  expandedProviderCount: number;
  bubbleCount: number;
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
    if (!this.running) {
      this.running = this.drain().finally(() => {
        this.running = null;
      });
    }
    return this.running;
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
