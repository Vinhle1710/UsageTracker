export type ControlAction = "minimize";

export function renderControls(onAction: (action: ControlAction) => void): HTMLElement {
  const bar = document.createElement("div");
  bar.className = "minimize-control";
  const button = document.createElement("button");
  button.type = "button";
  button.className = "minimize-control__button";
  button.dataset.action = "minimize";
  button.setAttribute("aria-label", "Minimize overlay to screen edge");
  button.title = "Minimize to screen edge";
  button.innerHTML = `
    <svg viewBox="0 0 16 16" aria-hidden="true" focusable="false">
      <path d="m5.5 3.5 4.5 4.5-4.5 4.5" />
    </svg>`;
  button.addEventListener("click", () => onAction("minimize"));
  bar.appendChild(button);
  return bar;
}
