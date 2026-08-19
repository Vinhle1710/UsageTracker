export class GeometryController {
  private started = false;
  start(): void { this.started = true; }
  stop(): void { this.started = false; }
  get active(): boolean { return this.started; }
}
