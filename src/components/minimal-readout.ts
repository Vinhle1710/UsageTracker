import { formatPercent, formatReset } from "../format";
import { minimalSupportsMeterShape } from "../state";
import {
  enhanceMinimalReadout,
  type MinimalMotionAdapters,
} from "../motion/minimal-readout-motion";
import type { MeterShape, Provider, SnapshotMap, UsageSnapshot, UsageWindow } from "../types";
import type { ControlAction } from "./controls";
import { progressOffset, progressPercent } from "./layer";

export interface MinimalReadoutOptions {
  snapshots: SnapshotMap;
  now: number;
  meterShape: MeterShape;
  corner: string;
  onAction: (action: ControlAction) => void;
  onGeometryChange: () => Promise<void>;
  motionAdapters?: MinimalMotionAdapters;
}

const providerOrder: Provider[] = ["claude", "openai"];
const motionCleanups = new WeakMap<HTMLElement, () => void>();

function providerName(provider: Provider): string {
  return provider === "claude" ? "Claude" : "ChatGPT";
}

function providerLogo(provider: Provider): string {
  return provider === "claude" ? "/assets/claude-logo.png" : "/assets/chatgpt-logo.png";
}

function fiveHourWindow(snapshot: UsageSnapshot | undefined): UsageWindow | undefined {
  return snapshot?.windows.find((window) => /hour|min/i.test(window.label));
}

function weeklyWindow(snapshot: UsageSnapshot | undefined): UsageWindow | undefined {
  return snapshot?.windows.find((window) => !/hour|min/i.test(window.label));
}

function compatibleShape(shape: MeterShape): "ring" | "columns" | "semicircle" {
  return minimalSupportsMeterShape(shape) ? shape as "ring" | "columns" | "semicircle" : "ring";
}

function decorativeLogo(provider: Provider): HTMLImageElement {
  const logo = document.createElement("img");
  logo.className = "minimal-meter__logo";
  logo.src = providerLogo(provider);
  logo.alt = "";
  logo.setAttribute("aria-hidden", "true");
  return logo;
}

function ringBody(): SVGSVGElement {
  const ring = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  ring.classList.add("meter__ring");
  ring.setAttribute("viewBox", "0 0 100 100");
  ring.setAttribute("aria-hidden", "true");
  for (const className of ["meter__track", "meter__progress"]) {
    const circle = document.createElementNS("http://www.w3.org/2000/svg", "circle");
    circle.classList.add(className);
    circle.setAttribute("cx", "50");
    circle.setAttribute("cy", "50");
    circle.setAttribute("r", "44");
    ring.appendChild(circle);
  }
  return ring;
}

function columnsBody(): HTMLSpanElement {
  const track = document.createElement("span");
  track.className = "meter__columns";
  track.setAttribute("aria-hidden", "true");
  const fill = document.createElement("i");
  fill.className = "meter__columns-fill";
  track.appendChild(fill);
  return track;
}

function semicircleBody(): SVGSVGElement {
  const gauge = document.createElementNS("http://www.w3.org/2000/svg", "svg");
  gauge.classList.add("meter__semicircle");
  gauge.setAttribute("viewBox", "0 0 100 62");
  gauge.setAttribute("aria-hidden", "true");
  for (const className of ["meter__semicircle-track", "meter__semicircle-progress"]) {
    const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
    path.classList.add(className);
    path.setAttribute("d", "M 10 54 A 40 40 0 0 1 90 54");
    path.setAttribute("pathLength", "100");
    gauge.appendChild(path);
  }
  return gauge;
}

function createMeter(provider: Provider, kind: "five-hour" | "weekly", window: UsageWindow, shape: "ring" | "columns" | "semicircle", now: number): HTMLElement {
  const meter = document.createElement("div");
  meter.className = "minimal-meter";
  meter.dataset.provider = provider;
  meter.dataset.window = kind;
  meter.dataset.label = window.label;
  meter.dataset.shape = shape;
  meter.setAttribute("role", "progressbar");
  meter.setAttribute("aria-valuemin", "0");
  meter.setAttribute("aria-valuemax", "100");
  const body = shape === "ring" ? ringBody() : shape === "columns" ? columnsBody() : semicircleBody();
  meter.append(body, decorativeLogo(provider));
  const value = document.createElement("span");
  value.className = "minimal-meter__value";
  meter.appendChild(value);
  updateMeter(meter, provider, window, now);
  return meter;
}

