import { renderControls, type ControlAction } from "./controls";
import { renderLayer, renderLoadingLayer, updateLayer } from "./layer";
import type { Provider, SnapshotMap } from "../types";

interface ReconcileOptions {
  snapshots: SnapshotMap;
  previousSnapshots: SnapshotMap;
  now: number;
  onAction: (action: ControlAction) => void;
}

const title = (provider: Provider) => provider === "claude" ? "Claude" : "ChatGPT";

export function reconcileProviderLayers(
  content: HTMLElement,
  providers: Provider[],
  options: ReconcileOptions,
): void {
  content.classList.add("layers");
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
