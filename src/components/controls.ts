import type { Provider } from "../types";

export type ControlAction =
  | { action: "minimize"; provider: Provider }
  | { action: "restore"; provider: Provider };

export function providerLabel(provider: Provider): string {
  return provider === "claude" ? "Claude" : "ChatGPT";
}

export function renderControls(provider: Provider, onAction: (action: ControlAction) => void): HTMLElement {
  const bar = document.createElement("div");
  bar.className = "minimize-control";
  const button = document.createElement("button");
  button.type = "button";
  button.className = "minimize-control__button";
  button.dataset.action = "minimize";
  button.dataset.provider = provider;
  button.setAttribute("aria-label", `Minimize ${providerLabel(provider)} usage`);
  button.title = `Minimize ${providerLabel(provider)} usage`;
  button.innerHTML = `
    <svg viewBox="0 0 16 16" aria-hidden="true" focusable="false">
      <path d="m5.5 3.5 4.5 4.5-4.5 4.5" />
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
