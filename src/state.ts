import type { ActiveSources, Layout, SizeState, UsageSnapshot } from "./types";

export function nextSize(current: SizeState): SizeState {
  return current === "compact" ? "square" : "compact";
}

export function nextLayout(current: Layout): Layout {
  return current === "stacked-compact" ? "provider-columns" : "stacked-compact";
}

export function worstPercent(snapshots: UsageSnapshot[]): number | null {
  const values = snapshots.flatMap((snapshot) => snapshot.windows.map((window) => window.used_percent));
  return values.length ? Math.max(...values) : null;
}

export function visibleLayers(sources: ActiveSources): Array<"claude" | "openai"> {
  const layers: Array<"claude" | "openai"> = [];
  if (sources.claude) layers.push("claude");
  if (sources.openai) layers.push("openai");
  return layers;
}
