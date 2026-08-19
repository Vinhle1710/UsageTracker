export type SnapshotState = "fresh" | "stale" | "error" | "pending" | "signed-out";
export type Layout = "stacked-compact" | "provider-columns";
export type ThemePreset = "clear" | "frosted" | "blur" | "solid";
export type Provider = "claude" | "openai";
export type ProviderCollapsed = Record<Provider, boolean>;

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

export interface ProviderUsageEvent {
  provider: Provider;
  snapshot: UsageSnapshot;
}

export type SnapshotMap = Partial<Record<Provider, UsageSnapshot>>;

export interface ActiveSources {
  claude: boolean;
  openai: boolean;
}

export interface BootstrapPayload {
  sources: ActiveSources;
  usage: ProviderUsageEvent[];
}

export interface Config {
  locale?: import("./i18n/types").Locale;
  monitorId: string | null;
  corner: string;
  scale: number;
  cardOpacity: number;
  theme: ThemePreset;
  backgroundColor: string;
  layout: Layout;
  alwaysOnTop: boolean;
  offscreenPeek: boolean;
  launchAtStartup: boolean;
  pollIntervalSec: number;
  detectIntervalSec: number;
  showTrayIndicator?: boolean;
  showScreenOverlay?: boolean;
  valueMode?: ValueMode;
  indicatorStyle?: IndicatorStyle;
  enabledMetrics?: MetricId[];
  metricOrder?: MetricId[];
  colorMode?: ColorMode;
  displayColors?: DisplayColors;
  adaptToSystemTheme?: boolean;
  glowEnabled?: boolean;
  autoInitializeSession?: boolean;
  autoInitCostWarningAccepted?: boolean;
  autoInitTaskKind?: "light" | "standard" | "reasoning";
  refreshOnWake?: boolean;
  monitorNetwork?: boolean;
  shortcutPopover?: string | null;
  shortcutRefresh?: string | null;
  shortcutSettings?: string | null;
}

export interface RuntimeStatus { online: boolean; lastRefreshAt: number | null; launchAtLoginRegistered?: boolean; autoInitLastAttemptAt?: number | null; }

export type ValueMode = "used" | "remaining";
export type IndicatorStyle = "battery" | "horizontal-progress" | "percentage" | "provider-icon-bar" | "compact";
export type MetricId = "session" | "weekly" | "api";
export type ColorMode = "multicolor" | "greyscale" | "single-color";
export interface DisplayColors { session: string; weekly: string; api: string; single: string; background: string; text: string; }

import type { ProviderState } from "./state";

export interface OverlayVisibility {
  enabled: boolean;
  providerAvailable: boolean;
  userHidden: boolean;
}

export interface AppSnapshot {
  config: Config;
  sources: ActiveSources;
  providers: ProviderState;
  visibility: OverlayVisibility;
}

export interface MonitorOption {
  id: string;
  label: string;
}

export interface ClaudeAccountInfo {
  organizationUuid: string | null;
  /** Not yet populated by the backend — reserved for once account-email lookup is wired up. */
  email?: string | null;
}
