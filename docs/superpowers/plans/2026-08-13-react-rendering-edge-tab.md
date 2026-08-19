# React Rendering Foundation and Screen-Edge Tab Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the imperative UI with React while preserving behavior, move visible curves entirely to CSS, and add independently configurable tray, overlay, and edge-tab surfaces.

**Architecture:** This is the prerequisite branch; create it from the current baseline and merge it before every later feature branch. A framework-neutral external store feeds three React roots selected by Tauri window label, while controllers retain native subscriptions, geometry, morphing, and visibility side effects; Rust remains authoritative for configuration and window state.

**Tech Stack:** React 19, TypeScript 5.6, Vite 6, Vitest/jsdom/vitest-axe, Tauri 2, Rust, Win32/WebView2

---

### Task 1: Install React and establish typed window routing

**Files:**
- Modify: `package.json`
- Modify: `vite.config.ts`
- Create: `src/app/window-root.tsx`
- Create: `src/app/window-root.test.tsx`
- Create: `src/app/roots.tsx`
- Modify: `src/main.ts`

- [ ] **Step 1: Write the failing routing test**

```tsx
import { describe, expect, it } from "vitest";
import { rootForWindow } from "./window-root";

describe("rootForWindow", () => {
  it.each([["main", "overlay"], ["settings", "settings"], ["edge-tab", "edge-tab"]] as const)("routes %s", (label, expected) => {
    expect(rootForWindow(label)).toBe(expected);
  });
  it("rejects an unknown label", () => expect(() => rootForWindow("other")).toThrow("Unsupported window label: other"));
});
```

- [ ] **Step 2: Verify red**

Run: `npm test -- src/app/window-root.test.tsx`
Expected: FAIL because `src/app/window-root.tsx` does not exist.

- [ ] **Step 3: Add dependencies and the router**

Add `react` and `react-dom` at `^19`, plus `@types/react`, `@types/react-dom`, and `@testing-library/react` as dev dependencies, then run `npm install`. Create:

```tsx
export type WindowRoot = "overlay" | "settings" | "edge-tab";
export function rootForWindow(label: string): WindowRoot {
  if (label === "main") return "overlay";
  if (label === "settings") return "settings";
  if (label === "edge-tab") return "edge-tab";
  throw new Error(`Unsupported window label: ${label}`);
}
```

In `roots.tsx`, export initially testable roots:

```tsx
export const OverlayApp = () => <main id="app" data-window="overlay" />;
export const SettingsApp = () => <main id="app" data-window="settings" />;
export const EdgeTabApp = () => <main id="app" data-window="edge-tab" />;
```

Replace the boot block in `main.ts` with `createRoot(document.getElementById("app")!).render(<StrictMode>{root === ...}</StrictMode>)`, using `getCurrentWindow().label` and `rootForWindow`. Rename it to `main.tsx`, update `index.html`, and enable `jsx: "react-jsx"` in `tsconfig.json`.

- [ ] **Step 4: Verify green and build**

Run: `npm test -- src/app/window-root.test.tsx && npm run build`
Expected: routing tests PASS and production build succeeds.

- [ ] **Step 5: Commit**

```bash
git add package.json package-lock.json tsconfig.json index.html src/main.tsx src/app
git commit -m "feat: establish React window roots"
```

### Task 2: Add the immutable external application store

**Files:**
- Create: `src/app/store.ts`
- Create: `src/app/store.test.ts`
- Modify: `src/types.ts`

- [ ] **Step 1: Write the failing store test**

```ts
import { expect, it, vi } from "vitest";
import { createAppStore } from "./store";

it("publishes immutable bootstrap and usage snapshots", () => {
  const store = createAppStore(); const listener = vi.fn(); store.subscribe(listener);
  store.dispatch({ type: "bootstrap", payload: { sources: { claude: true, openai: false }, usage: [] } });
  const first = store.getSnapshot();
  store.dispatch({ type: "usage", payload: { provider: "claude", snapshot: { windows: [], fetched_at: 1, state: "fresh" } } });
  expect(store.getSnapshot()).not.toBe(first); expect(listener).toHaveBeenCalledTimes(2);
});
```

- [ ] **Step 2: Verify red**

Run: `npm test -- src/app/store.test.ts`
Expected: FAIL with missing `createAppStore`.