function updateMeter(meter: HTMLElement, provider: Provider, window: UsageWindow, now: number): void {
  const percent = Math.min(100, Math.max(0, window.used_percent));
  const rounded = Math.round(window.used_percent);
  const reset = formatReset(window.label, window.resets_at, now);
  meter.dataset.label = window.label;
  meter.dataset.resetsAt = String(window.resets_at);
  meter.style.setProperty("--progress-offset", progressOffset(percent));
  meter.style.setProperty("--progress-percent", progressPercent(percent));
  meter.style.setProperty("--progress-percent-number", String(percent));
  meter.setAttribute("aria-valuenow", String(rounded));
  meter.setAttribute("aria-label", `${providerName(provider)} ${window.label} usage`);
  meter.setAttribute("aria-valuetext", `${rounded} percent used, ${reset}`);
  meter.querySelector<HTMLElement>(".minimal-meter__value")!.textContent = formatPercent(window.used_percent);
}

function unavailable(provider: Provider, kind: "five-hour" | "weekly"): HTMLElement {
  const state = document.createElement("div");
  state.className = "minimal-readout__unavailable";
  state.dataset.window = kind;
  state.setAttribute("aria-label", `${providerName(provider)} ${kind === "five-hour" ? "5 hour" : "Weekly"} usage unavailable`);
  state.appendChild(decorativeLogo(provider));
  const text = document.createElement("span");
  text.textContent = "Unavailable";
  state.appendChild(text);
  return state;
}

function createProvider(provider: Provider, snapshot: UsageSnapshot | undefined, shape: "ring" | "columns" | "semicircle", now: number): HTMLElement {
  const group = document.createElement("section");
  group.className = "minimal-readout__provider";
  group.dataset.provider = provider;

  const compact = document.createElement("div");
  compact.className = "minimal-readout__compact";
  const fiveHour = fiveHourWindow(snapshot);
  compact.appendChild(fiveHour ? createMeter(provider, "five-hour", fiveHour, shape, now) : unavailable(provider, "five-hour"));

  const weekly = document.createElement("div");
  weekly.className = "minimal-readout__weekly";
  weekly.setAttribute("aria-hidden", "true");
  const weeklyUsage = weeklyWindow(snapshot);
  weekly.appendChild(weeklyUsage ? createMeter(provider, "weekly", weeklyUsage, shape, now) : unavailable(provider, "weekly"));
  const label = document.createElement("span");
  label.className = "minimal-readout__weekly-label";
  label.textContent = "Weekly";
  weekly.appendChild(label);
  if (weeklyUsage) {
    const reset = document.createElement("span");
    reset.className = "minimal-readout__reset";
    reset.dataset.label = weeklyUsage.label;
    reset.dataset.resetsAt = String(weeklyUsage.resets_at);
    reset.textContent = formatReset(weeklyUsage.label, weeklyUsage.resets_at, now);
    weekly.appendChild(reset);
  }

  group.append(compact, weekly);
  return group;
}

function canUpdateProvider(group: HTMLElement, snapshot: UsageSnapshot | undefined, shape: string): boolean {
  const fiveHour = fiveHourWindow(snapshot);
  const weekly = weeklyWindow(snapshot);
  const fiveMeter = group.querySelector<HTMLElement>('[data-window="five-hour"]');
  const weeklyMeter = group.querySelector<HTMLElement>('[data-window="weekly"]');
  const fiveCompatible = Boolean(fiveHour) === (fiveMeter?.getAttribute("role") === "progressbar");
  const weeklyCompatible = Boolean(weekly) === (weeklyMeter?.getAttribute("role") === "progressbar");
  return fiveCompatible && weeklyCompatible
    && Array.from(group.querySelectorAll<HTMLElement>(".minimal-meter")).every((meter) => meter.dataset.shape === shape);
}

function updateProvider(group: HTMLElement, provider: Provider, snapshot: UsageSnapshot | undefined, now: number): void {
  const fiveHour = fiveHourWindow(snapshot);
  const weekly = weeklyWindow(snapshot);
  if (fiveHour) updateMeter(group.querySelector<HTMLElement>('[data-window="five-hour"]')!, provider, fiveHour, now);
  if (weekly) {
    updateMeter(group.querySelector<HTMLElement>('[data-window="weekly"]')!, provider, weekly, now);
    const reset = group.querySelector<HTMLElement>(".minimal-readout__reset")!;
    reset.dataset.label = weekly.label;
    reset.dataset.resetsAt = String(weekly.resets_at);
    reset.textContent = formatReset(weekly.label, weekly.resets_at, now);
  }
}

function actionButton(action: "settings" | "tuck", label: string, onAction: (action: ControlAction) => void): HTMLButtonElement {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "minimal-readout__dock-action";
  button.dataset.action = action;
  button.setAttribute("aria-label", label);
  button.title = label;
  button.tabIndex = -1;
  button.setAttribute("aria-hidden", "true");
  if (action === "settings") {
    button.setAttribute("aria-pressed", "false");
    button.dataset.settingsOpen = "false";
  }
  button.innerHTML = action === "settings"
    ? '<svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="8" cy="8" r="2.25"/><path d="M8 1.75v2M8 12.25v2M1.75 8h2M12.25 8h2M3.58 3.58 5 5m6 6 1.42 1.42m0-8.84L11 5m-6 6-1.42 1.42"/></svg>'
    : '<svg viewBox="0 0 16 16" aria-hidden="true"><path d="m10.5 3.5-4.5 4.5 4.5 4.5"/></svg>';
  button.addEventListener("click", () => onAction(action === "settings" ? { action: "toggle-settings" } : { action: "tuck" }));
  return button;
}

