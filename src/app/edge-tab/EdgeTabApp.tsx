export function EdgeTabApp({ side = "right", hidden = false, reducedMotion = false, onToggle = () => undefined }: { side?: "left" | "right"; hidden?: boolean; reducedMotion?: boolean; onToggle?: () => void }) {
  const direction = hidden ? (side === "right" ? "left" : "right") : side;
  return <main id="app" data-window="edge-tab"><button type="button" aria-label={hidden ? "Show usage overlay" : "Hide usage overlay"} data-direction={direction} style={{ width: 24, height: 48, transition: reducedMotion ? "none" : undefined }} onClick={onToggle}><span aria-hidden="true">{hidden ? "‹" : "›"}</span></button></main>;
}
