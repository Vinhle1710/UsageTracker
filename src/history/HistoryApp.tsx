import { useEffect, useRef, useState } from "react";
import { historyBounds } from "./range";
import type { BillingAggregate, HistoryPoint, HistoryRange, HistoryResult } from "./types";
import { queryBilling, queryHistory } from "./api";
import { HistoryChart } from "./HistoryChart";
import { BillingTable } from "./BillingTable";
import { ExportControls } from "./ExportControls";
import { formatMicros } from "../format";
import { useSurfaceMotion } from "../motion/use-surface-motion";

const empty: HistoryResult = { points: [], billing: [] };
const rangeLabels: Record<HistoryRange, string> = { "5h": "5 hours", "24h": "24 hours", "7d": "7 days", "30d": "30 days" };

const UNAVAILABLE = "Not reported";

interface Metric {
  label: string;
  value: string | null;
  note: string;
}

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
  const [generation, setGeneration] = useState(0);
  const surfaceRoot = useRef<HTMLElement>(null);
  // Keyed to completed loads, not to every state tick. Including the loading flags
  // and the row counts made one range click replay the whole staggered entrance
  // three times — and, because each replay rebuilds the Lenis instance, scroll
  // snapped back to the top mid-interaction.
  useSurfaceMotion(surfaceRoot, String(generation));

  const bounds = historyBounds(range, now());
  const query = { ...bounds, provider: provider || null, windowKind: series || null };

  useEffect(() => {
    const id = ++request.current;
    setLoading(true);
    setBillingLoading(true);
    setError("");
    setBillingError("");
    setAggregates([]);
    queryHistory(query).then((value) => { if (id === request.current) { setResult(value); setGeneration((count) => count + 1); } }).catch((reason) => { if (id === request.current) setError(String(reason)); }).finally(() => { if (id === request.current) setLoading(false); });
    queryBilling(query).then((value) => { if (id === request.current) setAggregates(value); }).catch((reason) => { if (id === request.current) setBillingError(String(reason)); }).finally(() => { if (id === request.current) setBillingLoading(false); });
  }, [range, provider, series]);

  // A quota percentage is a level, not a quantity: adding up every sample in the window
  // produced numbers like "4,000%" that mean nothing. The newest reading is what the
  // question "how much have I used?" actually asks for.
  const latestPercent = (windowKind: string): number | null => {
    const matching = result.points.filter((point) => point.windowKind === windowKind);
    if (!matching.length) return null;
    return matching.reduce((newest, point) => (point.sampledAt >= newest.sampledAt ? point : newest)).usedPercent;
  };
  // Counts and costs are quantities, so totalling them across the window is correct.
  const total = (read: (point: HistoryPoint) => number | null): number | null => {
    const values = result.points.map(read).filter((value): value is number => value != null);
    return values.length ? values.reduce((sum, value) => sum + value, 0) : null;
  };

  const session = latestPercent("session_5h");
  const weekly = latestPercent("weekly_7d");
  const apiCalls = total((point) => point.apiCalls);
  // HistoryPoint carries no currency of its own, unlike a billing entry; these are assumed
  // to be USD, which is what every provider reports today.
  const estimatedCost = total((point) => point.estimatedCostMicros);
  const overage = total((point) => point.overageCostMicros);

  const metrics: Metric[] = [
    { label: "Session usage", value: session === null ? null : `${session}%`, note: "Most recent reading" },
    { label: "Weekly usage", value: weekly === null ? null : `${weekly}%`, note: "Most recent reading" },
    { label: "API calls", value: apiCalls === null ? null : apiCalls.toLocaleString(), note: "Total for this range" },
    { label: "Estimated cost", value: estimatedCost === null ? null : formatMicros(estimatedCost), note: "Total for this range" },
    { label: "Overage", value: overage === null ? null : formatMicros(overage), note: "Total for this range" },
  ];
  const models = Array.from(new Set(result.points.map((point) => point.model).filter((model): model is string => Boolean(model))));
  const seriesOptions = Array.from(new Set(result.points.map((point) => point.windowKind)));

  return <main ref={surfaceRoot} className="history-shell">
    <div className="history-scroll" data-smooth-scroll>
      <header className="history-header surface-motion-item">
        <div className="history-header__title"><h1>History</h1><p>Usage and billing recorded on this device.</p></div>
        <div className="history-commandbar">
          <nav className="history-range" aria-label="Time range">{(Object.keys(rangeLabels) as HistoryRange[]).map((value) => <button className="surface-control" type="button" key={value} onClick={() => setRange(value)} aria-pressed={range === value}>{rangeLabels[value]}</button>)}</nav>
          <div className="history-filters">
            <label>Provider<select className="surface-control" value={provider} onChange={(event) => setProvider(event.target.value)}><option value="">All providers</option><option value="claude">Claude</option><option value="openai">OpenAI</option></select></label>
            <label>Series<select className="surface-control" value={series} onChange={(event) => setSeries(event.target.value)}><option value="">All series</option>{seriesOptions.map((value) => <option key={value}>{value}</option>)}</select></label>
          </div>
        </div>
      </header>

      <div className="history-content">
        <section className="history-summary surface-motion-item" aria-labelledby="summary-title">
          <div className="history-section-label"><h2 id="summary-title">Summary</h2><p>Across the last {rangeLabels[range]}.</p></div>
          <div className="history-metric-grid">{metrics.map((item) => <article className="history-metric-card surface-motion-item" data-available={String(item.value !== null)} key={item.label}>
            <p>{item.label}</p>
            <strong className={item.value === null ? undefined : "telemetry-value"}>{item.value ?? UNAVAILABLE}</strong>
            <small>{item.value === null ? "This provider does not send it" : item.note}</small>
          </article>)}</div>
        </section>

        <section className="history-chart-panel surface-motion-item" aria-labelledby="trend-title">
          <div className="history-panel-heading"><h2 id="trend-title">Usage over time</h2><p>{result.points.length} samples</p></div>
          <HistoryChart points={result.points} selectedSeries={series} range={range} loading={loading} error={error}/>
        </section>

        <div className="history-data-grid">
          <section className="history-data-panel surface-motion-item" aria-labelledby="models-title"><div className="history-panel-heading"><h2 id="models-title">By model</h2></div>{models.length ? <div className="history-table-wrap"><table className="history-table"><caption>Usage by model</caption><thead><tr><th scope="col">Model</th><th scope="col">Samples</th></tr></thead><tbody>{models.map((model) => <tr key={model}><th scope="row">{model}</th><td className="telemetry-value">{result.points.filter((point) => point.model === model).length}</td></tr>)}</tbody></table></div> : <div className="history-empty"><span aria-hidden="true">—</span><p>Per-model data unavailable</p><small>This provider only reports whole time windows.</small></div>}</section>
          <section className="history-data-panel surface-motion-item" aria-labelledby="billing-title"><div className="history-panel-heading"><h2 id="billing-title">Billing</h2></div><BillingTable entries={result.billing} aggregates={aggregates} loading={billingLoading} error={billingError}/></section>
        </div>

        <ExportControls query={query} onCleared={() => { setResult(empty); setAggregates([]); }}/>
      </div>
    </div>
  </main>;
}
