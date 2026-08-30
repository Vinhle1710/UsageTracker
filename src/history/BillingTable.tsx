import type { BillingAggregate, BillingEntry } from "./types";
import { formatMicros } from "../format";

type BillingTableProps = { entries: BillingEntry[]; aggregates?: BillingAggregate[]; loading?: boolean; error?: string };
const formatPeriod = (seconds: number) => new Date(seconds * 1000).toLocaleDateString(undefined, { year: "numeric", month: "short", day: "numeric" });

export function BillingTable({ entries, aggregates = [], loading = false, error = "" }: BillingTableProps) {
  if (loading) return <div className="history-inline-state" role="status"><i aria-hidden="true"/>Loading billing…</div>;
  if (error) return <div className="history-inline-state history-inline-state--error" role="alert">{error}</div>;
  if (aggregates.length === 0 && entries.length === 0) return <div className="history-empty" role="status"><span aria-hidden="true">—</span><p>No billing in this range</p><small>Costs appear here once a provider reports them.</small></div>;

  return <div className="history-table-wrap">
    {aggregates.length > 0 && <table className="history-table history-table--billing"><caption>Billing totals</caption><thead><tr><th scope="col">Provider</th><th scope="col">Source</th><th scope="col">Amount</th></tr></thead><tbody>{aggregates.map((aggregate, index) => <tr key={`${aggregate.provider}-${aggregate.source}-${aggregate.currency}-${index}`}><th scope="row">{aggregate.provider}</th><td>{aggregate.source}</td><td className="telemetry-value">{formatMicros(aggregate.amountMicros, aggregate.currency)}</td></tr>)}</tbody></table>}
    {entries.length > 0 && <table className="history-table history-table--billing"><caption>Billing history</caption><thead><tr><th scope="col">Provider</th><th scope="col">Period</th><th scope="col">Source</th><th scope="col">Amount</th></tr></thead><tbody>{entries.map((entry, index) => <tr key={`${entry.provider}-${entry.periodStart}-${index}`}><th scope="row">{entry.provider}</th><td className="telemetry-value">{formatPeriod(entry.periodStart)} – {formatPeriod(entry.periodEnd)}</td><td>{entry.source}</td><td className="telemetry-value">{formatMicros(entry.amountMicros, entry.currency)}</td></tr>)}</tbody></table>}
  </div>;
}
