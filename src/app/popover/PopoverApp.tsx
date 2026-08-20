import type { AppSnapshot } from "../../types";
import { initialAppSnapshot } from "../store";
import { createI18n } from "../../i18n/i18n";
export function PopoverApp({ snapshot = initialAppSnapshot(), detached = false, locale = snapshot.config.locale ?? "en" }: { snapshot?: AppSnapshot; detached?: boolean; locale?: string }) {
 const { t } = createI18n(locale); const providers = Object.values(snapshot.providers).filter((p) => p.snapshot);
 return <section role={detached ? "region" : "dialog"} aria-label={t("popover.title")} data-window="popover"><h1>{t("popover.title")}</h1><div aria-live="polite">{providers.map((p) => p.snapshot?.windows.map((w) => <div key={w.label}>{w.label}: {Math.round(w.used_percent)}%</div>))}</div><button type="button">{detached ? t("action.attach") : t("action.detach")}</button></section>;
}
