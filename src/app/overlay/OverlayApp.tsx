import { ProviderLayer } from "./ProviderLayer";
import { visibleLayers } from "../../state";
import type { AppSnapshot } from "../../types";
import { initialAppSnapshot } from "../store";
import { createI18n } from "../../i18n/i18n";

export function OverlayApp({ snapshot = initialAppSnapshot(), onAction }: { snapshot?: AppSnapshot; onAction?: (provider: string, action: string) => void }) {
  const { t } = createI18n(snapshot.config.locale ?? "en");
  const providers = visibleLayers(snapshot.sources);
  return <main id="app" data-window="overlay" className="overlay"><div className="layers">
    {providers.map((provider) => <ProviderLayer key={provider} provider={provider} snapshot={snapshot.providers[provider].snapshot} onAction={(action) => onAction?.(provider, action)} />)}
    {!providers.length && <p className="empty-state">{t("status.stale")}</p>}
    <p className="overlay-status" aria-live="polite" />
  </div></main>;
}
