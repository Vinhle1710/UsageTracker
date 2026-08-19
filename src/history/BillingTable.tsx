import type { BillingAggregate, BillingEntry } from "./types";

type BillingTableProps = {
  entries: BillingEntry[];
  aggregates?: BillingAggregate[];
  loading?: boolean;
  error?: string;
};

const formatAmount = (amountMicros: number, currency: string) =>
  `${(amountMicros / 1_000_000).toFixed(6)} ${currency}`;

export function BillingTable({
  entries,
  aggregates = [],
  loading = false,
  error = "",
}: BillingTableProps) {
  if (loading) return <p role="status">Loading billing…</p>;
  if (error) return <p role="alert">{error}</p>;

  return (
    <>
      {aggregates.length > 0 && (
        <table>
          <caption>Billing totals</caption>
          <thead>
            <tr>
              <th scope="col">Provider</th>
              <th scope="col">Source</th>
              <th scope="col">Amount</th>
              <th scope="col">Currency</th>
            </tr>
          </thead>
          <tbody>
            {aggregates.map((aggregate, index) => (
              <tr
                key={`${aggregate.provider}-${aggregate.source}-${aggregate.currency}-${index}`}
              >
                <td>{aggregate.provider}</td>
                <td>{aggregate.source}</td>
                <td>{formatAmount(aggregate.amountMicros, aggregate.currency)}</td>
                <td>{aggregate.currency}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {entries.length > 0 && (
        <table>
          <caption>Billing history</caption>
          <thead>
            <tr>
              <th scope="col">Provider</th>
              <th scope="col">UTC period</th>
              <th scope="col">Source</th>
              <th scope="col">Amount</th>
            </tr>
          </thead>
          <tbody>
            {entries.map((entry, index) => (
              <tr key={`${entry.provider}-${entry.periodStart}-${index}`}>
                <td>{entry.provider}</td>
                <td>
                  {new Date(entry.periodStart * 1000).toISOString()} – {new Date(entry.periodEnd * 1000).toISOString()}
                </td>
                <td>{entry.source}</td>
                <td>{formatAmount(entry.amountMicros, entry.currency)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}

      {aggregates.length === 0 && entries.length === 0 && (
        <p role="status">No billing entries in this range.</p>
      )}
    </>
  );
}