- [ ] **Step 3: Implement the store contract**

```ts
export type AppAction =
  | { type: "bootstrap"; payload: BootstrapPayload }
  | { type: "usage"; payload: ProviderUsageEvent }
  | { type: "sources"; payload: ActiveSources }
  | { type: "config"; payload: Config }
  | { type: "collapsed"; provider: Provider; collapsed: boolean };

export function createAppStore(initial = initialAppSnapshot()): AppStore {
  let snapshot = initial; const listeners = new Set<() => void>();
  return { getSnapshot: () => snapshot, subscribe: listener => (listeners.add(listener), () => listeners.delete(listener)),
    dispatch(action) { snapshot = reduceAppSnapshot(snapshot, action); listeners.forEach(listener => listener()); } };
}
```

Define `AppSnapshot` in `types.ts` with canonical `config`, `sources`, `providers`, and `visibility`; implement `reduceAppSnapshot` by calling existing pure functions from `state.ts`, never mutating prior objects. Export `useAppSnapshot(store)` using `useSyncExternalStore`.

- [ ] **Step 4: Verify green**

Run: `npm test -- src/app/store.test.ts src/state.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/app/store.ts src/app/store.test.ts src/types.ts
git commit -m "feat: add immutable React application store"
```

### Task 3: Migrate overlay and settings without losing stable nodes

**Files:**
- Create: `src/app/overlay/OverlayApp.tsx`
- Create: `src/app/overlay/ProviderLayer.tsx`
- Create: `src/app/settings/SettingsApp.tsx`
- Create: `src/app/overlay/OverlayApp.test.tsx`
- Create: `src/app/settings/SettingsApp.test.tsx`
- Modify: `src/app/roots.tsx`
- Delete: `src/components/overlay.ts`, `src/components/layer.ts`, `src/components/controls.ts`, `src/components/settings.ts`

- [ ] **Step 1: Write failing parity tests**

```tsx
it("keeps a provider node when only usage changes", () => {
  const { rerender } = render(<OverlayApp snapshot={fixture(20)} />);
  const node = screen.getByTestId("provider-claude");
  rerender(<OverlayApp snapshot={fixture(21)} />);
  expect(screen.getByTestId("provider-claude")).toBe(node);
});

it("prevents disabling the last presentation surface", async () => {
  render(<SettingsApp config={{ ...defaultConfig, showTrayIndicator: true, showScreenOverlay: false }} />);
  expect(screen.getByRole("checkbox", { name: "Show tray indicator" })).toBeDisabled();
});
```

- [ ] **Step 2: Verify red**

Run: `npm test -- src/app/overlay/OverlayApp.test.tsx src/app/settings/SettingsApp.test.tsx`
Expected: FAIL because React apps are absent.

- [ ] **Step 3: Implement keyed semantic components**

`OverlayApp` must map `visibleLayers(snapshot.sources)` to `<ProviderLayer key={provider} data-testid={...}>`; meters use `role="progressbar"`, `aria-valuenow`, and existing `formatReset`. Buttons dispatch `{ action: "minimize" | "restore" | "open-settings", provider }`. `SettingsApp` uses controlled inputs and this invariant:

```tsx
const lastSurface = config.showTrayIndicator !== config.showScreenOverlay;
<input aria-label="Show tray indicator" type="checkbox" checked={config.showTrayIndicator}
  disabled={lastSurface && config.showTrayIndicator} onChange={e => onChange({ ...config, showTrayIndicator: e.currentTarget.checked })} />
<input aria-label="Show screen overlay" type="checkbox" checked={config.showScreenOverlay}
  disabled={lastSurface && config.showScreenOverlay} onChange={e => onChange({ ...config, showScreenOverlay: e.currentTarget.checked })} />
```

Port every account action, page, custom select, loading/error/signed-out hint, provider logo, reset countdown, live status announcement, and focus handoff represented in existing component tests before deleting the imperative files. Convert tests to Testing Library assertions and keep all provider keys stable.

- [ ] **Step 4: Verify green, accessibility, and coverage**

Run: `npm test && npm run coverage`
Expected: all migrated tests PASS and line/function/branch thresholds remain at least 80%.

- [ ] **Step 5: Commit**

```bash
git add src/app src/components src/a11y.test.ts
git commit -m "feat: migrate overlay and settings to React"
```

