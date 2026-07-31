import type { SizeState } from "../types";

export interface ControlsState {
  sizeState: SizeState;
  alwaysOnTop: boolean;
}

export type ControlAction = "bubble" | "resize" | "pin" | "settings";

export function renderControls(state: ControlsState, onAction: (action: ControlAction) => void): HTMLElement {
  const bar = document.createElement("div");
  bar.className = "controls";
  bar.setAttribute("role", "toolbar");
  bar.setAttribute("aria-label", "Overlay controls");

  const definitions: Array<[ControlAction, string, string]> = [
    ["bubble", "Minimize to bubble", "−"],
    ["resize", state.sizeState === "square" ? "Shrink panel" : "Expand panel", "□"],
    ["pin", "Always on top", "◉"],
    ["settings", "Settings", "⚙"],
  ];

  for (const [action, label, glyph] of definitions) {
    const button = document.createElement("button");
    button.type = "button";
    button.dataset.action = action;
    button.setAttribute("aria-label", label);
    if (action === "pin") button.setAttribute("aria-pressed", String(state.alwaysOnTop));
    button.textContent = glyph;
    button.addEventListener("click", () => onAction(action));
    bar.appendChild(button);
  }
  return bar;
}
