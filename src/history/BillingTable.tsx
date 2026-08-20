import type { BillingAggregate, BillingEntry } from "./types";

type BillingTableProps = { entries: BillingEntry[]; aggregates?: BillingAggregate[]; loading?: boolean; error?: string };
const formatAmount = (amountMicros: number, currency: string) => `${(amountMicros / 1_000_000).toFixed(6)} ${currency}`;

export function BillingTable({ entries, aggregates = [], loading = false, error = "" }: BillingTableProps) {
  if (loading) return <div className="history-inline-state" role="status"><i aria-hidden="true"/>Loading billing…</div>;
  if (error) return <div className="history-inline-state history-inline-state--error" role="alert">{error}</div>;
  if (aggregates.length === 0 && entries.length === 0) return <div className="history-empty" role="status"><span aria-hidden="true">—</span><p>No billing entries in this range.</p><small>Cost records appear when the provider exposes them.</small></div>;

  return <div className="history-table-wrap">
    {aggregates.length > 0 && <table className="history-table history-table--billing"><caption>Billing totals</caption><thead><tr><th scope="col">Provider</th><th scope="col">Source</th><th scope="col">Amount</th><th scope="col">Currency</th></tr></thead><tbody>{aggregates.map((aggregate, index) => <tr key={`${aggregate.provider}-${aggregate.source}-${aggregate.currency}-${index}`}><td><span className="history-provider-dot" aria-hidden="true"/>{aggregate.provider}</td><td>{aggregate.source}</td><td className="telemetry-value">{formatAmount(aggregate.amountMicros, aggregate.currency)}</td><td className="telemetry-value">{aggregate.currency}</td></tr>)}</tbody></table>}
    {entries.length > 0 && <table className="history-table history-table--billing"><caption>Billing history</caption><thead><tr><th scope="col">Provider</th><th scope="col">UTC period</th><th scope="col">Source</th><th scope="col">Amount</th></tr></thead><tbody>{entries.map((entry, index) => <tr key={`${entry.provider}-${entry.periodStart}-${index}`}><td>{entry.provider}</td><td className="telemetry-value">{new Date(entry.periodStart * 1000).toISOString()} – {new Date(entry.periodEnd * 1000).toISOString()}</td><td>{entry.source}</td><td className="telemetry-value">{formatAmount(entry.amountMicros, entry.currency)}</td></tr>)}</tbody></table>}
  </div>;
}

