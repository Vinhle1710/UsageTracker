import { useEffect, useRef, useState } from "react";
import { historyBounds } from "./range";
import type { BillingAggregate, HistoryRange, HistoryResult } from "./types";
import { queryBilling, queryHistory } from "./api";
import { HistoryChart } from "./HistoryChart";
import { BillingTable } from "./BillingTable";
import { ExportControls } from "./ExportControls";
import { useSurfaceMotion } from "../motion/use-surface-motion";

const empty: HistoryResult = { points: [], billing: [] };
const rangeLabels: Record<HistoryRange, string> = { "5h": "5 hours", "24h": "24 hours", "7d": "7 days", "30d": "30 days" };

export function HistoryApp({ now = () => Math.floor(Date.now() / 1000) }: { now?: () => number }) {
  const [range, setRange] = useState<HistoryRange>("24h");
  const [provider, setProvider] = useState("");
  const [series, setSeries] = useState("");
  const [result, setResult] = useState(empty);
  const [aggregates, setAggregates] = useState<BillingAggregate[]>([]);
  const [error, setError] = useState("");
  const [billingError, setBillingError] = useState("");
  const [loading, setLoading] = useState(false);
  const [billingLoading, setBillingLoading] = useState(false);
  const request = useRef(0);
  const surfaceRoot = useRef<HTMLElement>(null);
  const motionRevision = `${range}:${provider}:${series}:${loading}:${billingLoading}:${result.points.length}:${aggregates.length}`;
  useSurfaceMotion(surfaceRoot, motionRevision);

  const bounds = historyBounds(range, now());
  const query = { ...bounds, provider: provider || null, windowKind: series || null };

  useEffect(() => {
    const id = ++request.current;
    setLoading(true);
    setBillingLoading(true);
    setError("");
    setBillingError("");
    setAggregates([]);
    queryHistory(query).then((value) => { if (id === request.current) setResult(value); }).catch((reason) => { if (id === request.current) setError(String(reason)); }).finally(() => { if (id === request.current) setLoading(false); });
    queryBilling(query).then((value) => { if (id === request.current) setAggregates(value); }).catch((reason) => { if (id === request.current) setBillingError(String(reason)); }).finally(() => { if (id === request.current) setBillingLoading(false); });
  }, [range, provider, series]);

  const metric = (read: (point: HistoryResult["points"][number]) => number | null) => {
    const values = result.points.map(read).filter((value): value is number => value != null);
    return values.length ? values.reduce((total, value) => total + value, 0).toLocaleString() : "Unavailable from provider";
  };
  const metrics = [
    { label: "Session usage", value: metric((point) => point.windowKind === "session_5h" ? point.usedPercent : null), unit: "%", tone: "amber" },
    { label: "Weekly usage", value: metric((point) => point.windowKind === "weekly_7d" ? point.usedPercent : null), unit: "%", tone: "cyan" },
    { label: "API calls", value: metric((point) => point.apiCalls), unit: "", tone: "violet" },
    { label: "Estimated cost", value: metric((point) => point.estimatedCostMicros), unit: " μ", tone: "coral" },
    { label: "Overage", value: metric((point) => point.overageCostMicros), unit: " μ", tone: "muted" },
  ];
  const models = Array.from(new Set(result.points.map((point) => point.model).filter((model): model is string => Boolean(model))));
  const seriesOptions = Array.from(new Set(result.points.map((point) => point.windowKind)));

  return <main ref={surfaceRoot} className="history-shell">
    <div className="history-scroll" data-smooth-scroll>
      <header className="history-header surface-motion-item">
        <div className="history-header__title"><span className="history-header__mark" aria-hidden="true">UT</span><div><p>Usage telemetry / archive</p><h1>Signal archive</h1></div></div>
        <div className="history-range-readout"><span>Window</span><strong className="telemetry-value">{rangeLabels[range]}</strong></div>
        <div className="history-commandbar">
          <div className="history-commandbar__ranges"><span>Timeline</span><nav className="history-range" aria-label="History range">{(Object.keys(rangeLabels) as HistoryRange[]).map((value) => <button className="surface-control" type="button" key={value} onClick={() => setRange(value)} aria-pressed={range === value}>{rangeLabels[value]}</button>)}</nav></div>
          <div className="history-filters" aria-label="History filters"><span className="history-filters__label">Live filters</span><label>Provider<select className="surface-control" value={provider} onChange={(event) => setProvider(event.target.value)}><option value="">All providers</option><option value="claude">Claude</option><option value="openai">OpenAI</option></select></label><label>Series<select className="surface-control" value={series} onChange={(event) => setSeries(event.target.value)}><option value="">All series</option>{seriesOptions.map((value) => <option key={value}>{value}</option>)}</select></label></div>
        </div>
      </header>

      <div className="history-content">
        <section className="history-summary surface-motion-item" aria-label="History summary">
          <div className="history-section-label"><span>01</span><h2>Signal scan</h2><p>Accumulated values inside the selected window.</p></div>
          <div className="history-metric-grid">{metrics.map((item) => { const available = item.value !== "Unavailable from provider"; return <article className="history-metric-card surface-motion-item" data-tone={item.tone} key={item.label}><span className="history-metric-card__pulse" aria-hidden="true"/><p>{item.label}</p><strong className="telemetry-value">{item.value}{available ? item.unit : ""}</strong><small>{available ? "window aggregate" : "source unavailable"}</small></article>; })}</div>
        </section>

        <section className="history-chart-panel surface-motion-item" aria-labelledby="trend-title">
          <div className="history-panel-heading"><div><span>02 / Trend field</span><h2 id="trend-title">Usage velocity</h2></div><p className="telemetry-value">{result.points.length} samples</p></div>
          <HistoryChart points={result.points} selectedSeries={series} range={range} loading={loading} error={error}/>
        </section>

        <div className="history-data-grid">
          <section className="history-data-panel surface-motion-item" aria-labelledby="models-title"><div className="history-panel-heading"><div><span>03 / Distribution</span><h2 id="models-title">Model mix</h2></div></div>{models.length ? <div className="history-table-wrap"><table className="history-table"><caption>Usage by model</caption><thead><tr><th scope="col">Model</th><th scope="col">Samples</th></tr></thead><tbody>{models.map((model) => <tr key={model}><th scope="row">{model}</th><td className="telemetry-value">{result.points.filter((point) => point.model === model).length}</td></tr>)}</tbody></table></div> : <div className="history-empty"><span aria-hidden="true">—</span><p>Per-model data unavailable</p><small>This provider only reports aggregate windows.</small></div>}</section>
          <section className="history-data-panel surface-motion-item" aria-labelledby="billing-title"><div className="history-panel-heading"><div><span>04 / Ledger</span><h2 id="billing-title">Billing</h2></div></div><BillingTable entries={result.billing} aggregates={aggregates} loading={billingLoading} error={billingError}/></section>
        </div>

        <ExportControls query={query} onCleared={() => { setResult(empty); setAggregates([]); }}/>
      </div>
    </div>
  </main>;
}
