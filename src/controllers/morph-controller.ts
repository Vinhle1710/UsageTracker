export class MorphController {
  private cancelled = false;
  cancelAll(): void { this.cancelled = true; }
  get isCancelled(): boolean { return this.cancelled; }
}
