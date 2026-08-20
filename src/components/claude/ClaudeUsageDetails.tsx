import type { ClaudeUsageDetails as Details, Money } from "../../types";

const money = (value: Money) => new Intl.NumberFormat(undefined, { style: "currency", currency: value.currency }).format(value.minorUnits / 100);
export function ClaudeUsageDetails({ details }: { details: Details }) {
  return <section aria-labelledby="claude-usage-heading"><h3 id="claude-usage-heading">Claude usage</h3>
    <div aria-live="polite"><p data-state={details.limits.state}>{details.limits.state === "unavailable" ? "Model limits unavailable" : details.limits.state === "stale" ? "Model limits are stale" : null}</p>
    {details.limits.value?.map((limit) => <div key={limit.modelKey}><span>{limit.displayName || limit.modelKey}</span><meter min="0" max="100" value={Math.max(0, Math.min(100, limit.utilizationPercent))} aria-label={`${limit.displayName || limit.modelKey} utilization`} /> <span>{limit.utilizationPercent}%</span></div>)}</div>
    <div aria-live="polite"><p>{details.extra.state === "unavailable" ? "Claude Extra unavailable" : details.extra.state === "stale" ? "Claude Extra is stale" : null}</p>{details.extra.value && Object.entries(details.extra.value).map(([key, value]) => value ? <p key={key}>{key}: {money(value)}</p> : null)}</div>
  </section>;
}
