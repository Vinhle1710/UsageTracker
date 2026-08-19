import { formatPercent, formatReset } from "../../format";
import type { Provider, UsageSnapshot } from "../../types";
import { Indicator } from "../../display/Indicator";
import { buildDisplayModel, snapshotFromWindows } from "../../display/model";

export function ProviderLayer({ provider, snapshot, onAction }: { provider: Provider; snapshot?: UsageSnapshot; onAction?: (action: "minimize" | "restore" | "open-settings") => void }) {
  const name = provider === "claude" ? "Claude" : "ChatGPT";
  if (!snapshot) return <section data-testid={`provider-${provider}`} data-provider={provider} className="layer layer--loading" aria-busy="true"><h2>{name}</h2><p>Loading usage…</p></section>;
  return <section data-testid={`provider-${provider}`} data-provider={provider} className="layer" aria-labelledby={`layer-${provider}`}>
    <h2 id={`layer-${provider}`} className="layer__title">{name}</h2>
    {snapshot.windows.length ? <><Indicator model={buildDisplayModel(snapshotFromWindows(provider, snapshot.windows), { valueMode: "used", indicatorStyle: "compact", enabledMetrics: ["session", "weekly", "api"], metricOrder: ["session", "weekly", "api"] })} /><div className="window-grid">{snapshot.windows.map((window) => <div className="window-card" key={window.label}>
      <div className="meter" role="progressbar" aria-valuenow={Math.round(window.used_percent)} aria-valuemin={0} aria-valuemax={100} aria-label={`${name} ${window.label} usage`}><span className="meter__value">{formatPercent(window.used_percent)}</span></div>
      <span className="window-card__label">{window.label}</span><span className="window-card__reset">{formatReset(window.label, window.resets_at, Math.floor(Date.now() / 1000))}</span>
    </div>)}</div></> : <p className="layer__empty">{snapshot.state === "signed-out" ? "Not signed in" : snapshot.state === "error" ? "Sign-in required" : "No usage limits reported"}</p>}
    <button type="button" className="minimize-control__button" onClick={() => onAction?.("minimize")}>Minimize</button>
  </section>;
}
