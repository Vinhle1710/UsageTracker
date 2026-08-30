import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { createRoot } from "react-dom/client";
import { createElement } from "react";
import { HistoryApp } from "./history/HistoryApp";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { ControlAction } from "./components/controls";
import { progressOffset } from "./components/layer";
import { reconcileProviderLayers, reconcileTuckControl, TUCK_REGION_SELECTOR } from "./components/overlay";
import { edgeForCorner, renderEdgeTab, TUCK_EASING, TUCK_MOTION_MS, tuckKeyframes } from "./components/edge-tab";
import { renderSettings } from "./components/settings";
import { formatReset, getFunPlaceholder } from "./format";
import { GeometryRequestScheduler } from "./geometry-scheduler";
import { calculateOverlayGeometry, OVERLAY_HEADROOM } from "./geometry";
import { crossfadeKeyframes, flipDelta, FLIP_EASING, flipKeyframes, isNegligibleFlipDelta, MORPH_DURATION_MS, MORPH_EASING, morphKeyframes, prefersReducedMotion, supportsElementAnimate, toAnchoredRect, type AnchoredRect } from "./morph";
import { createProviderState, clearJustActivated, geometryChanged, initialSnapshots, providerJustActivated, providerPreviousSnapshots, providerSnapshots, readoutShapeChanged, sameSources, updateProviderCollapsed, updateProviderSources, updateProviderUsage, visibleLayers } from "./state";
import { generateConfetti, spawnCelebration } from "./celebration";
import { enhanceSurface } from "./motion/surface-motion";
import type { ActiveSources, BootstrapPayload, ClaudeAccountInfo, Config, MonitorOption, Provider, ProviderCollapsed, ProviderUsageEvent, UsageSnapshot } from "./types";
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
const isHistoryWindow = nativeWindow?.label === "history";
const isEdgeTabWindow = nativeWindow?.label === "edge-tab";
app.dataset.window = isSettingsWindow ? "settings" : isHistoryWindow ? "history" : isEdgeTabWindow ? "edge-tab" : "overlay";

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
  launchAtStartup: true,
  pollIntervalSec: 60,
  detectIntervalSec: 5,
  showTrayIndicator: true,
  showScreenOverlay: true,
};
const previewMode = nativeWindow === null;
const initialSources: ActiveSources = previewMode ? { claude: true, openai: true } : { claude: false, openai: false };
let providerState = createProviderState(initialSources, initialSnapshots(previewMode, now()));
let monitors: MonitorOption[] = [];
let claudeAccount: ClaudeAccountInfo | null = null;
/** True while the overlay is parked at the edge tab. Gates geometry: see applyGeometry. */
let tucked = false;
let cleanupSettingsMotion: (() => void) | null = null;
const handledResets = new Set<string>();

function geometryRequest() {
  const rootRect = app.getBoundingClientRect();
  const cards = Array.from(app.querySelectorAll<HTMLElement>(".layer[data-provider]"))
    .map((layer) => layer.getBoundingClientRect());
  const bubbleRow = app.querySelector<HTMLElement>(".provider-bubble-row")?.getBoundingClientRect();
  const bubbles = Array.from(app.querySelectorAll<HTMLElement>(".provider-bubble"))
    .map((bubble) => bubble.getBoundingClientRect());
  const extras = Array.from(app.querySelectorAll<HTMLElement>(TUCK_REGION_SELECTOR))
    .map((extra) => extra.getBoundingClientRect());
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
    OVERLAY_HEADROOM * config.scale,
    extras,
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
    // Only meaningful once real regions were measured: with none, the window falls back to a
    // base size that carries no headroom, so offsetting its position would just misplace it.
    headroom: measured.regions.length ? OVERLAY_HEADROOM * config.scale : 0,
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
  // While tucked the window is hidden and the stack is parked mid-transform, so its rects are
  // the offset ones — measuring them would hand the OS a region 30px off from where the cards
  // actually come back. untuckIn re-measures once the stack is at rest again.
  if (tucked) return;
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

    // Scoped to this one window-card, never the whole app: only the limit that actually
    // reset should celebrate.
    const card = meter.closest<HTMLElement>(".window-card");
    if (card && !prefersReducedMotion()) spawnCelebration(card, generateConfetti(10));
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
    burstProviders: providerJustActivated(providerState),
    focusProvider,
    meterShape: config.meterShape ?? "ring",
    corner: config.corner,
    onAction: handleAction,
  });
  // A sibling of the stack, not a child: the stack is what the tuck animation slides away, and
  // the tab has to stay put through it. Omitted in the browser preview, where there is no
  // native window to hide and the control would do nothing.
  reconcileTuckControl(app, config.corner, nativeWindow ? () => void tuckAway() : undefined);
  updateCountdowns();
}

