import { formatPercent, formatReset } from "../format";
import type { ControlAction } from "./controls";
import type { ClaudeExtra, MeterShape, Money, Provider, UsageSnapshot, UsageWindow } from "../types";

const ringLength = 276.46;

function emptyUsageText(state: UsageSnapshot["state"]): string {
  // "pending" means the refresh has not landed yet, which is not the same claim as "stale" —
  // the latter asserts the usage is known to be unavailable.
  if (state === "pending") return "Checking usage…";
  if (state === "stale") return "Usage temporarily unavailable";
  if (state === "signed-out") return "Not signed in";
  if (state === "error") return "Sign-in required";
  return "No usage limits reported";
}

function providerKeyFromName(name: string): Provider {
  return name === "ChatGPT" ? "openai" : "claude";
}

function hintText(state: UsageSnapshot["state"], name: string): string | null {
  if (name === "Claude") {
    if (state === "signed-out") return "Run claude to sign in";
    if (state === "error") return "Re-authenticate in Claude Code";
    return null;
  }
  if (state === "signed-out") return "Run codex to sign in";
  if (state === "error") return "Re-authenticate in the CLI";
  return null;
}

// Both hint states name a one-action fix, so the hint doubles as a button that opens the
// corresponding provider CLI in a terminal.
function createHintButton(text: string, provider: Provider, onAction?: (action: ControlAction) => void): HTMLElement {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "layer__hint";
  button.textContent = text;
  button.addEventListener("click", () => {
    onAction?.({ action: "open-cli", provider });
  });
  return button;
}

function providerHeader(name: string, root: HTMLElement): void {
  root.dataset.provider = name.toLowerCase() === "chatgpt" ? "openai" : "claude";
  root.setAttribute("aria-labelledby", `layer-${name.toLowerCase()}`);
  const title = document.createElement("h2");
  title.id = `layer-${name.toLowerCase()}`;
  title.className = "layer__title";
  const mark = document.createElement("span");
  mark.className = "provider-mark";
  mark.setAttribute("aria-hidden", "true");
  const logo = document.createElement("img");
  logo.src = name === "Claude" ? "/assets/claude-logo.png" : "/assets/chatgpt-logo.png";
  logo.alt = "";
  mark.appendChild(logo);
  const titleText = document.createElement("span");
  titleText.textContent = name;
  title.append(mark, titleText);
  root.appendChild(title);
}

export function renderLoadingLayer(name: string): HTMLElement {
  const root = document.createElement("section");
  root.className = "layer layer--loading";
  root.setAttribute("aria-busy", "true");
  providerHeader(name, root);
  const loading = document.createElement("p");
  loading.className = "layer__empty";
  loading.textContent = "Loading usage…";
  root.appendChild(loading);
  return root;
}

export function progressOffset(percent: number): string {
  return String(ringLength * (1 - Math.min(100, Math.max(0, percent)) / 100));
}

/** The bar and line shapes fill a track by width, so they read a percentage rather than the
 *  ring's SVG dash offset. Both are written on every meter regardless of shape, which is what
 *  lets updateMeter stay shape-agnostic. */
export function progressPercent(percent: number): string {
  return `${Math.min(100, Math.max(0, percent))}%`;
}

/** The readout itself, minus the label/value text every shape shares. Ring keeps the SVG;
 *  every linear instrument is a track plus a fill driven by --progress-percent. */
