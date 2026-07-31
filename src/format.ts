export function formatPercent(percent: number): string {
  return `${Math.round(percent)}%`;
}

export function formatReset(resetsAt: number, now: number): string {
  const delta = resetsAt - now;
  if (delta <= 0) return "resetting";
  if (delta < 3600) return `resets in ${Math.round(delta / 60)}m`;
  if (delta < 86400) return `resets in ${Math.round(delta / 3600)}h`;
  return `resets in ${Math.round(delta / 86400)}d`;
}

export function formatAge(fetchedAt: number, now: number): string {
  const delta = Math.max(0, now - fetchedAt);
  if (delta < 60) return "just now";
  if (delta < 3600) return `${Math.floor(delta / 60)}m ago`;
  return `${Math.floor(delta / 3600)}h ago`;
}
