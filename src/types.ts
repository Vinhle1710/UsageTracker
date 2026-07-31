export type SnapshotState = "fresh" | "stale" | "error";
export type SizeState = "bubble" | "compact" | "square";
export type Layout = "stacked-compact" | "provider-columns";
export type ThemePreset = "clear" | "opaque" | "solid" | "custom";

export interface UsageWindow {
  label: string;
  used_percent: number;
  resets_at: number;
}

export interface UsageSnapshot {
  windows: UsageWindow[];
  fetched_at: number;
  state: SnapshotState;
}

export interface ActiveSources {
  claude: boolean;
  openai: boolean;
}

export interface Config {
  monitorId: string | null;
  corner: string;
  scale: number;
  cardOpacity: number;
  theme: ThemePreset;
  backgroundColor: string;
  layout: Layout;
  alwaysOnTop: boolean;
  offscreenPeek: boolean;
  pollIntervalSec: number;
  detectIntervalSec: number;
}

export interface MonitorOption {
  id: string;
  label: string;
}
