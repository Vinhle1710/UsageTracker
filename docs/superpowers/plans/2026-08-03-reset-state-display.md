# Reset State & Weekly Display Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the ring stuck at 100% after reset, replace "reset time unavailable" with fun placeholder messages, and display weekly reset countdowns instead of a fixed time-of-day.

**Architecture:**
- Add countdown formatting utilities to `format.ts` with per-window message caching on DOM elements.
- Export `progressOffset` from `layer.ts` so `main.ts` can apply optimistic 0% resets.
- Extend `updateCountdowns()` in `main.ts` to detect timeout crossings, apply the optimistic reset, swap reset text to a fun message, and clear cached messages when real data arrives.
- All changes follow TDD: write the failing test first, then the minimal implementation.

**Tech Stack:** TypeScript, Vitest, DOM manipulation via `dataset` caching.

---

## File Structure

| File | Responsibility |
|------|-----------------|
| `src/format.ts` | Countdown formatting, fun message pool, cached message selection |
| `src/format.test.ts` | Tests for countdown and weekly display formatting |
| `src/components/layer.ts` | Export `progressOffset()` for reuse by main.ts, clear message cache on real data |
| `src/components/layer.test.ts` | Tests for `progressOffset` export and message cache clearing on data update |
| `src/main.ts` | Extend `updateCountdowns()` to handle optimistic reset and fun messages |

---

## Task 1: Add countdown formatting for weekly resets

**Files:**
- Modify: `src/format.ts`
- Modify: `src/format.test.ts`

- [ ] **Step 1: Write failing test for weekly countdown format (under 24h)**

Open `src/format.test.ts` and add this test to the `describe("formatReset")` block:

```typescript
it("shows a short countdown for weekly windows under 24 hours", () => {
  const now = 1_000_000;
  const resetsIn2Hours = now + 7200; // 2 hours
  expect(formatReset("Weekly", resetsIn2Hours, now)).toBe("Aug 08 · 02:00:00");
});
```

Run: `npm test -- src/format.test.ts -t "short countdown"`
Expected: FAIL (current implementation still returns the old `"Aug 08 · 14:30"`-style text)

- [ ] **Step 2: Write failing test for weekly countdown format (over 24h)**

Add this test to the same block:

```typescript
it("shows a day-prefixed countdown for weekly windows over 24 hours", () => {
  const now = 1_000_000;
  const resetsIn3Days = now + 259200; // 3 days
  expect(formatReset("Weekly", resetsIn3Days, now)).toBe("Aug 08 · 3d 00:00:00");
});
```

Run: `npm test -- src/format.test.ts`
Expected: FAIL

- [ ] **Step 3: Implement `formatCountdownUntilReset()`**

Add this function to `src/format.ts`, directly above `formatWeeklyReset`:

```typescript
export function formatCountdownUntilReset(resetsAt: number, now: number): string {
  const secondsUntil = Math.max(0, Math.floor(resetsAt - now));
  const days = Math.floor(secondsUntil / 86400);
  const hours = Math.floor((secondsUntil % 86400) / 3600);
  const minutes = Math.floor((secondsUntil % 3600) / 60);
  const seconds = secondsUntil % 60;
  const clock = [hours, minutes, seconds].map((value) => String(value).padStart(2, "0")).join(":");
  return days > 0 ? `${days}d ${clock}` : clock;
}
```

- [ ] **Step 4: Update `formatWeeklyReset()` to drop the time-of-day and update `formatReset()` to append the live countdown**

Replace the entire `formatWeeklyReset` and `formatReset` functions in `src/format.ts` with:

```typescript
export function formatWeeklyReset(resetsAt: number): string {
  const date = new Date(resetsAt * 1000);
  const month = date.toLocaleString("en-US", { month: "short" });
  const day = String(date.getDate()).padStart(2, "0");
  return `${month} ${day}`;
}

export function formatReset(label: string, resetsAt: number, now: number): string {
  if (!Number.isFinite(resetsAt) || resetsAt <= 0) return "reset time unavailable";
  if (/(hour|min)/i.test(label)) return `resets in ${formatCountdown(resetsAt - now)}`;
  return `${formatWeeklyReset(resetsAt)} · ${formatCountdownUntilReset(resetsAt, now)}`;
}
```

