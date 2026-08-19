import type { IndicatorStyle, MetricId, ValueMode, Provider } from "../types";

export interface MetricSnapshot { usedPercent: number; }
export interface DisplaySnapshot { provider: Provider | string; metrics: Partial<Record<MetricId, MetricSnapshot>>; }
export function snapshotFromWindows(provider: string, windows: Array<{label:string;used_percent:number}>): DisplaySnapshot { const metrics: Partial<Record<MetricId,MetricSnapshot>> = {}; for (const w of windows) { const label=w.label.toLowerCase(); const id:MetricId = label.includes("week") || label.includes("7 day") ? "weekly" : label.includes("api") || label.includes("account") ? "api" : "session"; if (!metrics[id]) metrics[id]={usedPercent:w.used_percent}; } return {provider,metrics}; }
export interface DisplayPreferences { valueMode: ValueMode; indicatorStyle: IndicatorStyle; enabledMetrics: MetricId[]; metricOrder: MetricId[]; }
export interface DisplayMetric { id: MetricId; usedPercent: number; displayPercent: number; label: string; severity: "normal" | "warning" | "critical"; }
export interface DisplayModel { provider: string; style: IndicatorStyle; metrics: DisplayMetric[]; }
export function buildDisplayModel(snapshot: DisplaySnapshot, prefs: DisplayPreferences): DisplayModel {
 const metrics: DisplayMetric[] = prefs.metricOrder.filter(id => prefs.enabledMetrics.includes(id)).flatMap(id => { const value=snapshot.metrics[id]; if(!value)return []; const used=Math.max(0,Math.min(100,value.usedPercent)); const display=prefs.valueMode === "used" ? used : 100-used; const severity: DisplayMetric["severity"] = used>=90?"critical":used>=75?"warning":"normal"; return [{id,usedPercent:used,displayPercent:display,label:prefs.valueMode === "used" ? `${Math.round(used)}% used` : `${Math.round(display)}% remaining`,severity}]; });
 return { provider: String(snapshot.provider), style: prefs.indicatorStyle, metrics };
}
