# Configurable Display System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add used/remaining semantics, glow styling, five compact indicator styles, multi-metric composition, configurable color modes, automatic contrast adaptation, and live previews.

**Architecture:** Create `codex/feature-display-system` only after merging `codex/feature-react-rendering-foundation`. Keep display preferences as sanitized Rust-owned config, derive a pure serializable display model in TypeScript, and render that same model through reusable React primitives for the overlay, tray bitmap, and Settings previews.

**Tech Stack:** React 19, TypeScript, Canvas 2D/ImageData, Vitest/Testing Library, Tauri 2, Rust

---

### Task 1: Define and persist the display schema

**Files:**
- Modify: `src/types.ts`
- Modify: `src-tauri/src/config.rs`
- Create: `src/display/config.test.ts`

- [ ] **Step 1: Write failing migration tests**

```rust
#[test] fn display_defaults_are_backward_compatible() {
 let c: Config = serde_json::from_str("{}").unwrap();
 assert_eq!(c.value_mode, "used"); assert_eq!(c.indicator_style, "compact");
 assert_eq!(c.color_mode, "multicolor"); assert_eq!(c.metric_order, vec!["session", "weekly", "api"]);
}
#[test] fn invalid_display_values_are_sanitized() {
 let c = Config { value_mode: "bad".into(), indicator_style: "dial".into(), color_mode: "rainbow".into(), ..Default::default() }.sanitized();
 assert_eq!((c.value_mode.as_str(), c.indicator_style.as_str(), c.color_mode.as_str()), ("used", "compact", "multicolor"));
}
```

- [ ] **Step 2: Verify red**

Run: `cargo test --manifest-path src-tauri/Cargo.toml display_defaults_are_backward_compatible`
Expected: FAIL because display fields do not exist.

- [ ] **Step 3: Add exact schema**

```ts
export type ValueMode = "used" | "remaining";
export type IndicatorStyle = "battery" | "horizontal-progress" | "percentage" | "provider-icon-bar" | "compact";
export type MetricId = "session" | "weekly" | "api";
export type ColorMode = "multicolor" | "greyscale" | "single-color";
export interface DisplayColors { session: string; weekly: string; api: string; single: string; background: string; text: string; }
```

Add matching camelCase-serialized Rust fields: `value_mode`, `indicator_style`, `enabled_metrics`, `metric_order`, `color_mode`, `display_colors`, `adapt_to_system_theme`, and `glow_enabled`. Defaults are used/compact/all metrics/multicolor/current palette/adaptation true/glow false. Sanitize enum strings, deduplicate order, append missing metrics, require at least one enabled metric, and validate every `#RRGGBB` color with existing `valid_hex_color`.

- [ ] **Step 4: Verify green**

Run: `cargo test --manifest-path src-tauri/Cargo.toml config && npm run build`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/types.ts src-tauri/src/config.rs src/display/config.test.ts
git commit -m "feat: persist configurable display preferences"
```

### Task 2: Build the pure display-model projection

**Files:**
- Create: `src/display/model.ts`
- Create: `src/display/model.test.ts`

- [ ] **Step 1: Write failing model tests**

```ts
it("inverts used values without changing status thresholds", () => {
 const used = buildDisplayModel(snapshot(72), prefs({ valueMode: "used" }));
 const remaining = buildDisplayModel(snapshot(72), prefs({ valueMode: "remaining" }));
 expect(used.metrics[0].displayPercent).toBe(72);
 expect(remaining.metrics[0].displayPercent).toBe(28);
 expect(remaining.metrics[0].severity).toBe(used.metrics[0].severity);
});
it("orders and filters session weekly and api", () => {
 expect(buildDisplayModel(allMetrics, prefs({ enabledMetrics: ["api", "session"], metricOrder: ["api", "weekly", "session"] })).metrics.map(x => x.id)).toEqual(["api", "session"]);
});
```

- [ ] **Step 2: Verify red**

Run: `npm test -- src/display/model.test.ts`
Expected: FAIL with missing `buildDisplayModel`.

- [ ] **Step 3: Implement the projection**

```ts
export function buildDisplayModel(snapshot: DisplaySnapshot, prefs: DisplayPreferences): DisplayModel {
 const metrics = prefs.metricOrder.filter(id => prefs.enabledMetrics.includes(id)).flatMap(id => {
   const value = snapshot.metrics[id]; if (!value) return [];
   const used = Math.max(0, Math.min(100, value.usedPercent));
   return [{ id, usedPercent: used, displayPercent: prefs.valueMode === "used" ? used : 100 - used,
     label: prefs.valueMode === "used" ? `${Math.round(used)}% used` : `${Math.round(100-used)}% remaining`,
     severity: used >= 90 ? "critical" : used >= 75 ? "warning" : "normal" }];
 });
 return { provider: snapshot.provider, style: prefs.indicatorStyle, metrics };
}
```

Map existing window labels deterministically: `5h`/session → session, weekly → weekly, API/account → api; missing metrics are omitted rather than synthesized.

- [ ] **Step 4: Verify green**

Run: `npm test -- src/display/model.test.ts`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/display/model.ts src/display/model.test.ts
git commit -m "feat: derive reusable multi-metric display models"
```

