import { useEffect, useRef, useState } from "react";
import { historyBounds } from "./range";
import type { BillingAggregate, HistoryRange, HistoryResult } from "./types";
import { queryBilling, queryHistory } from "./api";
import { HistoryChart } from "./HistoryChart";
import { BillingTable } from "./BillingTable";
import { ExportControls } from "./ExportControls";

const empty: HistoryResult = { points: [], billing: [] };

export function HistoryApp({
  now = () => Math.floor(Date.now() / 1000),
}: {
  now?: () => number;
}) {
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

  const bounds = historyBounds(range, now());
  const query = {
    ...bounds,
    provider: provider || null,
    windowKind: series || null,
  };

  useEffect(() => {
    const id = ++request.current;
    setLoading(true);
    setBillingLoading(true);
    setError("");
    setBillingError("");
    setAggregates([]);

    queryHistory(query)
      .then((value) => {
        if (id === request.current) setResult(value);
      })
      .catch((reason) => {
        if (id === request.current) setError(String(reason));
      })
      .finally(() => {
        if (id === request.current) setLoading(false);
      });

    queryBilling(query)
      .then((value) => {
        if (id === request.current) setAggregates(value);
      })
      .catch((reason) => {
        if (id === request.current) setBillingError(String(reason));
      })
      .finally(() => {
        if (id === request.current) setBillingLoading(false);
      });
  }, [range, provider, series]);

  const metric = (
    read: (point: HistoryResult["points"][number]) => number | null,
  ) => {
    const values = result.points.map(read).filter((value): value is number => value != null);
    return values.length
      ? values.reduce((total, value) => total + value, 0).toLocaleString()
      : "Unavailable from provider";
  };

  const models = Array.from(
    new Set(result.points.map((point) => point.model).filter((model): model is string => Boolean(model))),
  );

  return (
    <main>
      <h1>History</h1>
      <nav aria-label="History range">
        {(["5h", "24h", "7d", "30d"] as HistoryRange[]).map((value) => (
          <button
            type="button"
            key={value}
            onClick={() => setRange(value)}
            aria-pressed={range === value}
          >
            {value === "5h" ? "5 hours" : value === "24h" ? "24 hours" : value === "7d" ? "7 days" : "30 days"}
          </button>
        ))}
      </nav>

      <label>
        Provider{" "}
        <select value={provider} onChange={(event) => setProvider(event.target.value)}>
          <option value="">All providers</option>
          <option value="claude">Claude</option>
          <option value="openai">OpenAI</option>
        </select>
      </label>
      <label>
        Series{" "}
        <select value={series} onChange={(event) => setSeries(event.target.value)}>
          <option value="">All series</option>
          {Array.from(new Set(result.points.map((point) => point.windowKind))).map((value) => (
            <option key={value}>{value}</option>
          ))}
        </select>
      </label>

      <section aria-label="History summary">
        <h2>Summary</h2>
        <p>Session usage: {metric((point) => (point.windowKind === "session_5h" ? point.usedPercent : null))}%</p>
        <p>Weekly usage: {metric((point) => (point.windowKind === "weekly_7d" ? point.usedPercent : null))}%</p>
        <p>API calls: {metric((point) => point.apiCalls)}</p>
        <p>Estimated cost: {metric((point) => point.estimatedCostMicros)}</p>
        <p>Overage: {metric((point) => point.overageCostMicros)}</p>
      </section>

      {models.length ? (
        <table>
          <caption>Usage by model</caption>
          <thead>
            <tr>
              <th scope="col">Model</th>
              <th scope="col">Samples</th>
            </tr>
          </thead>
          <tbody>
            {models.map((model) => (
              <tr key={model}>
                <th scope="row">{model}</th>
                <td>{result.points.filter((point) => point.model === model).length}</td>
              </tr>
            ))}
          </tbody>
        </table>
      ) : (
        <p>Per-model data unavailable</p>
      )}

      <HistoryChart
        points={result.points}
        selectedSeries={series}
        range={range}
        loading={loading}
        error={error}
      />

      <section aria-label="Billing">
        <h2>Billing</h2>
        <BillingTable
          entries={result.billing}
          aggregates={aggregates}
          loading={billingLoading}
          error={billingError}
        />
      </section>

      <ExportControls
        query={query}
        onCleared={() => {
          setResult(empty);
          setAggregates([]);
        }}
      />
    </main>
  );
}