function refreshProvider(provider: Provider, snapshot: UsageSnapshot): void {
  providerState = updateProviderUsage(providerState, { provider, snapshot });
  renderMain();
  void applyGeometry();
}

const morphingProviders = new Set<Provider>();

function handleAction(action: ControlAction): void {
  if (action.action === "open-settings") {
    void invoke("open_settings_window", { page: action.page ?? null }).catch(() => undefined);
    return;
  }
  if (action.action === "open-cli") {
    void invoke("open_cli_terminal", { provider: action.provider }).catch(() => undefined);
    return;
  }
  if (morphingProviders.has(action.provider)) return;
  if (action.action === "minimize") void morphMinimize(action.provider);
  else void morphRestore(action.provider);
}

function instantToggle(provider: Provider, collapsed: boolean): void {
  providerState = updateProviderCollapsed(providerState, provider, collapsed);
  renderMain(provider);
  void applyGeometry();
}

interface SurfaceStyle {
  background: string;
  backdropFilter: string;
  border: string;
  boxShadow: string;
}

/** Sampled from the real element's *computed* style, so the ghost's background/blur/border
 *  are pixel-exact for whatever theme is active, instead of an approximated flat color. */
function captureSurface(el: HTMLElement): SurfaceStyle {
  const computed = getComputedStyle(el);
  return {
    background: computed.background,
    backdropFilter: computed.backdropFilter,
    border: computed.border,
    boxShadow: computed.boxShadow,
  };
}

function applySurface(el: HTMLElement, surface: SurfaceStyle): void {
  el.style.background = surface.background;
  el.style.backdropFilter = surface.backdropFilter;
  el.style.setProperty("-webkit-backdrop-filter", surface.backdropFilter);
  el.style.border = surface.border;
  el.style.boxShadow = surface.boxShadow;
}

/** Every `.layer`/`.provider-bubble` currently on screen, keyed so a sibling that persists
 *  across a render (same kind, same provider) can be matched before vs. after. */
function snapshotLayerAndBubbleRects(container: HTMLElement): Map<string, DOMRect> {
  const rects = new Map<string, DOMRect>();
  container.querySelectorAll<HTMLElement>(".layer[data-provider], .provider-bubble[data-provider]").forEach((el) => {
    const kind = el.classList.contains("layer") ? "layer" : "bubble";
    rects.set(`${kind}:${el.dataset.provider}`, el.getBoundingClientRect());
  });
  return rects;
}

/** Slides every persisted sibling from its pre-render position to its post-render one.
 *  Re-rendering flips layout-mode CSS attributes (padding, bubble-row position) instantly for
 *  the whole app, which otherwise snaps every *other* card/bubble the instant one provider's
 *  collapsed state changes — this is what made an already-collapsed bubble seem to vanish and
 *  made a still-expanded sibling card jump. An element that only just appeared or disappeared
 *  (the one actually being morphed) has no entry on one side of `before`/`after` and is
 *  skipped automatically. */
