/** The overlay's third resting state: no cards, no bubbles, just a grab handle against the
 *  screen edge. Rendered into its own tiny always-on-top window (see the "edge-tab" entry in
 *  tauri.conf.json), which is why this is a standalone renderer rather than part of overlay.ts. */

/** Which screen edge the tab clings to. Derived from the overlay's own corner so the tab
 *  appears where the overlay just left, rather than at a fixed side. */
export function edgeForCorner(corner: string): "left" | "right" {
  return corner.endsWith("left") ? "left" : "right";
}

export function renderEdgeTab(corner: string, onRestore: () => void): HTMLElement {
  const edge = edgeForCorner(corner);
  const root = document.createElement("div");
  root.className = "edge-tab";
  root.dataset.edge = edge;

  const button = document.createElement("button");
  button.type = "button";
  button.className = "edge-tab__button";
  button.setAttribute("aria-label", "Show usage overlay");
  button.title = "Show usage overlay";
  // Points back toward the screen, not into the bezel: a right-edge tab is pulled leftward.
  button.innerHTML = `
    <svg viewBox="0 0 16 16" aria-hidden="true" focusable="false">
      <path d="${edge === "right" ? "m10 3.5-4.5 4.5 4.5 4.5" : "m6 3.5 4.5 4.5-4.5 4.5"}" />
    </svg>`;
  button.addEventListener("click", onRestore);
  root.appendChild(button);
  return root;
}

/** The control that puts the overlay into that state, shown on the bubble row. */
export function renderTuckControl(onTuck: () => void): HTMLElement {
  const bar = document.createElement("div");
  bar.className = "tuck-control";
  const button = document.createElement("button");
  button.type = "button";
  button.className = "tuck-control__button";
  const label = "Tuck usage to the screen edge";
  button.setAttribute("aria-label", label);
  button.title = label;
  button.innerHTML = `
    <svg viewBox="0 0 16 16" aria-hidden="true" focusable="false">
      <path d="m6 3.5 4.5 4.5-4.5 4.5" />
      <path d="M13 3v10" />
    </svg>`;
  button.addEventListener("click", onTuck);
  // Matches renderControls' handling: a <button> already fires click on Enter/Space in a real
  // browser, but the overlay's controls are asserted on both paths so keyboard activation
  // cannot regress unnoticed in either.
  button.addEventListener("keydown", (event) => {
    if (event.key !== "Enter" && event.key !== " ") return;
    event.preventDefault();
    onTuck();
  });
  bar.appendChild(button);
  return bar;
}
