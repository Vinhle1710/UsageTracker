import type { BillingEntry } from "./types";
export function BillingTable({ entries, loading = false, error = "" }: { entries: BillingEntry[]; loading?: boolean; error?: string }) {
  if (loading) return <p role="status">Loading billing…</p>;
  if (error) return <p role="alert">{error}</p>;
  if (!entries.length) return <p role="status">No billing entries in this range.</p>;
  return <table><caption>Billing history</caption><thead><tr><th scope="col">Provider</th><th scope="col">UTC period</th><th scope="col">Source</th><th scope="col">Amount</th></tr></thead><tbody>{entries.map((e, i) => <tr key={`${e.provider}-${e.periodStart}-${i}`}><td>{e.provider}</td><td>{new Date(e.periodStart * 1000).toISOString()} – {new Date(e.periodEnd * 1000).toISOString()}</td><td>{e.source}</td><td>{(e.amountMicros / 1e6).toFixed(6)} {e.currency}</td></tr>)}</tbody></table>;
}
