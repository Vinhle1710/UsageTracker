export class GeometryController {
  private started = false;
  private effectOutset = 0;
  start(): void { this.started = true; }
  stop(): void { this.started = false; }
  get active(): boolean { return this.started; }
  setGlow(enabled: boolean): void { this.effectOutset = enabled ? 8 : 0; }
  getEffectOutset(): number { return this.effectOutset; }
}
