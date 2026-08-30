/** The overlay's third resting state: no cards, no bubbles, just a grab handle against the
 *  screen edge. Rendered into its own tiny always-on-top window (see the "edge-tab" entry in
 *  tauri.conf.json), which is why this is a standalone renderer rather than part of overlay.ts. */

/** Which screen edge the tab clings to. Derived from the overlay's own corner so the tab
 *  appears where the overlay just left, rather than at a fixed side. */
export function edgeForCorner(corner: string): "left" | "right" {
  return corner.endsWith("left") ? "left" : "right";
}

/** Short enough that the native show/hide lands while the eye is still following the motion —
 *  the two windows swap at the far end of this, so a long travel would show the seam. */
export const TUCK_MOTION_MS = 220;
export const TUCK_EASING = "cubic-bezier(.32,.72,0,1)";
const TUCK_TRAVEL_PX = 30;

/** The overlay leaves toward the edge it is anchored to and returns from it, and the tab does
 *  the mirror of that inside its own window. Kept pure so the direction — the only thing that
 *  can be wrong here without looking obviously broken — is checkable without a compositor. */
export function tuckKeyframes(edge: "left" | "right", direction: "out" | "in"): Keyframe[] {
  const away = (edge === "left" ? -1 : 1) * TUCK_TRAVEL_PX;
  const resting = { transform: "translateX(0px) scale(1)", opacity: 1 };
  const gone = { transform: `translateX(${away}px) scale(.94)`, opacity: 0 };
  return direction === "out" ? [resting, gone] : [gone, resting];
}

/** One button, two homes. The tab in the overlay and the tab in its own window are the same
 *  control at the same size in the same style — tucking must not look like the handle changed
 *  into something else — so both are built here and only the chevron turns around. */
function renderTabButton(edge: "left" | "right", tucked: boolean, onActivate: () => void): HTMLButtonElement {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "usage-tab__button";
  const label = tucked ? "Show usage overlay" : "Tuck usage to the screen edge";
  button.setAttribute("aria-label", label);
  button.title = label;
  // Open, it points at the edge it is about to disappear into; tucked, it points back at the
  // screen it will bring the overlay out of. Either way it names the direction of travel.
  const pointsRight = tucked ? edge === "left" : edge === "right";
  button.innerHTML = `
    <svg viewBox="0 0 16 16" aria-hidden="true" focusable="false">
      <path d="${pointsRight ? "m6 3.5 4.5 4.5-4.5 4.5" : "m10 3.5-4.5 4.5 4.5 4.5"}" />
    </svg>`;
  button.addEventListener("click", onActivate);
  // Matches renderControls' handling: a <button> already fires click on Enter/Space in a real
  // browser, but the overlay's controls are asserted on both paths so keyboard activation
  // cannot regress unnoticed in either.
  button.addEventListener("keydown", (event) => {
    if (event.key !== "Enter" && event.key !== " ") return;
    event.preventDefault();
    onActivate();
  });
  return button;
}

export function renderEdgeTab(corner: string, onRestore: () => void): HTMLElement {
  const edge = edgeForCorner(corner);
  const root = document.createElement("div");
  root.className = "edge-tab";
  root.dataset.edge = edge;
  root.appendChild(renderTabButton(edge, true, onRestore));
  return root;
}

/** The control that puts the overlay into that state, riding the anchored edge of the stack. */
export function renderTuckControl(onTuck: () => void, corner = "bottom-right"): HTMLElement {
  const edge = edgeForCorner(corner);
  const bar = document.createElement("div");
  bar.className = "tuck-control";
  bar.dataset.edge = edge;
  bar.appendChild(renderTabButton(edge, false, onTuck));
  return bar;
}
