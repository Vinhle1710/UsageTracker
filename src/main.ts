import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { enable } from "@tauri-apps/plugin-autostart";
import { renderControls, type ControlAction } from "./components/controls";
import { renderLayer, renderLoadingLayer, updateLayer } from "./components/layer";
import { renderSettings } from "./components/settings";
import { formatReset } from "./format";
import { applyUsageEvent, geometryChanged, initialSnapshots, sameSources, visibleLayers } from "./state";
import type { ActiveSources, Config, MonitorOption, Provider, ProviderUsageEvent, SnapshotMap, UsageSnapshot } from "./types";
import "./styles/app.css";

const now = () => Math.floor(Date.now() / 1000);
const app = document.querySelector<HTMLElement>("#app")!;
const nativeWindow = (() => {
  try {
    return getCurrentWindow();
  } catch {
    return null;
  }
})();
const isSettingsWindow = nativeWindow?.label === "settings";
app.dataset.window = isSettingsWindow ? "settings" : "overlay";

let config: Config = {
  monitorId: null,
  corner: "bottom-right",
  scale: 1,
  cardOpacity: 0.98,
  theme: "acrylic",
  backgroundColor: "#07101f",
  layout: "stacked-compact",
  alwaysOnTop: true,
  offscreenPeek: false,
  pollIntervalSec: 60,
  detectIntervalSec: 5,
};
let minimized = false;
const previewMode = nativeWindow === null;
let sources: ActiveSources = previewMode ? { claude: true, openai: true } : { claude: false, openai: false };
let snapshots: SnapshotMap = initialSnapshots(previewMode, now());
let previousSnapshots: SnapshotMap = {};
let monitors: MonitorOption[] = [];
const handledResets = new Set<string>();

function providerTitle(provider: Provider): string {
  return provider === "claude" ? "Claude" : "ChatGPT";
}

function geometryRequest() {
  return {
    corner: config.corner,
    preferred: config.monitorId,
    layout: config.layout,
    scale: config.scale,
    providerCount: visibleLayers(sources).length,
    minimized,
  };
}

async function applyGeometry(): Promise<void> {
  if (isSettingsWindow) return;
  await invoke("apply_overlay_geometry", { request: geometryRequest() }).catch(() => undefined);
}

function updateCountdowns(): void {
  if (isSettingsWindow || minimized) return;
  const currentNow = now();
  app.querySelectorAll<HTMLElement>(".window-card__reset").forEach((reset) => {
    const label = reset.dataset.label;
    const resetsAt = Number(reset.dataset.resetsAt);
    if (!label || !Number.isFinite(resetsAt)) return;
    reset.textContent = formatReset(label, resetsAt, currentNow);
    const meter = reset.closest<HTMLElement>(".window-card")?.querySelector<HTMLElement>(".meter");
    if (!meter || resetsAt > currentNow) return;
    const provider = meter.dataset.provider ?? "unknown";
    const key = `${provider}:${label}:${resetsAt}`;
    if (handledResets.has(key)) return;
    handledResets.add(key);
    meter.classList.add("meter--resetting");
    window.setTimeout(() => meter.classList.remove("meter--resetting"), 850);
  });
}

function renderMain(): void {
  const active = visibleLayers(sources);
  app.dataset.layout = config.layout;
  app.dataset.minimized = String(minimized);
  app.style.setProperty("--ui-scale", String(config.scale));
  app.style.setProperty("--card-opacity", `${Math.round(config.cardOpacity * 100)}%`);
  app.style.setProperty("--card-background", config.backgroundColor);
  app.dataset.theme = config.theme;
  app.innerHTML = "";

  if (minimized) {
    const restore = document.createElement("button");
    restore.type = "button";
    restore.className = "minimized-pill";
    restore.setAttribute("aria-label", "Restore usage overlay");
    restore.innerHTML = "<span></span><span></span>";
    restore.addEventListener("click", () => {
      minimized = false;
      renderMain();
      void applyGeometry();
    });
    app.appendChild(restore);
    return;
  }

  const content = document.createElement("div");
  content.className = "layers";
  let firstLayer: HTMLElement | null = null;
  for (const provider of active) {
    const snapshot = snapshots[provider];
    const layer = snapshot
      ? renderLayer(providerTitle(provider), snapshot, now(), previousSnapshots[provider])
      : renderLoadingLayer(providerTitle(provider));
    firstLayer ??= layer;
    content.appendChild(layer);
  }
  if (!active.length) {
    const empty = document.createElement("p");
    empty.className = "empty-state";
    empty.textContent = "No supported AI client detected.";
    content.appendChild(empty);
  }
  if (firstLayer) firstLayer.appendChild(renderControls(handleAction));
  app.appendChild(content);
  updateCountdowns();
}

function refreshProvider(provider: Provider, snapshot: UsageSnapshot): void {
  previousSnapshots[provider] = snapshots[provider];
  snapshots = applyUsageEvent(snapshots, { provider, snapshot });
  const layer = app.querySelector<HTMLElement>(`.layer[data-provider="${provider}"]`);
  if (!minimized && layer && updateLayer(layer, snapshot, now())) {
    updateCountdowns();
    return;
  }
  renderMain();
}

function handleAction(action: ControlAction): void {
  if (action !== "minimize") return;
  minimized = true;
  renderMain();
  void applyGeometry();
}

function renderSettingsWindow(): void {
  app.innerHTML = "";
  app.appendChild(renderSettings(config, monitors, {
    onChange: (next) => {
      config = next;
      void invoke("set_config", { cfg: config }).catch(() => undefined);
    },
    onClose: () => {
      if (nativeWindow) {
        void invoke("close_settings").catch(() => nativeWindow.hide().catch(() => nativeWindow.close()));
      } else {
        app.replaceChildren();
        app.hidden = true;
      }
    },
    onDrag: () => void nativeWindow?.startDragging(),
  }));
}

async function connectSettings(): Promise<void> {
  try {
    config = await invoke<Config>("get_config");
    monitors = await invoke<MonitorOption[]>("list_monitors");
  } catch {
    monitors = [
      { id: "primary", label: "Primary screen" },
      { id: "secondary", label: "Secondary screen" },
    ];
  }
  renderSettingsWindow();
}

async function connectMain(): Promise<void> {
  try {
    config = await invoke<Config>("get_config");
    await enable();
    await listen<ActiveSources>("sources-changed", (event) => {
      const changed = !sameSources(sources, event.payload);
      sources = event.payload;
      if (changed) {
        renderMain();
        void applyGeometry();
      }
    });
    await listen<ProviderUsageEvent>("usage-changed", (event) => {
      refreshProvider(event.payload.provider, event.payload.snapshot);
    });
    await listen<Config>("config-changed", (event) => {
      const changed = geometryChanged(config, event.payload);
      config = event.payload;
      renderMain();
      if (changed) void applyGeometry();
    });
  } catch {
    // The browser preview keeps demo data visible when no Tauri runtime exists.
  }
  renderMain();
  void applyGeometry();
  window.setInterval(updateCountdowns, 1000);
}

if (isSettingsWindow) {
  void connectSettings();
} else {
  void connectMain();
}
