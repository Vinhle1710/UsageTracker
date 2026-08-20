import { useId, useMemo, useRef, useState } from "react";
import type { HistoryPoint, HistoryRange } from "./types";

const rangeNames: Record<HistoryRange, string> = { "5h": "5 hour", "24h": "24 hour", "7d": "7 day", "30d": "30 day" };
type Props = { points: HistoryPoint[]; selectedSeries?: string; range?: HistoryRange; loading?: boolean; error?: string };
const seriesLabel = (group: string) => group.replace("/", " · ").replaceAll("_", " ");

export function HistoryChart({ points, selectedSeries, range = "24h", loading, error }: Props) {
  const [active, setActive] = useState<HistoryPoint | null>(null);
  const refs = useRef<Record<string, HTMLButtonElement | null>>({});
  const gradientId = useId().replaceAll(":", "");
  const series = useMemo(() => selectedSeries ? points.filter((point) => point.windowKind === selectedSeries) : points, [points, selectedSeries]);

  if (loading) return <div className="history-chart-state" role="status"><i aria-hidden="true"/><p>Loading usage history…</p></div>;
  if (error) return <div className="history-chart-state history-chart-state--error" role="alert"><i aria-hidden="true"/><p>{error}</p></div>;
  if (!series.length) return <div className="history-chart-state" role="status"><i aria-hidden="true"/><p>No usage samples in this range.</p></div>;

  const min = Math.min(...series.map((point) => point.sampledAt));
  const max = Math.max(...series.map((point) => point.sampledAt), min + 1);
  const groups = Array.from(new Set(series.map((point) => `${point.provider}/${point.windowKind}`)));
  const pos = (point: HistoryPoint) => ({ x: (point.sampledAt - min) / (max - min) * 584 + 42, y: 220 - point.usedPercent * 1.86 });
  const summary = series.map((point) => `${point.provider} ${point.windowKind}: ${point.usedPercent}% at ${new Date(point.sampledAt * 1000).toISOString()}`).join("; ");

  return <section className="history-chart" aria-label="Usage history visualization">
    <div className="history-chart__legend" aria-label="Chart series">{groups.map((group, index) => <span className="history-chart__legend-item" key={group}><i style={{ background: `var(--history-series-${index % 4 + 1})` }} aria-hidden="true"/>{seriesLabel(group)}</span>)}</div>
    <div className="history-chart__plot">
      <svg role="img" aria-label={`${rangeNames[range]} usage history chart`} viewBox="0 0 640 240">
        <title>{`${rangeNames[range]} usage history chart`}</title>
        <desc>{summary}</desc>
        <defs>{groups.map((group, index) => <linearGradient id={`${gradientId}-${index}`} x1="0" y1="0" x2="0" y2="1" key={group}><stop offset="0%" stopColor={`var(--history-series-${index % 4 + 1})`} stopOpacity=".18"/><stop offset="100%" stopColor={`var(--history-series-${index % 4 + 1})`} stopOpacity="0"/></linearGradient>)}</defs>
        {[20, 70, 120, 170, 220].map((y, index) => <g data-chart-grid key={y}><line x1="42" x2="626" y1={y} y2={y}/><text x="4" y={y + 3}>{100 - index * 25}</text></g>)}
        {groups.map((group, groupIndex) => {
          const values = series.filter((point) => `${point.provider}/${point.windowKind}` === group);
          const positions = values.map(pos);
          const line = positions.map((point, index) => `${index ? "L" : "M"} ${point.x} ${point.y}`).join(" ");
          const area = positions.length ? `${line} L ${positions.at(-1)!.x} 220 L ${positions[0].x} 220 Z` : "";
          return <g key={group} data-series={group}><path className="history-chart__area" d={area} fill={`url(#${gradientId}-${groupIndex})`}/><path data-history-line className="history-chart__line" d={line} fill="none" stroke={`var(--history-series-${groupIndex % 4 + 1})`}/>{positions.map((point, index) => <circle key={values[index].sampledAt} cx={point.x} cy={point.y} r="3.5" fill={`var(--history-series-${groupIndex % 4 + 1})`} aria-hidden="true"/>)}</g>;
        })}
      </svg>
      {groups.flatMap((group) => {
        const values = series.filter((point) => `${point.provider}/${point.windowKind}` === group);
        return values.map((point, index) => {
          const position = pos(point);
          const key = `${group}-${point.sampledAt}`;
          return <button key={key} ref={(node) => { refs.current[key] = node; }} type="button" style={{ left: `${position.x / 640 * 100}%`, top: `${position.y / 240 * 100}%` }} aria-label={`${point.provider} ${point.windowKind} ${point.usedPercent}%`} onFocus={() => setActive(point)} onMouseEnter={() => setActive(point)} onMouseMove={() => setActive(point)} onKeyDown={(event) => { if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") return; event.preventDefault(); const next = values[Math.max(0, Math.min(values.length - 1, index + (event.key === "ArrowRight" ? 1 : -1)))]; refs.current[`${group}-${next.sampledAt}`]?.focus(); setActive(next); }}/>;
        });
      })}
    </div>
    <p className="history-chart__inspector telemetry-value" role="status" aria-live="polite">{active ? `${active.provider}, ${active.usedPercent}% (${active.windowKind})` : "Select a chart point to inspect usage."}</p>
    <p className="sr-only">{summary}</p>
  </section>;
}

