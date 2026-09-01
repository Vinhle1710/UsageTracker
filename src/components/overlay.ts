import { providerLabel, renderControls, type ControlAction } from "./controls";
import { renderLayer, renderLoadingLayer, updateLayer } from "./layer";
import { edgeForCorner, renderTuckControl } from "./edge-tab";
import {
  reconcileMinimalReadout,
  removeMinimalReadout,
  type MinimalReadoutOptions,
} from "./minimal-readout";
import type { Layout, MeterShape, Provider, ProviderCollapsed, SnapshotMap, UsageSnapshot } from "../types";

export interface ReconcileOptions {
  snapshots: SnapshotMap;
  previousSnapshots: SnapshotMap;
  now: number;
  collapsed?: ProviderCollapsed;
  /** Providers that just transitioned from inactive to active: their first-ever bubble
   *  in this activation plays a burst-entrance animation instead of no animation. */
  burstProviders?: ProviderCollapsed;
  focusProvider?: Provider;
  /** Shape of each card's usage readout. Defaults to the ring the app has always drawn. */
  meterShape?: MeterShape;
  /** Screen corner the overlay is anchored to; the tuck control points at its vertical edge. */
  corner?: string;
  onAction: (action: ControlAction) => void;
}

export interface OverlayLayoutOptions extends ReconcileOptions {
  layout: Layout;
  onGeometryChange: () => Promise<void>;
  motionAdapters?: MinimalReadoutOptions["motionAdapters"];
}

/** Measured into the native window's clip region as an "extra" (see calculateOverlayGeometry).
 *  Without a region of its own the tab is painted by the webview and then clipped away by the
 *  OS, which looks exactly like the control never rendering at all. */
export const TUCK_REGION_SELECTOR = ".tuck-control .usage-tab__button";

const providerOrder: Provider[] = ["claude", "openai"];
const title = providerLabel;

export function reconcileOverlayLayout(
  content: HTMLElement,
  providers: Provider[],
  options: OverlayLayoutOptions,
): void {
  content.classList.add("layers");
  if (options.layout === "minimal") {
    content.classList.add("layers--minimal");
    reconcileMinimalReadout(content, providers, {
      snapshots: options.snapshots,
      now: options.now,
      meterShape: options.meterShape ?? "ring",
      corner: options.corner ?? "bottom-right",
      onAction: options.onAction,
      onGeometryChange: options.onGeometryChange,
      motionAdapters: options.motionAdapters,
    });
    return;
  }

  content.classList.remove("layers--minimal");
  removeMinimalReadout(content);
  reconcileProviderLayers(content, providers, options);
}

function snapshotSignature(snapshot: UsageSnapshot): string {
  return JSON.stringify({
    state: snapshot.state,
    windows: snapshot.windows.map(({ label, used_percent, resets_at }) => ({ label, used_percent, resets_at })),
  });
}

function snapshotAnnouncement(provider: Provider, snapshot: UsageSnapshot): string {
  const name = title(provider);
  if (snapshot.state === "signed-out") return `${name} status: Not signed in.`;
  if (snapshot.state === "error") return `${name} status: Sign-in required.`;
  if (snapshot.state === "pending" && !snapshot.windows.length) return `${name} status: Checking usage.`;
  if (snapshot.state === "stale" && !snapshot.windows.length) return `${name} status: Usage temporarily unavailable.`;
  if (!snapshot.windows.length) return `${name} status: No usage limits reported.`;
  const usage = snapshot.windows
    .map((window) => `${window.label} ${Math.round(window.used_percent)} percent used`)
    .join(", ");
  return `${name} usage updated: ${usage}.`;
}

function ensureStatus(content: HTMLElement): HTMLElement {
  const existing = content.querySelector<HTMLElement>(".overlay-status");
  if (existing) return existing;
  const status = document.createElement("p");
  status.className = "overlay-status";
  status.setAttribute("aria-live", "polite");
  status.setAttribute("aria-atomic", "true");
  content.appendChild(status);
  return status;
}

function announceProviderUpdates(
  content: HTMLElement,
  providers: Provider[],
  snapshots: SnapshotMap,
): void {
  const status = ensureStatus(content);
  const previousProviders = (status.dataset.providers ?? "")
    .split(",")
    .filter((provider): provider is Provider => provider === "claude" || provider === "openai");
  const wanted = new Set(providers);
  const announcements: string[] = [];

  for (const provider of providers) {
    const snapshot = snapshots[provider];
    if (!snapshot) continue;
    const signature = snapshotSignature(snapshot);
    const key = `${provider}Signature`;
    if (status.dataset[key] === signature) continue;
    status.dataset[key] = signature;
    announcements.push(snapshotAnnouncement(provider, snapshot));
  }

  for (const provider of previousProviders) {
    if (wanted.has(provider)) continue;
    delete status.dataset[`${provider}Signature`];
    announcements.push(`${title(provider)} is no longer available.`);
  }

  status.dataset.providers = providers.join(",");
  if (announcements.length) status.textContent = announcements.join(" ");
}