Note: the `formatReset` fallback text ("reset time unavailable") is intentionally left as-is here — Task 2 replaces it with the fun message pool, and Task 5 wires that into the live countdown ticker. Keeping this task scoped to the weekly-display change only avoids conflating the two behaviors in one diff.

- [ ] **Step 5: Run tests to verify both new tests pass**

Run: `npm test -- src/format.test.ts -t "countdown"`
Expected: PASS for both "short countdown" and "day-prefixed countdown" tests

- [ ] **Step 6: Update the existing weekly-format test to match the new display**

In `src/format.test.ts`, find:

```typescript
it("uses a date and time for weekly windows", () => expect(formatReset("Weekly", 1_754_665_800, now)).toContain("Aug"));
```

Replace it with:

```typescript
it("uses a date and a live countdown for weekly windows", () => {
  const resetTime = now + 86400 + 3600; // 1 day + 1 hour away
  expect(formatReset("Weekly", resetTime, now)).toMatch(/^[A-Z][a-z]{2} \d{2} · 1d \d{2}:\d{2}:\d{2}$/);
});
```

Also update the `formatWeeklyReset` describe block, which currently asserts the old `"Aug 08 · 14:30"` format:

```typescript
describe("formatWeeklyReset", () => {
  it("includes the reset month, day, and local time", () => {
    const reset = new Date(2025, 7, 8, 14, 30).getTime() / 1000;
    expect(formatWeeklyReset(reset)).toBe("Aug 08 · 14:30");
  });
});
```

Replace with:

```typescript
describe("formatWeeklyReset", () => {
  it("includes only the reset month and day, no time-of-day", () => {
    const reset = new Date(2025, 7, 8, 14, 30).getTime() / 1000;
    expect(formatWeeklyReset(reset)).toBe("Aug 08");
  });
});
```

- [ ] **Step 7: Run all format tests to ensure no regressions**

Run: `npm test -- src/format.test.ts`
Expected: All tests PASS

- [ ] **Step 8: Commit**

```bash
git add src/format.ts src/format.test.ts
git commit -m "feat: show a live countdown for weekly reset windows"
```

---

## Task 2: Add fun placeholder messages pool

**Files:**
- Modify: `src/format.ts`
- Modify: `src/format.test.ts`

- [ ] **Step 1: Write failing test for placeholder message selection**

Add to `src/format.test.ts`, after the `formatReset` describe block:

```typescript
describe("getFunPlaceholder", () => {
  it("returns a non-empty string from the fun message pool", () => {
    const message = getFunPlaceholder();
    expect(typeof message).toBe("string");
    expect(message.length).toBeGreaterThan(0);
  });

  it("returns more than one distinct message across many calls", () => {
    const messages = new Set<string>();
    for (let i = 0; i < 50; i += 1) messages.add(getFunPlaceholder());
    expect(messages.size).toBeGreaterThan(1);
  });
});
```

Run: `npm test -- src/format.test.ts -t "getFunPlaceholder"`
Expected: FAIL (`getFunPlaceholder` is not exported)

- [ ] **Step 2: Implement `getFunPlaceholder()`**

Add this to `src/format.ts`, above `formatReset`:

```typescript
const FUN_RESET_MESSAGES = [
  "Recharging the quota…",
  "Reticulating tokens…",
  "Politely waiting its turn…",
  "Catching its breath…",
  "Warming up the limiter…",
  "Syncing with the mothership…",
  "Consulting the oracle…",
  "Running the cosmic clock…",
];

export function getFunPlaceholder(): string {
  return FUN_RESET_MESSAGES[Math.floor(Math.random() * FUN_RESET_MESSAGES.length)];
}
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `npm test -- src/format.test.ts -t "getFunPlaceholder"`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/format.ts src/format.test.ts
git commit -m "feat: add fun placeholder message pool for the reset-pending state"
```

---

## Task 3: Export `progressOffset()` from layer.ts for reuse