function buildMeterBody(shape: MeterShape): Node[] {
  if (shape === "ring") {
    const ring = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    ring.classList.add("meter__ring");
    ring.setAttribute("viewBox", "0 0 100 100");
    ring.setAttribute("aria-hidden", "true");
    for (const [className, element] of [["meter__track", "circle"], ["meter__progress", "circle"]] as const) {
      const circle = document.createElementNS("http://www.w3.org/2000/svg", element);
      circle.classList.add(className);
      circle.setAttribute("cx", "50");
      circle.setAttribute("cy", "50");
      circle.setAttribute("r", "44");
      ring.appendChild(circle);
    }
    return [ring];
  }
  if (shape === "reactor") {
    const reactor = document.createElement("span");
    reactor.className = "meter__reactor";
    reactor.setAttribute("aria-hidden", "true");
    for (let index = 0; index < 16; index += 1) {
      const segment = document.createElement("i");
      segment.className = "meter__reactor-segment";
      segment.dataset.segmentIndex = String(index);
      segment.style.setProperty("--segment-index", String(index));
      reactor.appendChild(segment);
    }
    const core = document.createElement("span");
    core.className = "meter__reactor-core";
    reactor.appendChild(core);
    return [reactor];
  }
  if (shape === "semicircle") {
    const gauge = document.createElementNS("http://www.w3.org/2000/svg", "svg");
    gauge.classList.add("meter__semicircle");
    gauge.setAttribute("viewBox", "0 0 100 62");
    gauge.setAttribute("aria-hidden", "true");
    for (const className of ["meter__semicircle-track", "meter__semicircle-progress"]) {
      const path = document.createElementNS("http://www.w3.org/2000/svg", "path");
      path.classList.add(className);
      path.setAttribute("d", "M 10 54 A 40 40 0 0 1 90 54");
      path.setAttribute("pathLength", "100");
      gauge.appendChild(path);
    }
    return [gauge];
  }
  const track = document.createElement("span");
  track.className = `meter__${shape}`;
  track.setAttribute("aria-hidden", "true");
  const fill = document.createElement("i");
  fill.className = `meter__${shape}-fill`;
  track.appendChild(fill);
  return [track];
}

function updateReactorSegments(meter: HTMLElement, percent: number): void {
  const activeCount = Math.ceil((Math.min(100, Math.max(0, percent)) / 100) * 16);
  for (const segment of meter.querySelectorAll<HTMLElement>(".meter__reactor-segment")) {
    segment.classList.toggle("is-active", Number(segment.dataset.segmentIndex) < activeCount);
  }
}

function formatMoney(money: Money): string {
  const amount = (money.minorUnits / 100).toFixed(2);
  return money.currency === "USD" ? `$${amount}` : `${amount} ${money.currency}`;
}

function extraOf(snapshot: UsageSnapshot): ClaudeExtra | null {
  const extra = snapshot.details?.claude.extra;
  if (!extra?.value) return null;
  const { spend, budget, balance } = extra.value;
  // A section that resolved but carries no amounts is the same as no section at all — the
  // user has extra usage switched off, and an empty bar would claim a limit that isn't there.
  return spend || budget || balance ? extra.value : null;
}

/** Spend against a budget is the only pair that makes a percentage; a bare grant balance has
 *  no denominator, so it is reported as a number rather than drawn against an invented one. */
function extraFill(extra: ClaudeExtra): number | null {
  if (!extra.budget || !extra.spend) return null;
  if (extra.budget.minorUnits <= 0) return 0;
  return Math.min(100, (extra.spend.minorUnits / extra.budget.minorUnits) * 100);
}

function extraLabel(extra: ClaudeExtra): string {
  if (extra.budget && extra.spend) return `${formatMoney(extra.spend)} of ${formatMoney(extra.budget)}`;
  if (extra.balance) return `${formatMoney(extra.balance)} left`;
  return extra.spend ? `${formatMoney(extra.spend)} used` : "";
}

/** Rewrites an existing extra-credit row in place. Returns false when the row cannot represent
 *  this data (a fill appearing or disappearing changes the element's shape and semantics), so
 *  the caller rebuilds it instead of leaving a progressbar with nothing to measure. */
function updateExtraCredit(row: HTMLElement, extra: ClaudeExtra): boolean {
  const fill = extraFill(extra);
  if ((fill === null) !== (row.querySelector(".extra-credit__fill") === null)) return false;
  const label = extraLabel(extra);
  row.querySelector<HTMLElement>(".extra-credit__amount")!.textContent = label;
  if (fill === null) return true;
  row.style.setProperty("--progress-percent", progressPercent(fill));
  row.setAttribute("aria-valuenow", String(Math.round(fill)));
  row.setAttribute("aria-valuetext", `Extra credit, ${label}`);
  return true;
}

