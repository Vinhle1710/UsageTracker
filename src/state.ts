import type { ActiveSources, Config, Layout, Provider, ProviderCollapsed, ProviderUsageEvent, SnapshotMap, UsageSnapshot } from "./types";

export interface ProviderRecord {
  active: boolean;
  collapsed: boolean;
  /** One-shot: true for the single render right after this provider transitions from
   *  inactive to active, so the overlay can play the "burst" entrance instead of the
   *  manual-minimize morph. Cleared via {@link clearJustActivated} once consumed. */
  justActivated: boolean;
  snapshot?: UsageSnapshot;
  previousSnapshot?: UsageSnapshot;
}

export type ProviderState = Record<Provider, ProviderRecord>;

type GeometrySettings = Pick<Config, "monitorId" | "corner" | "scale" | "layout" | "theme" | "backgroundColor" | "cardOpacity">;
type ReadoutSettings = Pick<Config, "meterShape">;

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

/** Changing readout shape replaces meter DOM, but uses the usage already held in memory. */
export function readoutShapeChanged(left: ReadoutSettings, right: ReadoutSettings): boolean {
  return (left.meterShape ?? "ring") !== (right.meterShape ?? "ring");
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
    claude: { active: sources.claude, collapsed: false, justActivated: false, snapshot: snapshots.claude },
    openai: { active: sources.openai, collapsed: false, justActivated: false, snapshot: snapshots.openai },
  };
}

export function updateProviderCollapsed(current: ProviderState, provider: Provider, collapsed: boolean): ProviderState {
  return {
    ...current,
    [provider]: { ...current[provider], collapsed },
  };
}

function nextProviderOnSourceChange(existing: ProviderRecord, active: boolean): ProviderRecord {
  const activated = active && !existing.active;
  return {
    ...existing,
    active,
    collapsed: activated ? true : existing.collapsed,
    justActivated: activated,
  };
}

/** A provider that goes from inactive to active starts collapsed (as a bubble) with
 *  `justActivated: true` so the UI can play a burst-entrance animation instead of
 *  immediately showing the full card. */
export function updateProviderSources(current: ProviderState, sources: ActiveSources): ProviderState {
  return {
    claude: nextProviderOnSourceChange(current.claude, sources.claude),
    openai: nextProviderOnSourceChange(current.openai, sources.openai),
  };
}

/** Clears the one-shot `justActivated` flag after a render has consumed it. */
export function clearJustActivated(current: ProviderState): ProviderState {
  if (!current.claude.justActivated && !current.openai.justActivated) return current;
  return {
    claude: current.claude.justActivated ? { ...current.claude, justActivated: false } : current.claude,
    openai: current.openai.justActivated ? { ...current.openai, justActivated: false } : current.openai,
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

export function providerJustActivated(state: ProviderState): ProviderCollapsed {
  return { claude: state.claude.justActivated, openai: state.openai.justActivated };
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