function createRoot(providers: Provider[], options: MinimalReadoutOptions, shape: "ring" | "columns" | "semicircle"): HTMLElement {
  const root = document.createElement("section");
  root.className = "minimal-readout";
  root.dataset.edge = options.corner.endsWith("left") ? "left" : "right";
  root.dataset.shape = shape;
  root.dataset.providers = providers.join(",");
  root.setAttribute("role", "region");
  root.setAttribute("aria-label", `${providers.map(providerName).join(" and ")} usage`);

  const reserved = document.createElement("div");
  reserved.className = "minimal-readout__reserved-bounds";
  const surfaceShell = document.createElement("div");
  surfaceShell.className = "minimal-readout__surface-shell";
  const surface = document.createElement("div");
  surface.className = "minimal-readout__surface minimal-readout__surface-region";
  surface.tabIndex = 0;
  surface.setAttribute("aria-label", "Show weekly usage");
  const stack = document.createElement("div");
  stack.className = "minimal-readout__providers";
  for (const provider of providers) stack.appendChild(createProvider(provider, options.snapshots[provider], shape, options.now));
  surface.appendChild(stack);
  surfaceShell.appendChild(surface);

  const actionShell = document.createElement("div");
  actionShell.className = "minimal-readout__action-shell";
  const handle = document.createElement("button");
  handle.type = "button";
  handle.className = "minimal-readout__dock-handle";
  handle.setAttribute("aria-label", "Show overlay actions");
  handle.setAttribute("aria-expanded", "false");
  handle.title = "Show overlay actions";
  const blade = document.createElement("span");
  blade.className = "minimal-readout__action-blade";
  blade.setAttribute("aria-hidden", "true");
  const dock = document.createElement("div");
  dock.className = "minimal-readout__dock";
  dock.setAttribute("aria-hidden", "true");
  dock.append(
    actionButton("settings", "Open settings", options.onAction),
    actionButton("tuck", "Tuck usage to the screen edge", options.onAction),
  );
  actionShell.append(handle, blade, dock);
  reserved.append(surfaceShell, actionShell);
  root.appendChild(reserved);
  motionCleanups.set(root, enhanceMinimalReadout(root, {
    onGeometryChange: options.onGeometryChange,
    adapters: options.motionAdapters,
  }));
  return root;
}

export function removeMinimalReadout(host: HTMLElement): void {
  const root = host.querySelector<HTMLElement>(":scope > .minimal-readout");
  if (!root) return;
  motionCleanups.get(root)?.();
  motionCleanups.delete(root);
  root.remove();
}

export function reconcileMinimalReadout(host: HTMLElement, requestedProviders: Provider[], options: MinimalReadoutOptions): void {
  const wanted = new Set(requestedProviders);
  const providers = providerOrder.filter((provider) => wanted.has(provider));
  const shape = compatibleShape(options.meterShape);
  let root = host.querySelector<HTMLElement>(":scope > .minimal-readout");
  const rebuild = !root
    || root.dataset.edge !== (options.corner.endsWith("left") ? "left" : "right")
    || root.dataset.shape !== shape
    || root.dataset.providers !== providers.join(",");
  if (rebuild) {
    if (root) {
      motionCleanups.get(root)?.();
      motionCleanups.delete(root);
      root.remove();
    }
    root = createRoot(providers, options, shape);
    host.replaceChildren(root);
    return;
  }

  const currentRoot = root!;
  for (const provider of providers) {
    const group = currentRoot.querySelector<HTMLElement>(`.minimal-readout__provider[data-provider="${provider}"]`)!;
    const snapshot = options.snapshots[provider];
    if (!canUpdateProvider(group, snapshot, shape)) {
      group.replaceWith(createProvider(provider, snapshot, shape, options.now));
    } else {
      updateProvider(group, provider, snapshot, options.now);
    }
  }
}

export function updateMinimalCountdowns(root: HTMLElement, now: number): void {
  for (const reset of root.querySelectorAll<HTMLElement>(".minimal-readout__reset")) {
    const label = reset.dataset.label;
    const resetsAt = Number(reset.dataset.resetsAt);
    if (!label || !Number.isFinite(resetsAt)) continue;
    reset.textContent = formatReset(label, resetsAt, now);
  }
}