function flipSiblings(container: HTMLElement, before: Map<string, DOMRect>): void {
  container.querySelectorAll<HTMLElement>(".layer[data-provider], .provider-bubble[data-provider]").forEach((el) => {
    const kind = el.classList.contains("layer") ? "layer" : "bubble";
    const key = `${kind}:${el.dataset.provider}`;
    const previous = before.get(key);
    if (!previous) return;
    const delta = flipDelta(previous, el.getBoundingClientRect());
    if (isNegligibleFlipDelta(delta)) return;
    el.animate(flipKeyframes(delta), { duration: MORPH_DURATION_MS, easing: FLIP_EASING });
  });
}

interface LogoSurface {
  rect: AnchoredRect;
  src: string;
  filter: string;
}

function viewportSize(): { width: number; height: number } {
  return { width: window.innerWidth, height: window.innerHeight };
}

/** Positions an element by whichever pair of sides `rect` is anchored to (left/top or
 *  right/bottom, etc.), explicitly clearing the other pair — required because a fixed-position
 *  element defaults `right`/`bottom` to `auto`, which does nothing on its own, but a stale
 *  `left` from a previous anchor would otherwise keep competing with a freshly-set `right`. */
function applyAnchoredRect(el: HTMLElement, rect: AnchoredRect): void {
  el.style.left = rect.xSide === "left" ? `${rect.x}px` : "auto";
  el.style.right = rect.xSide === "right" ? `${rect.x}px` : "auto";
  el.style.top = rect.ySide === "top" ? `${rect.y}px` : "auto";
  el.style.bottom = rect.ySide === "bottom" ? `${rect.y}px` : "auto";
  el.style.width = `${rect.width}px`;
  el.style.height = `${rect.height}px`;
}

/** The native window is clipped to the card/bubble shapes, so a ghost crossing the transparent
 *  gap between two cards — or sweeping past the old card's rounded corner — gets cropped by the
 *  OS, not by CSS. Opens the region up to the *entire* window for the morph's duration, headroom
 *  included: a tighter box derived from the animation's endpoints kept clipping the overshoot
 *  that springy easings and the burst ring travel through, and the region costs nothing to
 *  widen because everything outside the cards is transparent anyway. */
async function beginMorphRegion(): Promise<void> {
  const viewport = viewportSize();
  await invoke("set_morph_region", {
    region: { x: 0, y: 0, width: viewport.width, height: viewport.height, radius: 0 },
  }).catch(() => undefined);
}

/** Restores the exact card/bubble shapes. Callers run this *after* any geometry update, so the
 *  shapes it restores are the final ones and the window never flashes an outdated silhouette. */
async function endMorphRegion(): Promise<void> {
  await invoke("set_morph_region", { region: null }).catch(() => undefined);
}

/** Plays one leg of the tuck transition on `target`, resolving when the motion is done. Falls
 *  through instantly when the platform cannot animate or the user asked for reduced motion, so
 *  the caller's sequencing is identical either way. */
async function playTuckMotion(target: HTMLElement | null, direction: "out" | "in"): Promise<void> {
  if (!target || !supportsElementAnimate() || prefersReducedMotion()) return;
  const animation = target.animate(tuckKeyframes(edgeForCorner(config.corner), direction), {
    duration: TUCK_MOTION_MS,
    easing: TUCK_EASING,
    fill: "forwards",
  });
  await animation.finished.catch(() => undefined);
  // Dropped once the frame has landed: a lingering forwards fill would hold the stack offset
  // under the next real render, which measures geometry from these very rects.
  if (direction === "in") animation.cancel();
}

/** Slides the stack into its anchored edge, then lets the native swap happen behind the motion.
 *  The window is clipped to the card shapes, so the travel has to run with the morph region
 *  open or the OS crops the cards the moment they leave their own silhouette. */