export function reconcileProviderLayers(
  content: HTMLElement,
  providers: Provider[],
  options: ReconcileOptions,
): void {
  content.classList.add("layers");
  const collapsed = options.collapsed ?? { claude: false, openai: false };
  const wanted = new Set(providers);
  const orderedProviders = providerOrder.filter((provider) => wanted.has(provider));
  const expandedProviders = orderedProviders.filter((provider) => !collapsed[provider]);
  const collapsedProviders = orderedProviders.filter((provider) => collapsed[provider]);

  announceProviderUpdates(content, orderedProviders, options.snapshots);

  content.querySelectorAll<HTMLElement>(".layer[data-provider]").forEach((layer) => {
    const provider = layer.dataset.provider as Provider;
    if (!expandedProviders.includes(provider)) layer.remove();
  });

  const resolved = new Map<Provider, HTMLElement>();
  const anchoredEdge = edgeForCorner(options.corner ?? "bottom-right");

  for (const provider of expandedProviders) {
    const snapshot = options.snapshots[provider];
    let layer = content.querySelector<HTMLElement>(`.layer[data-provider="${provider}"]`);
    const canReuse = layer && snapshot && updateLayer(layer, snapshot, options.now, options.onAction, options.meterShape ?? "ring");
    if (!layer || (snapshot && !canReuse) || (!snapshot && !layer.classList.contains("layer--loading"))) {
      const replacement = snapshot
        ? renderLayer(title(provider), snapshot, options.now, options.previousSnapshots[provider], options.onAction, options.meterShape ?? "ring")
        : renderLoadingLayer(title(provider));
      if (layer) layer.replaceWith(replacement);
      layer = replacement;
    }
    const control = layer.querySelector<HTMLElement>(".minimize-control");
    if (!control || control.dataset.edge !== anchoredEdge) {
      control?.remove();
      layer.appendChild(renderControls(provider, options.onAction, options.corner));
    }
    resolved.set(provider, layer);
  }

  expandedProviders.forEach((provider, index) => {
    const layer = resolved.get(provider)!;
    const current = content.querySelectorAll<HTMLElement>(".layer[data-provider]")[index];
    if (current !== layer) content.insertBefore(layer, current ?? null);
  });

  content.querySelector(".empty-state")?.remove();
  if (!orderedProviders.length) {
    const empty = document.createElement("p");
    empty.className = "empty-state";
    empty.textContent = "No supported AI client detected.";
    content.appendChild(empty);
  }

  const burstProviders = options.burstProviders ?? { claude: false, openai: false };
  reconcileBubbles(content, collapsedProviders, burstProviders, options.onAction);
  if (options.focusProvider) focusProvider(content, options.focusProvider, collapsed);
}

function reconcileBubbles(
  content: HTMLElement,
  providers: Provider[],
  burstProviders: ProviderCollapsed,
  onAction: (action: ControlAction) => void,
): void {
  let row = content.querySelector<HTMLElement>(".provider-bubble-row");
  if (!providers.length) {
    row?.remove();
    return;
  }
  if (!row) {
    row = document.createElement("div");
    row.className = "provider-bubble-row";
    content.appendChild(row);
  }

  row.querySelectorAll<HTMLButtonElement>(".provider-bubble").forEach((bubble) => {
    if (!providers.includes(bubble.dataset.provider as Provider)) bubble.remove();
  });

  for (const provider of providers) {
    let bubble = row.querySelector<HTMLButtonElement>(`.provider-bubble[data-provider="${provider}"]`);
    if (!bubble) {
      bubble = document.createElement("button");
      bubble.type = "button";
      bubble.className = burstProviders[provider] ? "provider-bubble provider-bubble--burst" : "provider-bubble";
      bubble.dataset.provider = provider;
      const restore = () => onAction({ action: "restore", provider });
      bubble.addEventListener("click", restore);
      bubble.addEventListener("keydown", (event) => {
        if (event.key !== "Enter" && event.key !== " ") return;
        event.preventDefault();
        restore();
      });
      const logo = document.createElement("img");
      logo.className = "provider-bubble__logo";
      logo.alt = "";
      logo.setAttribute("aria-hidden", "true");
      bubble.appendChild(logo);
    }
    bubble.setAttribute("aria-label", `Expand ${title(provider)} usage`);
    bubble.title = `Expand ${title(provider)} usage`;
    bubble.querySelector<HTMLImageElement>("img")!.src = provider === "claude" ? "/assets/claude-logo.png" : "/assets/chatgpt-logo.png";
    const current = row.querySelectorAll<HTMLButtonElement>(".provider-bubble")[providers.indexOf(provider)];
    if (current !== bubble) row.insertBefore(bubble, current ?? null);
  }

}

/** Mounted on the overlay host, a sibling of the card stack rather than a child of it. Two
 *  reasons, both about the tab never moving: the stack is what the tuck animation slides away,
 *  and the stack's own box changes height every time a card collapses. Pinned to the host, the
 *  tab is anchored to the work-area corner instead — the exact point the edge-tab window is
 *  placed at, so tucking flips the arrow and nothing else. */
export function reconcileTuckControl(
  host: HTMLElement,
  corner: string,
  onTuck?: () => void,
  onOpenSettings?: () => void,
): void {
  const edge = edgeForCorner(corner);
  const existing = host.querySelector<HTMLElement>(".tuck-control");
  if (!onTuck) {
    existing?.remove();
    return;
  }
  // Rebuilt rather than mutated when the corner moves: the glyph is the only thing that differs
  // between the two edges, and re-rendering keeps that mirroring in one place.
  const settingsPresenceChanged = Boolean(existing?.querySelector(".usage-tab__settings-button")) !== Boolean(onOpenSettings);
  if (existing && existing.dataset.edge === edge && !settingsPresenceChanged) return;
  existing?.remove();
  host.appendChild(renderTuckControl(onTuck, corner, onOpenSettings));
}

function focusProvider(content: HTMLElement, provider: Provider, collapsed: ProviderCollapsed): void {
  queueMicrotask(() => {
    const selector = collapsed[provider]
      ? `.provider-bubble[data-provider="${provider}"]`
      : `.layer[data-provider="${provider}"] .minimize-control__button`;
    content.querySelector<HTMLElement>(selector)?.focus();
  });
}
