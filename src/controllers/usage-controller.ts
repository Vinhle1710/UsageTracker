import type { AppStore } from "../app/store";

type Unlisten = () => void;
type Runtime = { listen: (event: string, handler: (event: { payload: unknown }) => void) => Promise<Unlisten>; invoke: (command: string, args?: unknown) => Promise<unknown> };

export class UsageController {
  private started = false;
  private unlisteners: Unlisten[] = [];
  constructor(private readonly store: AppStore, private readonly runtime: Runtime) {}
  async start(): Promise<void> {
    if (this.started) return;
    this.started = true;
    await this.runtime.invoke("get_bootstrap").then((payload) => this.store.dispatch({ type: "bootstrap", payload: payload as never })).catch(() => undefined);
    this.unlisteners.push(await this.runtime.listen("usage-changed", (event) => this.store.dispatch({ type: "usage", payload: event.payload as never })));
    this.unlisteners.push(await this.runtime.listen("sources-changed", (event) => this.store.dispatch({ type: "sources", payload: event.payload as never })));
  }
  async stop(): Promise<void> {
    if (!this.started) return;
    this.started = false;
    const pending = this.unlisteners.splice(0);
    pending.forEach((unlisten) => unlisten());
  }
}
