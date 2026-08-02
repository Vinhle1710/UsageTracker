import { formatPercent, formatReset } from "../format";
import type { UsageSnapshot, UsageWindow } from "../types";

const ringLength = 276.46;

function emptyUsageText(state: UsageSnapshot["state"]): string {
  // "pending" means the refresh has not landed yet, which is not the same claim as "stale" —
  // the latter asserts the usage is known to be unavailable.
  if (state === "pending") return "Checking usage…";
  if (state === "stale") return "Usage temporarily unavailable";
  if (state === "error") return "Sign-in required";
  return "No usage limits reported";
}

function providerHeader(name: string, root: HTMLElement): void {
  root.dataset.provider = name.toLowerCase() === "chatgpt" ? "openai" : "claude";
  root.setAttribute("aria-labelledby", `layer-${name.toLowerCase()}`);
  const title = document.createElement("h2");
  title.id = `layer-${name.toLowerCase()}`;
  title.className = "layer__title";
  const mark = document.createElement("span");
  mark.className = "provider-mark";
  mark.setAttribute("aria-hidden", "true");
  const logo = document.createElement("img");
  logo.src = name === "Claude" ? "/assets/claude-logo.png" : "/assets/chatgpt-logo.png";
  logo.alt = "";
  mark.appendChild(logo);
  const titleText = document.createElement("span");
  titleText.textContent = name;
  title.append(mark, titleText);
  root.appendChild(title);
}

export function renderLoadingLayer(name: string): HTMLElement {
  const root = document.createElement("section");
  root.className = "layer layer--loading";
  root.setAttribute("aria-busy", "true");
  providerHeader(name, root);
  const loading = document.createElement("p");
  loading.className = "layer__empty";
  loading.textContent = "Loading usage…";
  root.appendChild(loading);
  return root;
}

function progressOffset(percent: number): string {
  return String(ringLength * (1 - Math.min(100, Math.max(0, percent)) / 100));
}

function updateMeter(meter: HTMLElement, name: string, window: UsageWindow, now: number): void {
  const rounded = Math.round(window.used_percent);
  const resetText = formatReset(window.label, window.resets_at, now);
  meter.setAttribute("aria-valuenow", String(rounded));
  meter.setAttribute("aria-valuetext", `${rounded} percent used, ${resetText}`);
  meter.dataset.resetsAt = String(window.resets_at);
  meter.style.setProperty("--progress-offset", progressOffset(window.used_percent));
  const value = meter.querySelector<HTMLElement>(".meter__value");
  if (value) value.textContent = formatPercent(window.used_percent);
  const reset = meter.closest<HTMLElement>(".window-card")?.querySelector<HTMLElement>(".window-card__reset");
  if (reset) {
    reset.dataset.resetsAt = String(window.resets_at);
    reset.textContent = resetText;
  }
  meter.setAttribute("aria-label", `${name} ${window.label} usage`);
}

export function updateLayer(root: HTMLElement, snapshot: UsageSnapshot, now: number): boolean {
  if (root.classList.contains("layer--loading")) return false;
  const meters = Array.from(root.querySelectorAll<HTMLElement>(".meter"));
  const existingLabels = meters.map((meter) => meter.dataset.label);
  if (meters.length !== snapshot.windows.length || snapshot.windows.some((window) => !existingLabels.includes(window.label))) return false;

  const name = root.dataset.provider === "openai" ? "ChatGPT" : "Claude";
  root.dataset.state = snapshot.state;
  const empty = root.querySelector<HTMLElement>(".layer__empty");
  if (empty && snapshot.windows.length === 0) empty.textContent = emptyUsageText(snapshot.state);
  for (const window of snapshot.windows) {
    const meter = meters.find((candidate) => candidate.dataset.label === window.label);
    if (meter) updateMeter(meter, name, window, now);
  }

  const existingHint = root.querySelector<HTMLElement>(".layer__hint");
  if (snapshot.state === "error" && !existingHint) {
    const hint = document.createElement("p");
    hint.className = "layer__hint";
    hint.textContent = "Re-authenticate in the CLI";
    root.appendChild(hint);
  } else if (snapshot.state !== "error") {
    existingHint?.remove();
  }
  return true;
}

export function renderLayer(name: string, snapshot: UsageSnapshot, now: number, previous?: UsageSnapshot): HTMLElement {
  const root = document.createElement("section");
  root.className = "layer";
  root.dataset.state = snapshot.state;
  providerHeader(name, root);

  if (snapshot.windows.length === 0) {
    const empty = document.createElement("p");
    empty.className = "layer__empty";
    empty.textContent = emptyUsageText(snapshot.state);
    root.appendChild(empty);
  }

  const grid = document.createElement("div");
  grid.className = "window-grid";
  grid.dataset.singleWindow = String(snapshot.windows.length === 1);
  for (const window of snapshot.windows) {
    const card = document.createElement("div");
    card.className = "window-card";
    card.dataset.windowLabel = window.label;

    const label = document.createElement("span");
    label.className = "window-card__label";
    label.textContent = window.label;

    const meter = document.createElement("div");
    meter.className = "meter";
    meter.setAttribute("role", "progressbar");
    meter.setAttribute("aria-valuenow", String(Math.round(window.used_percent)));
    meter.setAttribute("aria-valuemin", "0");
    meter.setAttribute("aria-valuemax", "100");
    meter.setAttribute("aria-label", `${name} ${window.label} usage`);
    meter.setAttribute("aria-valuetext", `${Math.round(window.used_percent)} percent used, ${formatReset(window.label, window.resets_at, now)}`);
    const percent = Math.min(100, Math.max(0, window.used_percent));
    meter.dataset.provider = root.dataset.provider;
    meter.dataset.label = window.label;
    meter.dataset.resetsAt = String(window.resets_at);
    meter.style.setProperty("--progress-offset", progressOffset(percent));
    const previousWindow = previous?.windows.find((candidate) => candidate.label === window.label);
    if (previousWindow && previousWindow.used_percent !== window.used_percent) {
      meter.dataset.usageChange = window.used_percent > previousWindow.used_percent ? "increase" : "decrease";
      meter.style.setProperty("--previous-progress-offset", progressOffset(previousWindow.used_percent));
    }

    const ring = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    ring.classList.add("meter__ring");
    ring.setAttribute("viewBox", "0 0 100 100");
    ring.setAttribute("aria-hidden", "true");
    const track = document.createElementNS("http://www.w3.org/2000/svg", "circle");
    track.classList.add("meter__track");
    track.setAttribute("cx", "50");
    track.setAttribute("cy", "50");
    track.setAttribute("r", "44");
    const progress = document.createElementNS("http://www.w3.org/2000/svg", "circle");
    progress.classList.add("meter__progress");
    progress.setAttribute("cx", "50");
    progress.setAttribute("cy", "50");
    progress.setAttribute("r", "44");
    ring.append(track, progress);
    meter.appendChild(ring);

    const value = document.createElement("span");
    value.className = "meter__value";
    value.textContent = formatPercent(window.used_percent);
    meter.appendChild(value);

    const reset = document.createElement("span");
    reset.className = "window-card__reset";
    reset.dataset.label = window.label;
    reset.dataset.resetsAt = String(window.resets_at);
    reset.textContent = formatReset(window.label, window.resets_at, now);

    card.append(meter, label, reset);
    grid.appendChild(card);
  }
  if (snapshot.windows.length) root.appendChild(grid);

  if (snapshot.state === "error") {
    const hint = document.createElement("p");
    hint.className = "layer__hint";
    hint.textContent = "Re-authenticate in the CLI";
    root.appendChild(hint);
  }

  return root;
}
