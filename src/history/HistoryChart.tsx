import { useMemo, useRef, useState } from "react";
import type { HistoryPoint } from "./types";
type Props = { points: HistoryPoint[]; selectedSeries?: string; loading?: boolean; error?: string };
export function HistoryChart({ points, selectedSeries, loading, error }: Props) {
  const [active, setActive] = useState<HistoryPoint | null>(null); const buttons = useRef<Record<string, HTMLButtonElement | null>>({});
  const series = useMemo(() => selectedSeries ? points.filter(p => p.windowKind === selectedSeries) : points, [points, selectedSeries]);
  if (loading) return <p role="status" aria-live="polite">Loading usage history…</p>;
  if (error) return <p role="alert">{error}</p>;
  if (!series.length) return <p role="status">No usage samples in this range.</p>;
  const min = Math.min(...series.map(p => p.sampledAt)), max = Math.max(...series.map(p => p.sampledAt), min + 1);
  const groups = Array.from(new Set(series.map(p => `${p.provider}/${p.windowKind}`)));
  const pos = (p: HistoryPoint) => ({ x: ((p.sampledAt - min) / (max - min)) * 600 + 20, y: 220 - p.usedPercent * 2 });
  return <section aria-label="Usage history visualization"><div className="history-chart" style={{ position: "relative" }}>
    <svg role="img" aria-label="5 hour usage history chart" viewBox="0 0 640 240"><title>Usage history chart</title><desc>{series.map(p => `${p.provider} ${p.windowKind}: ${p.usedPercent}% at ${new Date(p.sampledAt * 1000).toISOString()}`).join("; ")}</desc>
      {groups.map((group, g) => { const ps = series.filter(p => `${p.provider}/${p.windowKind}` === group); return <g key={group} data-series={group} aria-label={`${group} series`}><polyline points={ps.map(p => { const q = pos(p); return `${q.x},${q.y}`; }).join(" ")} fill="none" stroke={`var(--history-series-${g % 4 + 1})`} />{ps.map(p => { const q = pos(p); return <circle key={`${group}-${p.sampledAt}`} cx={q.x} cy={q.y} r="5" fill={`var(--history-series-${g % 4 + 1})`} aria-hidden="true" />; })}</g>; })}
    </svg>{groups.flatMap(group => series.filter(p => `${p.provider}/${p.windowKind}` === group).map((p, i, ps) => { const q = pos(p), key = `${group}-${p.sampledAt}`; return <button key={key} ref={node => { buttons.current[key] = node; }} type="button" className="history-chart-point" style={{ position: "absolute", left: `${q.x / 640 * 100}%`, top: `${q.y / 240 * 100}%`, width: 24, height: 24, transform: "translate(-50%, -50%)", opacity: 0 }} aria-label={`${p.provider} ${p.windowKind} ${p.usedPercent}%`} onFocus={() => setActive(p)} onMouseEnter={() => setActive(p)} onKeyDown={event => { if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return; event.preventDefault(); const next = ps[Math.max(0, Math.min(ps.length - 1, i + (event.key === "ArrowRight" ? 1 : -1)))]; buttons.current[`${group}-${next.sampledAt}`]?.focus(); setActive(next); }} />; }))}</div><p role="status" aria-live="polite">{active ? `${active.provider}, ${active.usedPercent}% (${active.windowKind})` : "Select a chart point to inspect usage."}</p><p className="sr-only">{series.map(p => `${p.provider} ${p.windowKind}: ${p.usedPercent}% at ${new Date(p.sampledAt * 1000).toISOString()}`).join("; ")}</p></section>;
}
