export interface ConfettiPiece {
  angle: number;
  distance: number;
  delayMs: number;
  colorVar: string;
}

const CELEBRATION_COLOR_VARS = ["--claude", "--chatgpt", "--accent", "--warn"];
const CELEBRATION_DURATION_MS = 950;
const CELEBRATION_CLEANUP_MS = 1100;

/** Deterministic given `rand`, so tests can assert exact output without relying on Math.random. */
export function generateConfetti(count: number, rand: () => number = Math.random): ConfettiPiece[] {
  return Array.from({ length: count }, (_, index) => ({
    angle: (360 / count) * index + (rand() * 24 - 12),
    distance: 34 + rand() * 18,
    delayMs: rand() * 90,
    colorVar: CELEBRATION_COLOR_VARS[index % CELEBRATION_COLOR_VARS.length],
  }));
}

/** Appends the burst as a child of `card`, so it is visually and structurally scoped to the
 *  single window-card whose limit reset — never a sibling card, never the whole app. */
export function spawnCelebration(card: HTMLElement, pieces: ConfettiPiece[]): void {
  const burst = document.createElement("div");
  burst.className = "celebration-burst";
  burst.setAttribute("aria-hidden", "true");
  for (const piece of pieces) {
    const dot = document.createElement("span");
    dot.className = "celebration-piece";
    dot.style.setProperty("--angle", `${piece.angle}deg`);
    dot.style.setProperty("--distance", `${piece.distance}px`);
    dot.style.setProperty("--delay", `${piece.delayMs}ms`);
    dot.style.setProperty("--piece-color", `var(${piece.colorVar})`);
    burst.appendChild(dot);
  }
  card.appendChild(burst);
  window.setTimeout(() => burst.remove(), CELEBRATION_CLEANUP_MS);
}

export { CELEBRATION_DURATION_MS };