### Task 4: Extract Strict-Mode-safe controllers

**Files:**
- Create: `src/controllers/usage-controller.ts`
- Create: `src/controllers/settings-controller.ts`
- Create: `src/controllers/geometry-controller.ts`
- Create: `src/controllers/morph-controller.ts`
- Create: `src/controllers/controllers.test.ts`
- Modify: `src/app/overlay/OverlayApp.tsx`
- Modify: `src/app/settings/SettingsApp.tsx`

- [ ] **Step 1: Write the failing lifecycle test**

```ts
it("start and stop are idempotent", async () => {
  const unlisten = vi.fn(); const listen = vi.fn().mockResolvedValue(unlisten);
  const controller = new UsageController(store, { listen, invoke: vi.fn() });
  await controller.start(); await controller.start(); await controller.stop(); await controller.stop();
  expect(listen).toHaveBeenCalledTimes(3); expect(unlisten).toHaveBeenCalledTimes(3);
});
```

- [ ] **Step 2: Verify red**

Run: `npm test -- src/controllers/controllers.test.ts`
Expected: FAIL with missing controllers.

- [ ] **Step 3: Implement symmetric lifecycle and retained imperative behavior**

Each controller owns `private started = false` and an `unlisteners` array. `start()` invokes bootstrap and subscribes exactly once; `stop()` awaits and clears every unlistener. `GeometryController` wraps the existing `GeometryRequestScheduler`, measures refs in `useLayoutEffect`, and sends `{ cards, bubbles, effectOutset: 0 }`. `MorphController` ports `runMorph`, FLIP siblings, timeout race, and a `finally` that removes ghosts and clears destination opacity; `cancelAll()` is called before full-overlay hide.

- [ ] **Step 4: Verify green under Strict Mode**

Run: `npm test -- src/controllers src/app/overlay && npm run build`
Expected: PASS; StrictMode mount replay produces no duplicate subscriptions or commands.

- [ ] **Step 5: Commit**

```bash
git add src/controllers src/app
git commit -m "refactor: isolate React side effects in controllers"
```

### Task 5: Replace rounded GDI clipping with rectangular containment coverage

