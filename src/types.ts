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
export type MoneyMinorUnits = string & { readonly __moneyMinorUnits: unique symbol };
export interface ConsoleMoney { minorUnits: MoneyMinorUnits; currency: string; }
export type UnavailableReason = "noCredential" | "insufficientRole" | "unsupportedBySource" | "providerUnavailable";
export interface CostPeriod { startsAt: string; endsAt: string; timezone: string; }
export interface CostPoint { key: string; label: string; amount: ConsoleMoney; }
export interface ConsoleCostsDashboard {
  period: CostPeriod;
  spend: DataSection<ConsoleMoney>;
  prepaidBalance: DataSection<ConsoleMoney>;
  daily: DataSection<CostPoint[]>;
  byApiKey: DataSection<CostPoint[]>;
  byModel: DataSection<CostPoint[]>;
}
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
  historyRetentionDays?: number;
}

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
