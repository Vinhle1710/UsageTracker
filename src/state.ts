import type { ActiveSources, Config, Layout, Provider, ProviderUsageEvent, SizeState, SnapshotMap, UsageSnapshot } from "./types";

export interface ProviderRecord {
  active: boolean;
  snapshot?: UsageSnapshot;
  previousSnapshot?: UsageSnapshot;
}

export type ProviderState = Record<Provider, ProviderRecord>;

type GeometrySettings = Pick<Config, "monitorId" | "corner" | "scale" | "layout" | "theme" | "backgroundColor" | "cardOpacity">;

export function sameSources(left: ActiveSources, right: ActiveSources): boolean {
  return left.claude === right.claude && left.openai === right.openai;
}

export function geometryChanged(left: GeometrySettings, right: GeometrySettings): boolean {
  return left.monitorId !== right.monitorId
    || left.corner !== right.corner
    || left.scale !== right.scale
    || left.layout !== right.layout
    || left.theme !== right.theme
    || left.backgroundColor !== right.backgroundColor
    || left.cardOpacity !== right.cardOpacity;
}

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

export function createProviderState(sources: ActiveSources, snapshots: SnapshotMap = {}): ProviderState {
  return {
    claude: { active: sources.claude, snapshot: snapshots.claude },
    openai: { active: sources.openai, snapshot: snapshots.openai },
  };
}

export function updateProviderSources(current: ProviderState, sources: ActiveSources): ProviderState {
  return {
    claude: { ...current.claude, active: sources.claude },
    openai: { ...current.openai, active: sources.openai },
  };
}

export function updateProviderUsage(current: ProviderState, event: ProviderUsageEvent): ProviderState {
  const existing = current[event.provider];
  if (existing.snapshot && existing.snapshot.fetched_at > event.snapshot.fetched_at) return current;
  return {
    ...current,
    [event.provider]: {
      ...existing,
      previousSnapshot: existing.snapshot,
      snapshot: event.snapshot,
    },
  };
}

export function providerSnapshots(state: ProviderState): SnapshotMap {
  const snapshots: SnapshotMap = {};
  if (state.claude.snapshot) snapshots.claude = state.claude.snapshot;
  if (state.openai.snapshot) snapshots.openai = state.openai.snapshot;
  return snapshots;
}

export function providerPreviousSnapshots(state: ProviderState): SnapshotMap {
  const snapshots: SnapshotMap = {};
  if (state.claude.previousSnapshot) snapshots.claude = state.claude.previousSnapshot;
  if (state.openai.previousSnapshot) snapshots.openai = state.openai.previousSnapshot;
  return snapshots;
}

export function initialSnapshots(usePreviewData: boolean, currentTime: number): SnapshotMap {
  if (!usePreviewData) return {};
  const preview = (used: number, resetAfter: number): UsageSnapshot => ({
    windows: [
      { label: "5 hour", used_percent: used, resets_at: currentTime + resetAfter },
      { label: "Weekly", used_percent: Math.min(100, used + 18), resets_at: currentTime + 3 * 86400 },
    ],
    fetched_at: currentTime,
    state: "fresh",
  });
  return { claude: preview(21, 2 * 3600), openai: preview(34, 5 * 3600) };
}

export function applyUsageEvent(current: SnapshotMap, event: ProviderUsageEvent): SnapshotMap {
  const existing = current[event.provider];
  if (existing && existing.fetched_at > event.snapshot.fetched_at) return current;
  return { ...current, [event.provider]: event.snapshot };
}

export function mergeBootstrap(current: SnapshotMap, events: ProviderUsageEvent[]): SnapshotMap {
  return events.reduce(applyUsageEvent, current);
}