**Files:**
- Modify: `src/geometry.ts`
- Modify: `src/geometry.test.ts`
- Modify: `src-tauri/src/material.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Add failing Rust containment tests**

```rust
#[test]
fn physical_coverage_contains_fractional_logical_surface() {
    let result = physical_card_regions(&[LogicalCardRegion { x: 0.25, y: 1.25, width: 47.5, height: 47.5, radius: 24.0, effect_outset: 0.0 }], 1.25);
    assert_eq!(result[0], CardRegion { x: 0, y: 1, width: 60, height: 60 });
}
#[test]
fn surface_regions_are_rectangular_and_keep_disjoint_gaps() {
    assert_eq!(region_shape_kind(), RegionShapeKind::Rectangles);
}
```

- [ ] **Step 2: Verify red**

Run: `cargo test --manifest-path src-tauri/Cargo.toml material::tests::physical_coverage_contains_fractional_logical_surface`
Expected: FAIL because `effect_outset` and rectangular shape contract are absent.

- [ ] **Step 3: Implement containment conversion**

Add `effect_outset: f64` to `LogicalCardRegion`; compute `left/top = floor((origin-outset)*scale)` and `right/bottom = ceil((origin+size+outset)*scale)`, validate finite values and checked integer differences, and return the last known-good region on invalid input. In `apply_card_region`, replace every `CreateRoundRectRgn` call with `CreateRectRgn(region.x, region.y, right, bottom)` and keep `CombineRgn(..., RGN_OR)`. Keep transient morph coverage rectangular and settings coverage inset.

- [ ] **Step 4: Verify all scale factors**

Run: `cargo test --manifest-path src-tauri/Cargo.toml material && npm test -- src/geometry.test.ts`
Expected: PASS at 1.0, 1.25, 1.5, and 2.0 scale; invalid and overflow cases retain cached coverage.

- [ ] **Step 5: Commit**

```bash
git add src/geometry.ts src/geometry.test.ts src-tauri/src/material.rs src-tauri/src/lib.rs
git commit -m "fix: preserve CSS antialiasing with rectangular coverage"
```

### Task 6: Add presentation config and the edge-tab visibility state machine

**Files:**
- Modify: `src/types.ts`
- Modify: `src-tauri/src/config.rs`
- Modify: `src-tauri/src/visibility.rs`
- Modify: `src-tauri/src/window.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/tauri.conf.json`
- Create: `src/app/edge-tab/EdgeTabApp.tsx`
- Create: `src/app/edge-tab/EdgeTabApp.test.tsx`

- [ ] **Step 1: Write failing config, placement, and arrow tests**

```rust
#[test] fn config_never_disables_both_surfaces() {
  let c = Config { show_tray_indicator: false, show_screen_overlay: false, ..Default::default() }.sanitized();
  assert!(c.show_tray_indicator); assert!(!c.show_screen_overlay);
}
#[test] fn left_bottom_tab_uses_work_area_edge() {
  assert_eq!(edge_tab_position(Rect { x: 10, y: 20, width: 1000, height: 800 }, (24, 48), "bottom-left"), (10, 760));
}
```

```tsx
it("labels the action and reverses the arrow", () => {
  const { rerender } = render(<EdgeTabApp side="right" hidden={false} reducedMotion={false} onToggle={vi.fn()} />);
  expect(screen.getByRole("button", { name: "Hide usage overlay" })).toHaveAttribute("data-direction", "right");
  rerender(<EdgeTabApp side="right" hidden reducedMotion={false} onToggle={vi.fn()} />);
  expect(screen.getByRole("button", { name: "Show usage overlay" })).toHaveAttribute("data-direction", "left");
});
```

- [ ] **Step 2: Verify red**

Run: `cargo test --manifest-path src-tauri/Cargo.toml config_never_disables_both_surfaces && npm test -- src/app/edge-tab`
Expected: FAIL because fields, placement, and component are absent.

- [ ] **Step 3: Implement the full contract**

Add serde-defaulted `show_tray_indicator: bool` and `show_screen_overlay: bool`; sanitize both false to tray true. Add a transparent, undecorated, 24x48 logical `edge-tab` window in `tauri.conf.json`. Define in `visibility.rs`:

```rust
pub struct OverlayVisibilityState { pub enabled: bool, pub provider_available: bool, pub user_hidden: bool,
  pub generation: u64, pub phase: AnimationPhase, pub stable_position: (i32, i32) }
pub enum AnimationPhase { Stable, Hiding, Revealing }
```

Implement idempotent `request_hidden(bool)` by incrementing generation, sampling current position, animating position+opacity for 200ms, checking generation on each frame, and always settling canonical position/full opacity plus requested native visibility. Expose `toggle_overlay_visibility`, `get_overlay_visibility`, and `visibility-changed`. Create/reposition the tab from the selected monitor work area; if tab creation fails while tray is disabled, create the tray for that process. The React button is 24x48, keyboard-native, internally focused, rotates its decorative SVG 180 degrees, and disables transition under `prefers-reduced-motion`.

- [ ] **Step 4: Verify interruption and fallback**

Run: `npm test && cargo test --manifest-path src-tauri/Cargo.toml && npm run build && cargo build --manifest-path src-tauri/Cargo.toml`
Expected: all checks PASS, including reversal, failure settlement, monitor fallback, and all three presentation configurations.

- [ ] **Step 5: Commit**

```bash
git add src src-tauri
git commit -m "feat: add independent edge-tab overlay control"
```

### Task 7: Perform packaged Windows acceptance verification

**Files:**
- Create: `docs/qa/react-rendering-edge-tab-windows.md`

- [ ] **Step 1: Run automated release checks**

Run: `npm test && npm run coverage && npm run tauri build`
Expected: all tests and coverage thresholds PASS and a packaged Windows executable is produced.

- [ ] **Step 2: Record the complete live matrix**

Create the QA file with checked rows for 100%, 125%, 150%, and 200% scaling on light/dark backgrounds; all four corners; tray-only/overlay-only/both; mouse/keyboard/reduced motion; rapid reversal; top/left/right/bottom taskbars; monitor disconnect/reconnect; no stair-stepped curves; disjoint gaps click through; hidden HWND does not block the desktop; settings frame remains repaired.

- [ ] **Step 3: Commit**

```bash
git add docs/qa/react-rendering-edge-tab-windows.md
git commit -m "test: document edge-tab Windows acceptance matrix"
```