async function tuckAway(): Promise<void> {
  await beginMorphRegion();
  await playTuckMotion(app.querySelector<HTMLElement>(".layers"), "out");
  // The forwards fill is deliberately left in place: the stack stays parked off-edge and
  // transparent for as long as it is tucked, so the restore has nothing to flash before its
  // own entrance gets a frame. `tucked` is what keeps geometry off those parked rects.
  tucked = true;
  await invoke("set_overlay_tucked", { tucked: true }).catch(() => undefined);
  await endMorphRegion();
}

/** The mirror, driven by the `overlay-tucked` event rather than a click: the restore is asked
 *  for from the tab's own window, so this one only ever learns about it second-hand. */
async function untuckIn(): Promise<void> {
  await beginMorphRegion();
  await playTuckMotion(app.querySelector<HTMLElement>(".layers"), "in");
  tucked = false;
  await applyGeometry();
  await endMorphRegion();
}

/** Sampled from the real logo `<img>`'s own rect and computed `filter` (the per-theme
 *  ChatGPT recolor is applied via CSS filter, not a different asset), so the logo ghost
 *  matches exactly regardless of provider or theme. */
function captureLogo(container: HTMLElement): LogoSurface | null {
  const img = container.querySelector<HTMLImageElement>(".provider-mark img, .provider-bubble__logo");
  if (!img) return null;
  const computed = getComputedStyle(img);
  return { rect: toAnchoredRect(img.getBoundingClientRect(), 0, config.corner, viewportSize()), src: img.src, filter: computed.filter };
}

/** Animates a ghost shape directly from `fromRect` to `toRect` — an exact shape-to-shape blend
 *  of the box's own geometry, with no independent-axis transform scaling (which both distorted
 *  the box into an ellipse mid-flight and visually stretched the border thicker or thinner) —
 *  while cross-fading the real destination element in, so the shape settles before the content
 *  swap becomes visible. Both rects are anchored to whichever corner the overlay itself is
 *  configured to (see toAnchoredRect): the ghost is positioned via `right`/`bottom` for a
 *  right/bottom-anchored overlay instead of `left`/`top`, so its target position stays correct
 *  even if the native window resizes mid-animation, instead of relying on a snapshot of
 *  left/top pixels from a window size that may no longer be current. The provider logo travels
 *  as its own independent element — visible for the entire transition, handed off to the real
 *  logo only in the last quarter once `reveal` is already fading in — instead of disappearing
 *  along with the rest of the source's content. */
async function runMorph(fromRect: AnchoredRect, toRect: AnchoredRect, surface: SurfaceStyle, reveal: HTMLElement, logoFrom: LogoSurface | null, logoTo: LogoSurface | null): Promise<void> {
  const ghost = document.createElement("div");
  ghost.className = "morph-ghost";
  applyAnchoredRect(ghost, fromRect);
  ghost.style.borderRadius = `${fromRect.borderRadius}px`;
  applySurface(ghost, surface);
  document.body.appendChild(ghost);
  reveal.style.opacity = "0";

  let logoGhost: HTMLImageElement | null = null;
  if (logoFrom && logoTo) {
    logoGhost = document.createElement("img");
    logoGhost.className = "morph-logo-ghost";
    logoGhost.src = logoFrom.src;
    logoGhost.alt = "";
    applyAnchoredRect(logoGhost, logoFrom.rect);
    logoGhost.style.filter = logoFrom.filter;
    document.body.appendChild(logoGhost);
    logoGhost.animate(morphKeyframes(logoFrom.rect, logoTo.rect), { duration: MORPH_DURATION_MS, easing: MORPH_EASING, fill: "forwards" });
    logoGhost.animate(crossfadeKeyframes("out"), { duration: MORPH_DURATION_MS, fill: "forwards" });
  }

  const shape = ghost.animate(morphKeyframes(fromRect, toRect), { duration: MORPH_DURATION_MS, easing: MORPH_EASING, fill: "forwards" });
  ghost.animate(crossfadeKeyframes("out"), { duration: MORPH_DURATION_MS, fill: "forwards" });
  reveal.animate(crossfadeKeyframes("in"), { duration: MORPH_DURATION_MS, fill: "forwards" });

  // Race against a timeout, not just `shape.finished` directly: a WAAPI animation promise
  // that never settles (a throttled/backgrounded window, a browser quirk) would otherwise
  // strand the ghost on screen forever and leave `reveal` invisible — a real, observed failure
  // mode, not a hypothetical one.
  await Promise.race([
    shape.finished.catch(() => undefined),
    new Promise<void>((resolve) => window.setTimeout(resolve, MORPH_DURATION_MS + 400)),
  ]);
  ghost.remove();
  logoGhost?.remove();
  reveal.style.opacity = "";
}

