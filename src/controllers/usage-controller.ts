import type { AppStore } from "../app/store";
import { rasterizeTrayIndicator } from "../display/tray-raster";
import { buildDisplayModel } from "../display/model";

type Unlisten = () => void;
type Runtime = { listen: (event: string, handler: (event: { payload: unknown }) => void) => Promise<Unlisten>; invoke: (command: string, args?: unknown) => Promise<unknown> };

export class UsageController {
  private refreshing = false;
  private started = false;
  private unlisteners: Unlisten[] = [];
  private trayTimer: ReturnType<typeof setTimeout> | undefined;
  constructor(private readonly store: AppStore, private readonly runtime: Runtime) {}
  async start(): Promise<void> {
    if (this.started) return;
    this.started = true;
    await this.runtime.invoke("get_bootstrap").then((payload) => this.store.dispatch({ type: "bootstrap", payload: payload as never })).catch(() => undefined);
    this.unlisteners.push(await this.runtime.listen("usage-changed", (event) => { this.store.dispatch({ type: "usage", payload: event.payload as never }); this.scheduleTray(); }));
    this.unlisteners.push(await this.runtime.listen("sources-changed", (event) => this.store.dispatch({ type: "sources", payload: event.payload as never })));
    this.unlisteners.push(await this.runtime.listen("refresh-started", () => { this.refreshing = true; }));
    this.unlisteners.push(await this.runtime.listen("refresh-completed", () => { this.refreshing = false; }));
  }
  async stop(): Promise<void> {
    if (!this.started) return;
    this.started = false;
    const pending = this.unlisteners.splice(0);
    pending.forEach((unlisten) => unlisten());
    if (this.trayTimer) clearTimeout(this.trayTimer);
  }
  private scheduleTray(): void { if (this.trayTimer) return; this.trayTimer=setTimeout(()=>{ this.trayTimer=undefined; const config=this.store.getSnapshot().config; if (config.showTrayIndicator === false) return; const providers=this.store.getSnapshot().providers; const first=Object.values(providers)[0]; const used=first?.snapshot?.windows[0]?.used_percent ?? 0; const image=rasterizeTrayIndicator(buildDisplayModel({provider:"usage",metrics:{session:{usedPercent:used}}},{valueMode:"used",indicatorStyle:"compact",enabledMetrics:["session"],metricOrder:["session"]}),{session:"#60a5fa"},32); void this.runtime.invoke("set_tray_indicator",image);},0); }
}
