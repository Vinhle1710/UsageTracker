export type ControlAction = "minimize";

export function renderControls(onAction: (action: ControlAction) => void): HTMLElement {
  const bar = document.createElement("div");
  bar.className = "minimize-control";
  const button = document.createElement("button");
  button.type = "button";
  button.dataset.action = "minimize";
  button.setAttribute("aria-label", "Minimize overlay to screen edge");
  button.textContent = "›";
  button.addEventListener("click", () => onAction("minimize"));
  bar.appendChild(button);
  return bar;
}