**Files:**
- Modify: `src/components/layer.ts`
- Modify: `src/components/layer.test.ts`

- [ ] **Step 1: Write failing test for the exported `progressOffset`**

Add to the top of `src/components/layer.test.ts` (alongside existing imports from `./layer`) and add a new describe block:

```typescript
import { progressOffset } from "./layer";
```

```typescript
describe("progressOffset", () => {
  it("returns the full ring length at 0%", () => expect(progressOffset(0)).toBe("276.46"));
  it("returns zero offset at 100%", () => expect(progressOffset(100)).toBe("0"));
  it("clamps values below 0", () => expect(progressOffset(-10)).toBe("276.46"));
  it("clamps values above 100", () => expect(progressOffset(150)).toBe("0"));
});
```

Run: `npm test -- src/components/layer.test.ts -t "progressOffset"`
Expected: FAIL (`progressOffset` is not exported from `layer.ts`)

- [ ] **Step 2: Export `progressOffset()` from layer.ts**

In `src/components/layer.ts`, change:

```typescript
function progressOffset(percent: number): string {
```

to:

```typescript
export function progressOffset(percent: number): string {
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `npm test -- src/components/layer.test.ts -t "progressOffset"`
Expected: PASS

- [ ] **Step 4: Commit**

```bash
git add src/components/layer.ts src/components/layer.test.ts
git commit -m "refactor: export progressOffset from layer.ts for reuse in main.ts"
```

---

## Task 4: Clear the cached fun message when real reset data arrives

**Files:**
- Modify: `src/components/layer.ts`
- Modify: `src/components/layer.test.ts`

- [ ] **Step 1: Write failing test for message cache clearing**

Add to `src/components/layer.test.ts`. This test builds the minimal DOM shape `updateMeter` expects (a `.window-card` containing a `.meter` and a `.window-card__reset`), seeds a stale cached message on the reset span the same way `main.ts` will (Task 5), and asserts `updateMeter` clears it once a fresh `resets_at` comes in:

```typescript
import { updateMeter } from "./layer";
```

(Note: `updateMeter` is not currently exported — see Step 2.)

```typescript
describe("updateMeter cache clearing", () => {
  it("clears a cached fun message once a new resets_at value arrives", () => {
    const card = document.createElement("div");
    card.className = "window-card";

    const meter = document.createElement("div");
    meter.className = "meter";
    meter.dataset.label = "5 hour";
    meter.dataset.resetsAt = "2000000";
    const value = document.createElement("span");
    value.className = "meter__value";
    meter.appendChild(value);

    const reset = document.createElement("span");
    reset.className = "window-card__reset";
    reset.dataset.label = "5 hour";
    reset.dataset.resetsAt = "2000000"; // stale value from before the reset
    reset.dataset.cachedMessage = "Recharging the quota…";
    reset.textContent = "Recharging the quota…";

    card.append(meter, reset);

    updateMeter(meter, "Claude", { label: "5 hour", used_percent: 4, resets_at: 2_100_000 }, 1_000_000);

    expect(reset.dataset.cachedMessage).toBeUndefined();
    expect(reset.dataset.resetsAt).toBe("2100000");
  });

  it("keeps the cached fun message when resets_at has not changed yet", () => {
    const card = document.createElement("div");
    card.className = "window-card";

    const meter = document.createElement("div");
    meter.className = "meter";
    meter.dataset.label = "5 hour";
    meter.dataset.resetsAt = "2000000";
    const value = document.createElement("span");
    value.className = "meter__value";
    meter.appendChild(value);

    const reset = document.createElement("span");
    reset.className = "window-card__reset";
    reset.dataset.label = "5 hour";
    reset.dataset.resetsAt = "2000000";
    reset.dataset.cachedMessage = "Recharging the quota…";
    reset.textContent = "Recharging the quota…";

    card.append(meter, reset);

    // Backend still reports the same (now-elapsed) resets_at — no fresh data yet.
    updateMeter(meter, "Claude", { label: "5 hour", used_percent: 0, resets_at: 2_000_000 }, 2_500_000);

    expect(reset.dataset.cachedMessage).toBe("Recharging the quota…");
  });
});
```

Run: `npm test -- src/components/layer.test.ts -t "cache clearing"`
Expected: FAIL (`updateMeter` is not exported)

- [ ] **Step 2: Export `updateMeter()` and clear the cache on a real resets_at change**

In `src/components/layer.ts`, change the function declaration from:

```typescript
function updateMeter(meter: HTMLElement, name: string, window: UsageWindow, now: number): void {
```

to:

```typescript
export function updateMeter(meter: HTMLElement, name: string, window: UsageWindow, now: number): void {
```

Then, inside `updateMeter`, find this block:

```typescript
  const reset = meter.closest<HTMLElement>(".window-card")?.querySelector<HTMLElement>(".window-card__reset");
  if (reset) {
    reset.dataset.resetsAt = String(window.resets_at);
    reset.textContent = resetText;
  }
```

Replace it with:

```typescript
  const reset = meter.closest<HTMLElement>(".window-card")?.querySelector<HTMLElement>(".window-card__reset");
  if (reset) {
    if (reset.dataset.resetsAt !== String(window.resets_at)) delete reset.dataset.cachedMessage;
    reset.dataset.resetsAt = String(window.resets_at);
    reset.textContent = reset.dataset.cachedMessage ?? resetText;
  }
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `npm test -- src/components/layer.test.ts -t "cache clearing"`
Expected: PASS (both cases)

- [ ] **Step 4: Run all layer tests to ensure no regressions**

Run: `npm test -- src/components/layer.test.ts`
Expected: All tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/components/layer.ts src/components/layer.test.ts
git commit -m "feat: clear cached fun message once a fresh resets_at value arrives"
```

---

## Task 5: Extend `updateCountdowns()` for optimistic reset and fun messages

**Files:**
- Modify: `src/main.ts`

- [ ] **Step 1: Update imports at the top of main.ts**

Find:

```typescript
import { formatReset } from "./format";
```

Replace with:

```typescript
import { formatReset, getFunPlaceholder } from "./format";
```

Find the `reconcileProviderLayers` import line:

```typescript
import { reconcileProviderLayers } from "./components/overlay";
```

Add a new import directly below it:

```typescript
import { progressOffset } from "./components/layer";
```

- [ ] **Step 2: Replace `updateCountdowns()` with the extended version**

Find the existing `updateCountdowns` function (currently around line 93):

```typescript
function updateCountdowns(): void {
  if (isSettingsWindow) return;
  const currentNow = now();
  app.querySelectorAll<HTMLElement>(".window-card__reset").forEach((reset) => {
    const label = reset.dataset.label;
    const resetsAt = Number(reset.dataset.resetsAt);
    if (!label || !Number.isFinite(resetsAt)) return;
    reset.textContent = formatReset(label, resetsAt, currentNow);
    const meter = reset.closest<HTMLElement>(".window-card")?.querySelector<HTMLElement>(".meter");
    if (!meter || resetsAt > currentNow) return;
    const provider = meter.dataset.provider ?? "unknown";
    const key = `${provider}:${label}:${resetsAt}`;
    if (handledResets.has(key)) return;
    handledResets.add(key);
    meter.classList.add("meter--resetting");
    window.setTimeout(() => meter.classList.remove("meter--resetting"), 850);
  });
}
```

Replace it with:

```typescript
function updateCountdowns(): void {
  if (isSettingsWindow) return;
  const currentNow = now();
  app.querySelectorAll<HTMLElement>(".window-card__reset").forEach((reset) => {
    const label = reset.dataset.label;
    const resetsAt = Number(reset.dataset.resetsAt);
    if (!label || !Number.isFinite(resetsAt)) return;
    const meter = reset.closest<HTMLElement>(".window-card")?.querySelector<HTMLElement>(".meter");

    if (reset.dataset.cachedMessage) {
      reset.textContent = reset.dataset.cachedMessage;
      return;
    }

    if (resetsAt > currentNow) {
      reset.textContent = formatReset(label, resetsAt, currentNow);
      return;
    }

    // The countdown just reached (or already passed) zero: apply the optimistic
    // reset once per (provider, label, resetsAt) triple, same key the pulse animation used.
    const provider = meter?.dataset.provider ?? "unknown";
    const key = `${provider}:${label}:${resetsAt}`;
    const funMessage = reset.dataset.cachedMessage ?? getFunPlaceholder();
    reset.dataset.cachedMessage = funMessage;
    reset.textContent = funMessage;

    if (!meter || handledResets.has(key)) return;
    handledResets.add(key);
    meter.style.setProperty("--progress-offset", progressOffset(0));
    const value = meter.querySelector<HTMLElement>(".meter__value");
    if (value) value.textContent = "0%";
    meter.setAttribute("aria-valuenow", "0");
    meter.setAttribute("aria-valuetext", `0 percent used, ${funMessage}`);
    meter.classList.add("meter--resetting");
    window.setTimeout(() => meter.classList.remove("meter--resetting"), 850);
  });
}
```

- [ ] **Step 3: Run the full test suite**

Run: `npm test`
Expected: All tests PASS (no regressions in `main.ts`-adjacent behavior; `main.ts` itself has no dedicated unit test file today, so this step is a safety net for everything else)

- [ ] **Step 4: Manually verify in the dev/preview build**

Run: `npm run dev`

In the browser preview, open devtools and run this in the console to simulate a window about to expire in 2 seconds:

```js
document.querySelectorAll(".window-card__reset").forEach((el) => {
  el.dataset.resetsAt = String(Math.floor(Date.now() / 1000) + 2);
});
```

Expected: after ~2 seconds, the ring for each meter animates down to 0%, the `%` label reads "0%", and the reset line switches to one of the fun placeholder messages instead of "resets in 00:00:00" staying frozen or "reset time unavailable" appearing.

- [ ] **Step 5: Commit**

```bash
git add src/main.ts
git commit -m "feat: optimistically reset the ring to 0% and show a fun message when a countdown expires"
```

---

## Self-Review

**Spec coverage:**
- Ring stuck at 100% after reset → Task 5 (optimistic 0% reset, reusing the existing `bubbly-reset` pulse via `.meter--resetting`).
- "reset time unavailable" replaced with a fun, punny message → Task 2 (message pool) + Task 5 (wiring into the live ticker).
- Message doesn't flicker every second while pending → Task 4 (cache on `reset.dataset.cachedMessage`, read back in Task 5's `updateCountdowns`).
- Cache clears once real data lands → Task 4's `updateMeter` change.
- Weekly windows show a live countdown instead of a fixed HH:MM, keeping the `·` separator → Task 1 (`formatWeeklyReset` drops the time, `formatCountdownUntilReset` adds the live duration, under-24h shows `HH:MM:SS`, over-24h prefixes `Nd `).
- `progressOffset` reuse without duplicating the ring math → Task 3.

**Placeholder scan:** No "TBD"/"TODO"/"implement later" markers; every step has complete, runnable code.

**Type consistency check:**
- `formatCountdownUntilReset(resetsAt: number, now: number): string` — defined in Task 1, used only internally by `formatReset` in the same file.
- `getFunPlaceholder(): string` — defined in Task 2, imported and called in Task 5 (`main.ts`) and referenced conceptually (not directly imported) by Task 4's test fixtures.
- `progressOffset(percent: number): string` — exported in Task 3, imported in Task 5 (`main.ts`).
- `updateMeter(meter: HTMLElement, name: string, window: UsageWindow, now: number): void` — exported in Task 4, matches the existing internal signature exactly (only the `export` keyword and the cache-clear lines change), and the tests in Task 4 call it with the same 4 positional arguments used internally by `updateLayer`.
- `reset.dataset.cachedMessage` — the one piece of shared state across Task 4 (`layer.ts`) and Task 5 (`main.ts`); both read/write the same `dataset` key on the same `.window-card__reset` element, so no mismatch between "cache" naming in one file vs. the other.

**Scope check:** This plan covers one cohesive subsystem (reset-state display: ring, percent text, and reset label) across 5 small, independently testable tasks. No further decomposition needed.
