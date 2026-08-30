import type { Provider } from "../types";

export type ControlAction =
  | { action: "minimize"; provider: Provider }
  | { action: "restore"; provider: Provider }
  | { action: "open-cli"; provider: Provider }
  | { action: "open-settings"; page?: string };

export function providerLabel(provider: Provider): string {
  return provider === "claude" ? "Claude" : "ChatGPT";
}

export function renderControls(
  provider: Provider,
  onAction: (action: ControlAction) => void,
  corner = "bottom-right",
): HTMLElement {
  const edge = corner.endsWith("left") ? "left" : "right";
  const bar = document.createElement("div");
  bar.className = "minimize-control";
  bar.dataset.edge = edge;
  const button = document.createElement("button");
  button.type = "button";
  button.className = "minimize-control__button";
  button.dataset.action = "minimize";
  button.dataset.provider = provider;
  button.setAttribute("aria-label", `Minimize ${providerLabel(provider)} usage`);
  button.title = `Minimize ${providerLabel(provider)} usage`;
  // Points at the nearest screen edge, which is also the direction the card collapses toward:
  // the bubble it becomes sits on the anchored side, so a fixed rightward chevron pointed away
  // from the destination on a left-anchored overlay.
  button.innerHTML = `
    <svg viewBox="0 0 16 16" aria-hidden="true" focusable="false">
      <path d="${edge === "right" ? "m5.5 3.5 4.5 4.5-4.5 4.5" : "m10.5 3.5-4.5 4.5 4.5 4.5"}" />
    </svg>`;
  const activate = () => onAction({ action: "minimize", provider });
  button.addEventListener("click", activate);
  button.addEventListener("keydown", (event) => {
    if (event.key !== "Enter" && event.key !== " ") return;
    event.preventDefault();
    activate();
  });
  bar.appendChild(button);
  return bar;
}