function renderExtraCredit(extra: ClaudeExtra): HTMLElement {
  const row = document.createElement("div");
  row.className = "extra-credit";
  const label = extraLabel(extra);

  const caption = document.createElement("span");
  caption.className = "extra-credit__label";
  caption.textContent = "Extra credit";
  const amount = document.createElement("span");
  amount.className = "extra-credit__amount";
  amount.textContent = label;

  const fill = extraFill(extra);
  if (fill !== null) {
    row.setAttribute("role", "progressbar");
    row.setAttribute("aria-valuemin", "0");
    row.setAttribute("aria-valuemax", "100");
    row.setAttribute("aria-valuenow", String(Math.round(fill)));
    row.setAttribute("aria-valuetext", `Extra credit, ${label}`);
    row.style.setProperty("--progress-percent", progressPercent(fill));
    const track = document.createElement("span");
    track.className = "extra-credit__track";
    track.setAttribute("aria-hidden", "true");
    const bar = document.createElement("i");
    bar.className = "extra-credit__fill";
    track.appendChild(bar);
    row.append(caption, amount, track);
    return row;
  }
  row.append(caption, amount);
  return row;
}

export function updateMeter(meter: HTMLElement, name: string, window: UsageWindow, now: number): void {
  const rounded = Math.round(window.used_percent);
  const resetText = formatReset(window.label, window.resets_at, now);
  meter.setAttribute("aria-valuenow", String(rounded));
  meter.setAttribute("aria-valuetext", `${rounded} percent used, ${resetText}`);
  meter.dataset.resetsAt = String(window.resets_at);
  meter.style.setProperty("--progress-offset", progressOffset(window.used_percent));
  meter.style.setProperty("--progress-percent", progressPercent(window.used_percent));
  meter.style.setProperty("--progress-percent-number", String(Math.min(100, Math.max(0, window.used_percent))));
  updateReactorSegments(meter, window.used_percent);
  renderPace(meter, window);
  const value = meter.querySelector<HTMLElement>(".meter__value");
  if (value) value.textContent = formatPercent(window.used_percent);
  const reset = meter.closest<HTMLElement>(".window-card")?.querySelector<HTMLElement>(".window-card__reset");
  if (reset) {
    if (reset.dataset.resetsAt !== String(window.resets_at)) delete reset.dataset.cachedMessage;
    reset.dataset.resetsAt = String(window.resets_at);
    reset.textContent = reset.dataset.cachedMessage ?? resetText;
  }
  meter.setAttribute("aria-label", `${name} ${window.label} usage`);
}

function renderPace(meter: HTMLElement, window: UsageWindow): void {
  meter.querySelector(".meter__pace")?.remove();
  meter.closest(".window-card")?.querySelector(".window-card__pace")?.remove();
  if (!window.pace) return;
  const marker = document.createElement("span"); marker.className = "meter__pace"; marker.dataset.testid = "pace-marker";
  marker.setAttribute("aria-hidden", "true"); marker.style.left = `${window.pace.expectedPercent}%`; meter.appendChild(marker);
  const text = document.createElement("p"); text.className = "window-card__pace";
  const amount = Math.round(Math.abs(window.pace.deltaPercent));
  text.textContent = window.pace.status === "ahead" ? `${amount} points ahead of pace` : window.pace.status === "behind" ? `${amount} points under pace` : "On pace";
  meter.closest(".window-card")?.appendChild(text);
}

export function updateLayer(root: HTMLElement, snapshot: UsageSnapshot, now: number, onAction?: (action: ControlAction) => void, shape: MeterShape = "ring"): boolean {
  if (root.classList.contains("layer--loading")) return false;
  const meters = Array.from(root.querySelectorAll<HTMLElement>(".meter"));
  const existingLabels = meters.map((meter) => meter.dataset.label);
  if (meters.length !== snapshot.windows.length || snapshot.windows.some((window) => !existingLabels.includes(window.label))) return false;
  // A shape change swaps the readout element itself, which a value patch cannot do — fall
  // back to a full rebuild rather than leaving a ring behind under a "bar" attribute.
  if (meters.some((meter) => meter.dataset.shape !== shape)) return false;

  const name = root.dataset.provider === "openai" ? "ChatGPT" : "Claude";
  root.dataset.state = snapshot.state;
  const empty = root.querySelector<HTMLElement>(".layer__empty");
  if (empty && snapshot.windows.length === 0) empty.textContent = emptyUsageText(snapshot.state);
  for (const window of snapshot.windows) {
    const meter = meters.find((candidate) => candidate.dataset.label === window.label);
    if (meter) updateMeter(meter, name, window, now);
  }

  const extraRow = root.querySelector<HTMLElement>(".extra-credit");
  const extra = extraOf(snapshot);
  if (!extra) {
    extraRow?.remove();
  } else if (!extraRow) {
    root.querySelector(".window-grid")?.after(renderExtraCredit(extra));
  } else if (!updateExtraCredit(extraRow, extra)) {
    extraRow.replaceWith(renderExtraCredit(extra));
  }

  const existingHint = root.querySelector<HTMLElement>(".layer__hint");
  const hint = hintText(snapshot.state, name);
  if (!hint) {
    existingHint?.remove();
  } else {
    // Replaced rather than patched in place: reusing the element would leave it bound to
    // whichever `onAction` closure was in scope when it was first created.
    existingHint?.remove();
    root.appendChild(createHintButton(hint, providerKeyFromName(name), onAction));
  }
  return true;
}

