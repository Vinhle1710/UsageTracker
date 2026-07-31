import { formatAge, formatPercent, formatReset } from "../format";
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

  for (const window of snapshot.windows) {
    const row = document.createElement("div");
    row.className = "window-row";

    const label = document.createElement("span");
    label.className = "window-row__label";
    label.textContent = window.label;

    const value = document.createElement("span");
    value.className = "window-row__value";
    value.textContent = formatPercent(window.used_percent);

    const bar = document.createElement("div");
    bar.className = "bar";
    bar.setAttribute("role", "progressbar");
    bar.setAttribute("aria-valuenow", String(Math.round(window.used_percent)));
    bar.setAttribute("aria-valuemin", "0");
    bar.setAttribute("aria-valuemax", "100");
    bar.setAttribute("aria-label", `${name} ${window.label} usage`);
    bar.setAttribute("aria-valuetext", `${Math.round(window.used_percent)} percent used, ${formatReset(window.resets_at, now)}`);

    const fill = document.createElement("div");
    fill.className = "bar__fill";
    fill.style.width = `${Math.min(100, Math.max(0, window.used_percent))}%`;
    bar.appendChild(fill);

    const reset = document.createElement("span");
    reset.className = "window-row__reset";
    reset.textContent = formatReset(window.resets_at, now);

    row.append(label, value, bar, reset);
    root.appendChild(row);
  }

  if (snapshot.state === "error") {
    const hint = document.createElement("p");
    hint.className = "layer__hint";
    hint.textContent = "Re-authenticate in the CLI";
    root.appendChild(hint);
  }

  const age = document.createElement("p");
  age.className = "layer__age";
  age.textContent = `Updated ${formatAge(snapshot.fetched_at, now)}`;
  root.appendChild(age);
  return root;
}