async function morphMinimize(provider: Provider): Promise<void> {
  const layer = app.querySelector<HTMLElement>(`.layer[data-provider="${provider}"]`);
  if (!layer || prefersReducedMotion() || !supportsElementAnimate()) {
    instantToggle(provider, true);
    return;
  }
  morphingProviders.add(provider);
  try {
    const fromRect = toAnchoredRect(layer.getBoundingClientRect(), 14, config.corner, viewportSize());
    const surface = captureSurface(layer);
    const logoFrom = captureLogo(layer);
    const before = snapshotLayerAndBubbleRects(app);

    providerState = updateProviderCollapsed(providerState, provider, true);
    renderMain(provider);
    flipSiblings(app, before);

    const bubble = app.querySelector<HTMLElement>(`.provider-bubble[data-provider="${provider}"]`);
    if (!bubble) {
      void applyGeometry();
      return;
    }
    // Deliberately deferred: shrinking the native window before the shape settles would
    // clip the ghost, since the window still needs to be large enough to hold it. Anchoring the
    // rects to the overlay's actual corner (instead of left/top) keeps the target correct even
    // though the window is still its old size while this measurement happens.
    const toRect = toAnchoredRect(bubble.getBoundingClientRect(), 24, config.corner, viewportSize());
    await beginMorphRegion();
    await runMorph(fromRect, toRect, surface, bubble, logoFrom, captureLogo(bubble));
    // Order matters: applyGeometry writes the final shapes, so the restore below re-applies
    // those rather than the stale pre-minimize card silhouette.
    await applyGeometry();
    await endMorphRegion();
  } finally {
    morphingProviders.delete(provider);
  }
}

async function morphRestore(provider: Provider): Promise<void> {
  const bubble = app.querySelector<HTMLElement>(`.provider-bubble[data-provider="${provider}"]`);
  if (!bubble || prefersReducedMotion() || !supportsElementAnimate()) {
    instantToggle(provider, false);
    return;
  }
  morphingProviders.add(provider);
  try {
    const fromRect = toAnchoredRect(bubble.getBoundingClientRect(), 24, config.corner, viewportSize());
    const surface = captureSurface(bubble);
    const logoFrom = captureLogo(bubble);
    const before = snapshotLayerAndBubbleRects(app);

    providerState = updateProviderCollapsed(providerState, provider, false);
    renderMain(provider);

    // Grow the window to the target size first: unlike minimize, growing content beyond the
    // current (smaller) window would otherwise be clipped by #app's overflow:hidden.
    await applyGeometry();
    await new Promise<void>((resolve) => requestAnimationFrame(() => resolve()));
    flipSiblings(app, before);

    const layer = app.querySelector<HTMLElement>(`.layer[data-provider="${provider}"]`);
    if (!layer) return;
    const toRect = toAnchoredRect(layer.getBoundingClientRect(), 14, config.corner, viewportSize());
    await beginMorphRegion();
    await runMorph(fromRect, toRect, surface, layer, logoFrom, captureLogo(layer));
    // applyGeometry already ran above (the window had to grow first), so the cached shapes are
    // the final ones and restoring them here is exactly right.
    await endMorphRegion();
  } finally {
    morphingProviders.delete(provider);
  }
}

