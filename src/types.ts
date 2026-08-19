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
  details?: ProviderDetails;
}

export type ProviderDetails = { claude: ClaudeUsageDetails };
export type DataSectionState = "fresh" | "stale" | "unavailable" | "error";
export interface DataSection<T> { value: T | null; fetchedAt: number; state: DataSectionState; errorCode: string | null; }
export interface ClaudeModelLimit { modelKey: string; displayName: string; utilizationPercent: number; resetsAt: number | null; }
export interface Money { minorUnits: number; currency: string; }
export interface ClaudeExtra { spend?: Money; budget?: Money; balance?: Money; }
export interface ClaudeIncident { name: string; status: string; url?: string | null; }
export interface ClaudeServiceStatus { indicator: string; description: string; incidents: ClaudeIncident[]; }
export interface ClaudeUsageDetails { limits: DataSection<ClaudeModelLimit[]>; extra: DataSection<ClaudeExtra>; status?: ClaudeServiceStatus | null; }

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
}

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

export type AccountKind = "claude-ai" | "anthropic-console";
export type CredentialSource = "claude-code" | "secure-store" | "manual";
export type AccountStatus = "signed-in" | "needs-reauthentication" | "unavailable";
export interface AccountSummary {
  id: string;
  kind: AccountKind;
  source: CredentialSource;
  email: string | null;
  status: AccountStatus;
  credentialHint?: string;
}
