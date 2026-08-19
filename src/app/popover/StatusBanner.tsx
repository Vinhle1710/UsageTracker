import type { MessageKey } from "../../i18n/types";
type State = "fresh" | "stale" | "error" | "refreshing" | "none";
export function statusBannerState(input: { refreshing?: boolean; state?: string }): State { if (input.refreshing) return "refreshing"; if (input.state === "error") return "error"; if (input.state === "stale") return "stale"; return "none"; }
export function StatusBanner({ state, t }: { state: Exclude<State, "none">; t: (key: MessageKey) => string }) { const key = `status.${state}` as MessageKey; return <div role="status" aria-live="polite">{t(key)}</div>; }