let hasSessionKey = false;

function renderSettingsWindow(initialPage = "general"): void {
  cleanupSettingsMotion?.();
  cleanupSettingsMotion = null;
  app.innerHTML = "";
  const settingsSurface = renderSettings(config, monitors, {
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
    onResetNotificationHistory: () => invoke("reset_notification_history"),
    onRefreshNow: async () => { await invoke<void>("refresh_usage").catch(() => undefined); },
    onClaudeSignIn: () => invoke<string>("start_claude_login").catch(() => null),
    onClaudeSignInSubmit: async (code) => {
      try {
        await invoke("finish_claude_login", { pasted: code });
      } catch (error) {
        return { ok: false, error: typeof error === "string" ? error : "Sign-in failed." };
      }
      const account = await invoke<ClaudeAccountInfo | null>("get_claude_account").catch(() => null);
      return account ? { ok: true, account } : { ok: false, error: "Signed in, but the account could not be read." };
    },
    onClaudeLogout: async () => {
      await invoke("claude_logout").catch(() => undefined);
    },
    onHistory: () => { void invoke("open_history_window").catch(() => undefined); },
    onSaveSessionKey: async (sessionKey) => {
      await invoke("save_claude_session_key", { sessionKey });
      hasSessionKey = true;
    },
    onClearSessionKey: async () => {
      await invoke("clear_claude_session_key").catch(() => undefined);
      hasSessionKey = false;
    },
  }, claudeAccount, initialPage, hasSessionKey);
  app.appendChild(settingsSurface);
  cleanupSettingsMotion = enhanceSurface(settingsSurface);
}

async function loadSettingsData(): Promise<void> {
  try {
    config = await invoke<Config>("get_config");
    monitors = await invoke<MonitorOption[]>("list_monitors");
    claudeAccount = await invoke<ClaudeAccountInfo | null>("get_claude_account").catch(() => null);
    hasSessionKey = await invoke<boolean>("has_claude_session_key").catch(() => false);
  } catch {
    monitors = [
      { id: "primary", label: "Primary screen" },
      { id: "secondary", label: "Secondary screen" },
    ];
  }
}

