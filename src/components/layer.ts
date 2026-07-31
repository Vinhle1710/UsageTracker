import { formatPercent, formatReset } from "../format";
import type { UsageSnapshot } from "../types";

export function renderLayer(name: string, snapshot: UsageSnapshot, now: number): HTMLElement {
  const root = document.createElement("section");
  root.className = "layer";
  root.dataset.state = snapshot.state;
  root.setAttribute("aria-labelledby", `layer-${name.toLowerCase()}`);

  const title = document.createElement("h2");
  title.id = `layer-${name.toLowerCase()}`;
  title.className = "layer__title";
  title.textContent = name;
  root.appendChild(title);

  if (snapshot.windows.length === 0) {
    const empty = document.createElement("p");
    empty.className = "layer__empty";
    empty.textContent = "No active window";
    root.appendChild(empty);
  }

  const grid = document.createElement("div");
  grid.className = "window-grid";
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
    meter.style.setProperty("--progress", `${Math.min(100, Math.max(0, window.used_percent))}%`);

    const value = document.createElement("span");
    value.className = "meter__value";
    value.textContent = formatPercent(window.used_percent);
    meter.appendChild(value);

    const reset = document.createElement("span");
    reset.className = "window-card__reset";
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
