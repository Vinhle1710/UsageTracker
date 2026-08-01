import { renderControls, type ControlAction } from "./controls";
import { renderLayer, renderLoadingLayer, updateLayer } from "./layer";
import type { Provider, SnapshotMap, UsageSnapshot } from "../types";

interface ReconcileOptions {
  snapshots: SnapshotMap;
  previousSnapshots: SnapshotMap;
  now: number;
  onAction: (action: ControlAction) => void;
}

const title = (provider: Provider) => provider === "claude" ? "Claude" : "ChatGPT";

function snapshotSignature(snapshot: UsageSnapshot): string {
  return JSON.stringify({
    state: snapshot.state,
    windows: snapshot.windows.map(({ label, used_percent, resets_at }) => ({ label, used_percent, resets_at })),
  });
}

function snapshotAnnouncement(provider: Provider, snapshot: UsageSnapshot): string {
  const name = title(provider);
  if (snapshot.state === "error") return `${name} status: Sign-in required.`;
  if (snapshot.state === "stale") return `${name} status: Usage temporarily unavailable.`;
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
  announceProviderUpdates(content, providers, options.snapshots);
  const controls = content.querySelector<HTMLElement>(".minimize-control") ?? renderControls(options.onAction);
  const wanted = new Set(providers);

  content.querySelectorAll<HTMLElement>(".layer[data-provider]").forEach((layer) => {
    if (!wanted.has(layer.dataset.provider as Provider)) layer.remove();
  });

  const resolved = new Map<Provider, HTMLElement>();

  for (const provider of providers) {
    const snapshot = options.snapshots[provider];
    let layer = content.querySelector<HTMLElement>(`.layer[data-provider="${provider}"]`);
    const canReuse = layer && snapshot && updateLayer(layer, snapshot, options.now);
    if (!layer || (snapshot && !canReuse) || (!snapshot && !layer.classList.contains("layer--loading"))) {
      const replacement = snapshot
        ? renderLayer(title(provider), snapshot, options.now, options.previousSnapshots[provider])
        : renderLoadingLayer(title(provider));
      if (layer) layer.replaceWith(replacement);
      layer = replacement;
    }
    resolved.set(provider, layer);
  }

  providers.forEach((provider, index) => {
    const layer = resolved.get(provider)!;
    const current = content.querySelectorAll<HTMLElement>(".layer[data-provider]")[index];
    if (current !== layer) content.insertBefore(layer, current ?? null);
  });

  content.querySelector(".empty-state")?.remove();
  if (providers.length) {
    const firstLayer = resolved.get(providers[0]);
    if (firstLayer && controls.parentElement !== firstLayer) firstLayer.appendChild(controls);
  } else {
    const empty = document.createElement("p");
    empty.className = "empty-state";
    empty.textContent = "No supported AI client detected.";
    content.appendChild(empty);
  }
}