async function connectSettings(): Promise<void> {
  await loadSettingsData();
  renderSettingsWindow();
  const initialRuntimeStatus = await invoke<{ online: boolean; lastRefreshAt: number | null; launchAtLoginRegistered?: boolean; autoInitLastAttemptAt?: number | null }>("get_runtime_status").catch(() => ({ online: true, lastRefreshAt: null, launchAtLoginRegistered: undefined, autoInitLastAttemptAt: undefined }));
  const initialStatus = document.querySelector<HTMLElement>("[data-runtime-status]");
  if (initialStatus) initialStatus.textContent = initialRuntimeStatus.online ? "Online — waiting for first refresh" : "Offline — automatic polling paused";
  const startupStatus = document.querySelector<HTMLElement>("[data-startup-status]");
  if (startupStatus && initialRuntimeStatus.launchAtLoginRegistered !== undefined) startupStatus.textContent = initialRuntimeStatus.launchAtLoginRegistered ? "Launch at login is registered." : "Launch at login is not registered.";
  await listen<{ online: boolean; lastRefreshAt: number | null; launchAtLoginRegistered?: boolean; autoInitLastAttemptAt?: number | null }>("runtime-status-changed", (event) => {
    const status = document.querySelector<HTMLElement>("[data-runtime-status]");
    if (status) status.textContent = event.payload.online ? (event.payload.lastRefreshAt ? `Online · last refresh ${new Date(event.payload.lastRefreshAt * 1000).toLocaleTimeString()}` : "Online — waiting for first refresh") : "Offline — automatic polling paused";
    const startup = document.querySelector<HTMLElement>("[data-startup-status]");
    if (startup && event.payload.launchAtLoginRegistered !== undefined) startup.textContent = event.payload.launchAtLoginRegistered ? "Launch at login is registered." : "Launch at login is not registered.";
    const autoInit = document.querySelector<HTMLElement>("[data-auto-init-status]");
    if (autoInit) autoInit.textContent = event.payload.autoInitLastAttemptAt ? "Automatic initialization is cooling down after its last attempt." : "No automatic initialization attempt recorded.";
  });
  // Sign-in harvests the sessionKey from the auth webview's cookie jar, so the panel has to
  // repaint to drop the "paste this by hand" affordance it was showing a moment earlier.
  await listen("claude-session-key-captured", async () => {
    hasSessionKey = true;
    await loadSettingsData();
    renderSettingsWindow("account");
  });
  // The settings window is a single persistent webview created at startup, just shown/hidden
  // rather than reloaded — without this, an account signed in via a separate `claude` CLI
  // session (or a config change) while the window stays hidden would never be reflected once
  // the window reopens.
  await listen<string | null>("settings-shown", async (event) => {
    await loadSettingsData();
    renderSettingsWindow(event.payload ?? "general");
  });
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
      providerState = clearJustActivated(providerState);
    });
    await listen<ProviderUsageEvent>("usage-changed", (event) => {
      refreshProvider(event.payload.provider, event.payload.snapshot);
    });
    await listen<Config>("config-changed", (event) => {
      const changed = geometryChanged(config, event.payload);
      // The minimize chevrons and the tuck tab are mirrored onto the anchored edge, so a corner
      // change is a re-render, not just a re-measure — appearance alone would leave both
      // pointing at the edge the overlay used to sit on.
      const movedCorner = config.corner !== event.payload.corner;
      const readoutChanged = readoutShapeChanged(config, event.payload);
      config = event.payload;
      // A readout switch is a local repaint from the current provider snapshots. Waiting for
      // usage-changed tied this visual setting to the network poll interval for no reason.
      if (movedCorner || readoutChanged) renderMain();
      else applyAppearance();
      if (changed || readoutChanged) void applyGeometry();
    });
    // The restore is clicked in the tab's own window, so this one only hears about it here.
    // `tucked` is cleared by untuckIn rather than here, so nothing measures the stack until it
    // has finished travelling back.
    await listen<boolean>("overlay-tucked", (event) => {
      if (event.payload) tucked = true;
      else void untuckIn();
    });
    const bootstrap = await invoke<BootstrapPayload>("get_bootstrap");
    providerState = updateProviderSources(providerState, bootstrap.sources);
    for (const event of bootstrap.usage) providerState = updateProviderUsage(providerState, event);
    renderMain();
    providerState = clearJustActivated(providerState);
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

/** The edge tab is its own tiny window with no usage data of its own — it only needs the
 *  overlay's corner (to know which edge it sits on) and a way back. */
async function connectEdgeTab(): Promise<void> {
  // Deliberately motionless. The tab sits at the same work-area corner whether the overlay is
  // open or tucked, so animating this window would be the one thing that made it appear to move.
  const paint = () => {
    app.replaceChildren(renderEdgeTab(config.corner, () => {
      void invoke("set_overlay_tucked", { tucked: false }).catch(() => undefined);
    }));
  };
  try {
    config = await invoke<Config>("get_config");
    await listen<Config>("config-changed", (event) => {
      const changedCorner = config.corner !== event.payload.corner;
      config = event.payload;
      if (changedCorner) paint();
    });
  } catch {
    // Browser preview: fall through and paint with the default corner.
  }
  paint();
}

if (isHistoryWindow) {
  createRoot(app).render(createElement(HistoryApp));
} else if (isSettingsWindow) {
  void connectSettings();
} else if (isEdgeTabWindow) {
  void connectEdgeTab();
} else {
  void connectMain();
}