export function renderLayer(name: string, snapshot: UsageSnapshot, now: number, previous?: UsageSnapshot, onAction?: (action: ControlAction) => void, shape: MeterShape = "ring"): HTMLElement {
  const root = document.createElement("section");
  root.className = "layer";
  root.dataset.state = snapshot.state;
  providerHeader(name, root);

  if (snapshot.windows.length === 0) {
    const empty = document.createElement("p");
    empty.className = "layer__empty";
    empty.textContent = emptyUsageText(snapshot.state);
    root.appendChild(empty);
  }

  const grid = document.createElement("div");
  grid.className = "window-grid";
  grid.dataset.singleWindow = String(snapshot.windows.length === 1);
  grid.dataset.shape = shape;
  for (const window of snapshot.windows) {
    const card = document.createElement("div");
    card.className = "window-card";
    card.dataset.windowLabel = window.label;

    const label = document.createElement("span");
    label.className = "window-card__label";
    label.textContent = window.label;

    const meter = document.createElement("div");
    meter.className = "meter";
    meter.setAttribute("role", "progressbar");
    meter.setAttribute("aria-valuenow", String(Math.round(window.used_percent)));
    meter.setAttribute("aria-valuemin", "0");
    meter.setAttribute("aria-valuemax", "100");
    meter.setAttribute("aria-label", `${name} ${window.label} usage`);
    meter.setAttribute("aria-valuetext", `${Math.round(window.used_percent)} percent used, ${formatReset(window.label, window.resets_at, now)}`);
    const percent = Math.min(100, Math.max(0, window.used_percent));
    meter.dataset.provider = root.dataset.provider;
    meter.dataset.label = window.label;
    meter.dataset.resetsAt = String(window.resets_at);
    meter.dataset.shape = shape;
    meter.style.setProperty("--progress-offset", progressOffset(percent));
    meter.style.setProperty("--progress-percent", progressPercent(percent));
    meter.style.setProperty("--progress-percent-number", String(percent));
    renderPace(meter, window);
    const previousWindow = previous?.windows.find((candidate) => candidate.label === window.label);
    if (previousWindow && previousWindow.used_percent !== window.used_percent) {
      meter.dataset.usageChange = window.used_percent > previousWindow.used_percent ? "increase" : "decrease";
      meter.style.setProperty("--previous-progress-offset", progressOffset(previousWindow.used_percent));
    }

    meter.append(...buildMeterBody(shape));
    updateReactorSegments(meter, percent);

    const value = document.createElement("span");
    value.className = "meter__value";
    value.textContent = formatPercent(window.used_percent);
    meter.appendChild(value);

    const reset = document.createElement("span");
    reset.className = "window-card__reset";
    reset.dataset.label = window.label;
    reset.dataset.resetsAt = String(window.resets_at);
    reset.textContent = formatReset(window.label, window.resets_at, now);

    card.append(meter, label, reset);
    grid.appendChild(card);
  }
  if (snapshot.windows.length) root.appendChild(grid);

  // Below the 5 hour / Weekly grid, never inside it: extra credit is money against a monthly
  // budget, not another rolling usage window, and pairing it with them in the same row would
  // read as a third limit of the same kind.
  const extra = extraOf(snapshot);
  if (extra) root.appendChild(renderExtraCredit(extra));

  const hint = hintText(snapshot.state, name);
  if (hint) root.appendChild(createHintButton(hint, providerKeyFromName(name), onAction));

  return root;
}