### Task 3: Render all five styles from shared primitives

**Files:**
- Create: `src/display/Indicator.tsx`
- Create: `src/display/Indicator.test.tsx`
- Create: `src/display/indicator.css`
- Modify: `src/app/overlay/ProviderLayer.tsx`

- [ ] **Step 1: Write failing style matrix test**

```tsx
it.each(["battery", "horizontal-progress", "percentage", "provider-icon-bar", "compact"] as const)("renders %s accessibly", style => {
 render(<Indicator model={{ ...model, style }} />);
 expect(screen.getByTestId(`indicator-${style}`)).toBeVisible();
 expect(screen.getAllByRole("progressbar")).toHaveLength(model.metrics.length);
});
```

- [ ] **Step 2: Verify red**

Run: `npm test -- src/display/Indicator.test.tsx`
Expected: FAIL because `Indicator` is absent.

- [ ] **Step 3: Implement exhaustive rendering**

Use an exhaustive `switch (model.style)` returning `BatteryIndicator`, `HorizontalProgressIndicator`, `PercentageIndicator`, `ProviderIconBarIndicator`, or `CompactIndicator`. Every metric primitive receives `aria-label={metric.label}`, `aria-valuemin={0}`, `aria-valuemax={100}`, `aria-valuenow={metric.displayPercent}`, `data-metric={metric.id}`, and CSS variable `--indicator-progress`. Battery uses a segmented body, horizontal uses a track/fill, percentage uses large text, provider-icon-bar uses the existing provider asset beside a bar, and compact uses a 16px radial conic indicator plus short percentage.

- [ ] **Step 4: Verify green and axe**

Run: `npm test -- src/display/Indicator.test.tsx src/a11y.test.ts`
Expected: PASS with no axe violations.

- [ ] **Step 5: Commit**

```bash
git add src/display src/app/overlay/ProviderLayer.tsx src/a11y.test.ts
git commit -m "feat: render five accessible indicator styles"
```

### Task 4: Add colors, system adaptation, and glow coverage

**Files:**
- Create: `src/display/colors.ts`
- Create: `src/display/colors.test.ts`
- Modify: `src/display/indicator.css`
- Modify: `src/controllers/geometry-controller.ts`

- [ ] **Step 1: Write failing palette tests**

```ts
it("resolves every color mode", () => {
 expect(resolvePalette("greyscale", colors, "dark").session).toBe("#d1d5db");
 expect(resolvePalette("single-color", colors, "light").weekly).toBe(colors.single);
 expect(resolvePalette("multicolor", colors, "dark").api).toBe(colors.api);
});
it("chooses readable text for the system appearance", () => {
 expect(adaptPalette(colors, "light").text).toBe("#111827");
 expect(adaptPalette(colors, "dark").text).toBe("#f9fafb");
});
```

- [ ] **Step 2: Verify red**

Run: `npm test -- src/display/colors.test.ts`
Expected: FAIL with missing palette functions.

- [ ] **Step 3: Implement deterministic palettes and glow**

`resolvePalette` maps multicolor per element, greyscale to `#d1d5db/#9ca3af/#6b7280`, and single-color to `colors.single`; `adaptPalette` selects `#111827` on light and `#f9fafb` on dark while retaining explicit metric colors. Subscribe to `matchMedia("(prefers-color-scheme: dark)")` with cleanup. Apply glow only when enabled:

```css
.indicator[data-glow="true"] { filter: drop-shadow(0 0 6px color-mix(in srgb, var(--metric-color) 70%, transparent)); }
```

