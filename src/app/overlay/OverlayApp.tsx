import { ProviderLayer } from "./ProviderLayer";
import { visibleLayers } from "../../state";
import type { AppSnapshot } from "../../types";
import { initialAppSnapshot } from "../store";

export function OverlayApp({ snapshot = initialAppSnapshot(), onAction }: { snapshot?: AppSnapshot; onAction?: (provider: string, action: string) => void }) {
  const providers = visibleLayers(snapshot.sources);
  return <main id="app" data-window="overlay" className="overlay"><div className="layers">
    {providers.map((provider) => <ProviderLayer key={provider} provider={provider} snapshot={snapshot.providers[provider].snapshot} onAction={(action) => onAction?.(provider, action)} />)}
    {!providers.length && <p className="empty-state">No supported AI client detected.</p>}
    <p className="overlay-status" aria-live="polite" />
  </div></main>;
}
