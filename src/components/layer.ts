import { formatPercent, formatReset } from "../format";
import type { UsageSnapshot } from "../types";

export function renderLayer(name: string, snapshot: UsageSnapshot, now: number, previous?: UsageSnapshot): HTMLElement {
  const root = document.createElement("section");
  root.className = "layer";
  root.dataset.provider = name.toLowerCase() === "chatgpt" ? "openai" : "claude";
  root.dataset.state = snapshot.state;
  root.setAttribute("aria-labelledby", `layer-${name.toLowerCase()}`);

  const title = document.createElement("h2");
  title.id = `layer-${name.toLowerCase()}`;
  title.className = "layer__title";
  const mark = document.createElement("span");
  mark.className = "provider-mark";
  mark.setAttribute("aria-hidden", "true");
  mark.innerHTML = name === "Claude"
    ? `<svg viewBox="0 0 24 24"><path d="M12 1.8l2.2 6.2 6.2 2.2-6.2 2.2-2.2 6.2-2.2-6.2-6.2-2.2 6.2-2.2L12 1.8z"/></svg>`
    : `<svg viewBox="0 0 24 24"><path d="M8.4 4.1c1.4-1.3 3.7-1.1 4.9.4l1.1 1.4 1.7-.5c2-.6 4 .6 4.5 2.6.4 1.6-.3 3.3-1.7 4.2l-1.5 1 1 1.5c1.1 1.7.6 4-1.1 5.1-1.4.9-3.2.7-4.4-.5l-1.1-1.3-1.7.5c-2 .6-4-.6-4.5-2.6-.4-1.6.3-3.3 1.7-4.2l1.5-1-1-1.5c-1.1-1.7-.6-4 1.1-5.1z"/></svg>`;
  const titleText = document.createElement("span");
  titleText.textContent = name;
  title.append(mark, titleText);
  root.appendChild(title);

  if (snapshot.windows.length === 0) {
    const empty = document.createElement("p");
    empty.className = "layer__empty";
    empty.textContent = "No active window";
    root.appendChild(empty);
  }

  const grid = document.createElement("div");
  grid.className = "window-grid";
  grid.dataset.singleWindow = String(snapshot.windows.length === 1);
  for (const window of snapshot.windows) {
    const card = document.createElement("div");
    card.className = "window-card";

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
    meter.style.setProperty("--progress-offset", String(276.46 * (1 - percent / 100)));
    const previousWindow = previous?.windows.find((candidate) => candidate.label === window.label);
    if (previousWindow && previousWindow.used_percent !== window.used_percent) {
      meter.dataset.usageChange = window.used_percent > previousWindow.used_percent ? "increase" : "decrease";
      meter.style.setProperty("--previous-progress-offset", String(276.46 * (1 - Math.min(100, Math.max(0, previousWindow.used_percent)) / 100)));
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