When glow is enabled, `GeometryController` sends `effectOutset: 8`; otherwise zero, using the rectangular coverage contract from the prerequisite branch.

- [ ] **Step 4: Verify green**

Run: `npm test -- src/display/colors.test.ts src/controllers && npm run build`
Expected: PASS; glow coverage is 8 logical pixels and system listener cleanup is balanced.

- [ ] **Step 5: Commit**

```bash
git add src/display src/controllers/geometry-controller.ts
git commit -m "feat: add adaptive palettes and glow theme"
```

### Task 5: Render dynamic native tray indicators

**Files:**
- Create: `src/display/tray-raster.ts`
- Create: `src/display/tray-raster.test.ts`
- Modify: `src/controllers/usage-controller.ts`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write the failing raster contract**

```ts
it("produces a 32px premultiplied RGBA tray payload", () => {
 const image = rasterizeTrayIndicator(model, palette, 32);
 expect(image).toEqual(expect.objectContaining({ width: 32, height: 32 }));
 expect(image.rgba).toHaveLength(32 * 32 * 4);
});
```

- [ ] **Step 2: Verify red**

Run: `npm test -- src/display/tray-raster.test.ts`
Expected: FAIL because rasterizer is absent.

- [ ] **Step 3: Implement and bridge the tray image**

Render the same model into an offscreen canvas at 32 logical pixels, scale by `devicePixelRatio`, read RGBA, and return `{ width, height, rgba: Array.from(data) }`. Add command:

```rust
#[tauri::command]
fn set_tray_indicator(app: tauri::AppHandle, width: u32, height: u32, rgba: Vec<u8>) -> Result<(), String> {
 if rgba.len() != width as usize * height as usize * 4 || width > 256 || height > 256 { return Err("invalid tray image".into()); }
 app.tray_by_id("usage").ok_or("tray unavailable")?.set_icon(Some(tauri::image::Image::new_owned(rgba, width, height))).map_err(|_| "tray update failed".into())
}
```

Coalesce usage/config changes in `UsageController`; when tray is disabled, do not rasterize or invoke.

- [ ] **Step 4: Verify green**

Run: `npm test -- src/display/tray-raster.test.ts && cargo test --manifest-path src-tauri/Cargo.toml && npm run build`
Expected: PASS; malformed dimensions are rejected and valid images update the `usage` tray.

- [ ] **Step 5: Commit**

```bash
git add src/display src/controllers/usage-controller.ts src-tauri/src/lib.rs
git commit -m "feat: draw live compact tray indicators"
```

### Task 6: Add complete live Settings previews

**Files:**
- Create: `src/app/settings/DisplaySettings.tsx`
- Create: `src/app/settings/DisplaySettings.test.tsx`
- Modify: `src/app/settings/SettingsApp.tsx`

- [ ] **Step 1: Write the failing interaction test**

```tsx
it("updates preview immediately and persists one canonical patch", async () => {
 const onChange = vi.fn(); render(<DisplaySettings value={prefs()} onChange={onChange} />);
 await userEvent.click(screen.getByRole("radio", { name: "Remaining" }));
 expect(screen.getByTestId("display-preview")).toHaveTextContent("remaining");
 expect(onChange).toHaveBeenLastCalledWith(expect.objectContaining({ valueMode: "remaining" }));
});
```

- [ ] **Step 2: Verify red**

Run: `npm test -- src/app/settings/DisplaySettings.test.tsx`
Expected: FAIL because Display Settings is absent.

- [ ] **Step 3: Implement controls and preview fixtures**

Add used/remaining radios; five style radios; session/weekly/API checkboxes with drag-free Move Up/Move Down buttons; multicolor/greyscale/single-color radios; six labeled color inputs; adaptation and glow switches. Disable the final enabled metric. Render `<Indicator>` with a fixed preview snapshot (session 42, weekly 68, API 84) and the unsaved local value; debounce persistence 150ms, cancel on unmount, and revert to canonical config with an inline `role="alert"` if `set_config` rejects.

- [ ] **Step 4: Verify full suite**

Run: `npm test && npm run coverage && npm run tauri build`
Expected: all checks PASS and previews update without waiting for Rust persistence.

- [ ] **Step 5: Commit**

```bash
git add src/app/settings
git commit -m "feat: add live display configuration previews"
```

