import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { ControlAction } from "./components/controls";
import { progressOffset } from "./components/layer";
import { reconcileProviderLayers } from "./components/overlay";
import { renderSettings } from "./components/settings";
import { formatReset, getFunPlaceholder } from "./format";
import { GeometryRequestScheduler } from "./geometry-scheduler";
import { calculateOverlayGeometry } from "./geometry";
import { createProviderState, geometryChanged, initialSnapshots, providerPreviousSnapshots, providerSnapshots, sameSources, updateProviderCollapsed, updateProviderSources, updateProviderUsage, visibleLayers } from "./state";
import type { ActiveSources, BootstrapPayload, Config, MonitorOption, Provider, ProviderCollapsed, ProviderUsageEvent, UsageSnapshot } from "./types";
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
  theme: "frosted",
  backgroundColor: "#07101f",
  layout: "stacked-compact",
  alwaysOnTop: true,
  offscreenPeek: false,
  pollIntervalSec: 60,
  detectIntervalSec: 5,
};
const previewMode = nativeWindow === null;
const initialSources: ActiveSources = previewMode ? { claude: true, openai: true } : { claude: false, openai: false };
let providerState = createProviderState(initialSources, initialSnapshots(previewMode, now()));
let monitors: MonitorOption[] = [];
const handledResets = new Set<string>();

function geometryRequest() {
  const rootRect = app.getBoundingClientRect();
  const cards = Array.from(app.querySelectorAll<HTMLElement>(".layer[data-provider]"))
    .map((layer) => layer.getBoundingClientRect());
  const bubbleRow = app.querySelector<HTMLElement>(".provider-bubble-row")?.getBoundingClientRect();
  const bubbles = Array.from(app.querySelectorAll<HTMLElement>(".provider-bubble"))
    .map((bubble) => bubble.getBoundingClientRect());
  const activeProviders = visibleLayers(activeSources());
  const expandedProviders = activeProviders.filter((provider) => !providerState[provider].collapsed);
  const bubbleCount = activeProviders.length - expandedProviders.length;
  const measured = calculateOverlayGeometry(
    rootRect,
    cards,
    bubbles,
    8 * config.scale,
    14 * config.scale,
    24 * config.scale,
    bubbleRow,
  );
  return {
    corner: config.corner,
    preferred: config.monitorId,
    layout: config.layout,
    scale: config.scale,
    expandedProviderCount: expandedProviders.length,
    bubbleCount,
    theme: config.theme,
    backgroundColor: config.backgroundColor,
    cardOpacity: config.cardOpacity,
    regions: measured.regions,
    contentWidth: measured.contentWidth,
    contentHeight: measured.contentHeight,
  };
}

function activeSources(): ActiveSources {
  return { claude: providerState.claude.active, openai: providerState.openai.active };
}

const geometryScheduler = new GeometryRequestScheduler<ReturnType<typeof geometryRequest>>(
  (request) => invoke("apply_overlay_geometry", { request }),
);

async function applyGeometry(): Promise<void> {
  if (isSettingsWindow) return;
  await geometryScheduler.enqueue(geometryRequest());
}

function updateCountdowns(): void {
  if (isSettingsWindow) return;
  const currentNow = now();
  app.querySelectorAll<HTMLElement>(".window-card__reset").forEach((reset) => {
    const label = reset.dataset.label;
    const resetsAt = Number(reset.dataset.resetsAt);
    if (!label || !Number.isFinite(resetsAt)) return;
    const meter = reset.closest<HTMLElement>(".window-card")?.querySelector<HTMLElement>(".meter");

    if (reset.dataset.cachedMessage) {
      reset.textContent = reset.dataset.cachedMessage;
      return;
    }

    if (resetsAt > currentNow) {
      reset.textContent = formatReset(label, resetsAt, currentNow);
      return;
    }

    // The countdown just reached (or already passed) zero: apply the optimistic
    // reset once per (provider, label, resetsAt) triple, same key the pulse animation uses.
    const provider = meter?.dataset.provider ?? "unknown";
    const key = `${provider}:${label}:${resetsAt}`;
    const funMessage = getFunPlaceholder();
    reset.dataset.cachedMessage = funMessage;
    reset.textContent = funMessage;

    if (!meter || handledResets.has(key)) return;
    handledResets.add(key);
    meter.style.setProperty("--progress-offset", progressOffset(0));
    const value = meter.querySelector<HTMLElement>(".meter__value");
    if (value) value.textContent = "0%";
    meter.setAttribute("aria-valuenow", "0");
    meter.setAttribute("aria-valuetext", `0 percent used, ${funMessage}`);
    meter.classList.add("meter--resetting");
    window.setTimeout(() => meter.classList.remove("meter--resetting"), 850);
  });
}

function applyAppearance(): void {
  app.dataset.layout = config.layout;
  app.dataset.corner = config.corner;
  const activeProviders = visibleLayers(activeSources());
  app.dataset.expandedCount = String(activeProviders.filter((provider) => !providerState[provider].collapsed).length);
  app.dataset.bubbleCount = String(activeProviders.filter((provider) => providerState[provider].collapsed).length);
  app.dataset.collapsedProviders = visibleLayers(activeSources())
    .filter((provider) => providerState[provider].collapsed)
    .join(",");
  app.style.setProperty("--ui-scale", String(config.scale));
  app.style.setProperty("--card-opacity", `${Math.round(config.cardOpacity * 100)}%`);
  app.style.setProperty("--frosted-opacity", `${Math.round(config.cardOpacity * 72)}%`);
  app.style.setProperty("--blur-opacity", `${Math.round(config.cardOpacity * 58)}%`);
  app.style.setProperty("--card-background", config.backgroundColor);
  app.dataset.theme = config.theme;
}

function renderMain(focusProvider?: Provider): void {
  applyAppearance();

  let content = app.querySelector<HTMLElement>(".layers");
  if (!content) {
    content = document.createElement("div");
    app.appendChild(content);
  }
  reconcileProviderLayers(content, visibleLayers(activeSources()), {
    snapshots: providerSnapshots(providerState),
    previousSnapshots: providerPreviousSnapshots(providerState),
    now: now(),
    collapsed: {
      claude: providerState.claude.collapsed,
      openai: providerState.openai.collapsed,
    } satisfies ProviderCollapsed,
    focusProvider,
    onAction: handleAction,
  });
  updateCountdowns();
}

function refreshProvider(provider: Provider, snapshot: UsageSnapshot): void {
  providerState = updateProviderUsage(providerState, { provider, snapshot });
  renderMain();
  void applyGeometry();
}

function handleAction(action: ControlAction): void {
  providerState = updateProviderCollapsed(providerState, action.provider, action.action === "minimize");
  renderMain(action.provider);
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
        void invoke("close_settings").catch(() => undefined);
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
    await listen<ActiveSources>("sources-changed", (event) => {
      const changed = !sameSources(activeSources(), event.payload);
      providerState = updateProviderSources(providerState, event.payload);
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
      applyAppearance();
      if (changed) void applyGeometry();
    });
    const bootstrap = await invoke<BootstrapPayload>("get_bootstrap");
    providerState = updateProviderSources(providerState, bootstrap.sources);
    for (const event of bootstrap.usage) providerState = updateProviderUsage(providerState, event);
    renderMain();
    await applyGeometry();
    await invoke("mark_overlay_ready");
    window.setInterval(updateCountdowns, 1000);
    return;
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
