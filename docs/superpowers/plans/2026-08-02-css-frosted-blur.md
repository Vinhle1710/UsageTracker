# CSS Frosted and Blur Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace native Acrylic and Blur rendering with card-local CSS Frosted and Blur themes while preserving the recovered borderless single-window lifecycle.

**Architecture:** Keep the existing Tauri window, Win32 border repair, rounded multi-card region, geometry, tray, and polling paths unchanged. Migrate the public theme ID from `acrylic` to `frosted`; map Frosted and Blur to native Clear in Rust, then render their visual treatment entirely on each `.layer` card with CSS.

**Tech Stack:** Rust, Tauri v2, TypeScript, CSS, Vitest/jsdom, Cargo test, Windows WebView2

---

## File map

- `src-tauri/src/config.rs`: persisted theme default, validation, and migration.
- `src-tauri/src/material.rs`: maps visible presets to the native material used by the stable host window.
- `src/types.ts`: frontend theme union.
- `src/main.ts`: browser-preview default and CSS opacity variables.
- `src/components/settings.ts`: Frosted/Blur controls and previews.
- `src/components/settings.test.ts`, `src/a11y.test.ts`, `src/state.test.ts`: frontend behavior and accessibility fixtures.
- `src/styles/app.css`: full-card CSS Frosted and Blur effects.
- `src/styles/app-css.test.ts`: CSS contract preventing native-era transparent-card rules from returning.
- Package/version files: produce a distinct `0.1.2` installer.

### Task 1: Migrate persisted themes and disable native Acrylic/Blur selection

**Files:**
- Modify: `src-tauri/src/config.rs`
- Modify: `src-tauri/src/material.rs`

- [ ] **Step 1: Write failing Rust tests for the new default, migration, and native Clear mapping**

Update the config assertions and replace the old migration tests with:

```rust
#[test]
fn defaults_to_frosted() {
    assert_eq!(Config::default().theme, "frosted");
}

#[test]
fn sanitize_migrates_native_material_themes_to_frosted() {
    for legacy in ["acrylic", "opaque", "custom"] {
        let sanitized = Config {
            theme: legacy.into(),
            ..Default::default()
        }
        .sanitized();
        assert_eq!(sanitized.theme, "frosted");
    }
}

#[test]
fn sanitize_accepts_css_blur() {
    let sanitized = Config {
        theme: "blur".into(),
        ..Default::default()
    }
    .sanitized();
    assert_eq!(sanitized.theme, "blur");
}
```

Replace `maps_supported_materials_without_guessing` with:

```rust
#[test]
fn css_glass_themes_keep_the_native_host_clear() {
    assert_eq!(material_for_theme("clear"), Material::Clear);
    assert_eq!(material_for_theme("frosted"), Material::Clear);
    assert_eq!(material_for_theme("blur"), Material::Clear);
    assert_eq!(material_for_theme("solid"), Material::Solid);
    assert_eq!(material_for_theme("unknown"), Material::Clear);
}
```

- [ ] **Step 2: Run the focused tests and verify they fail**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml defaults_to_frosted
cargo test --manifest-path src-tauri/Cargo.toml css_glass_themes_keep_the_native_host_clear
```

Expected: assertions fail because the default is `acrylic` and Blur still maps to `Material::Blur`.

- [ ] **Step 3: Implement the minimal Rust migration and mapping**

Change the default and sanitization to:

```rust
fn default_theme() -> String {
    "frosted".into()
}

pub fn sanitized(mut self) -> Self {
    self.scale = self.scale.clamp(0.75, 1.5);
    self.card_opacity = self.card_opacity.clamp(0.82, 1.0);
    if matches!(self.theme.as_str(), "acrylic" | "opaque" | "custom") {
        self.theme = "frosted".into();
    }
    if !matches!(self.theme.as_str(), "clear" | "frosted" | "blur" | "solid") {
        self.theme = default_theme();
    }
    // Keep the existing color, layout, and interval validation below unchanged.
```

Change the material selector to:

```rust
pub fn material_for_theme(theme: &str) -> Material {
    match theme {
        "solid" => Material::Solid,
        "clear" | "frosted" | "blur" => Material::Clear,
        _ => Material::Clear,
    }
}
```

Do not change `apply_to_window`, `enforce_borderless`, region creation, focus restoration, or show/hide code.

- [ ] **Step 4: Run all Rust tests and formatting**

Run:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: all Rust tests pass.

- [ ] **Step 5: Commit the persistence/native boundary**

```powershell
git add src-tauri/src/config.rs src-tauri/src/material.rs
git commit -m "fix: route glass themes through the clear native host"
```

### Task 2: Render Frosted and Blur on complete provider cards

**Files:**
- Create: `src/styles/app-css.test.ts`
- Modify: `src/types.ts`
- Modify: `src/main.ts`
- Modify: `src/components/settings.ts`
- Modify: `src/components/settings.test.ts`
- Modify: `src/a11y.test.ts`
- Modify: `src/state.test.ts`
- Modify: `src/styles/app.css`

- [ ] **Step 1: Write failing frontend tests for Frosted settings and CSS contracts**

Change all typed fixture themes from `"acrylic"` to `"frosted"`. In `settings.test.ts`, assert:

```ts
expect(el.querySelector('[data-theme="frosted"]')).not.toBeNull();
expect(el.querySelector('[data-theme="acrylic"]')).toBeNull();
expect(el.querySelector('[data-theme="blur"]')).not.toBeNull();

el.querySelector<HTMLButtonElement>('[data-theme="frosted"]')!.click();
expect(onChange).toHaveBeenCalledWith(
  expect.objectContaining({ theme: "frosted", cardOpacity: config.cardOpacity }),
);
```

Create `src/styles/app-css.test.ts`:

```ts
import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";

const css = readFileSync(new URL("./app.css", import.meta.url), "utf8");

describe("CSS glass themes", () => {
  it("renders Frosted and Blur on full provider cards", () => {
    expect(css).toContain('#app[data-theme="frosted"] .layer');
    expect(css).toContain("backdrop-filter: blur(18px) saturate(145%)");
    expect(css).toContain("-webkit-backdrop-filter: blur(18px) saturate(145%)");
    expect(css).toContain('#app[data-theme="blur"] .layer');
    expect(css).toContain("backdrop-filter: blur(12px)");
    expect(css).toContain("-webkit-backdrop-filter: blur(12px)");
  });

  it("does not retain the native Acrylic selector or glass gradients", () => {
    expect(css).not.toContain('data-theme="acrylic"');
    expect(css).not.toMatch(/data-theme="(?:frosted|blur)"[^\n]*linear-gradient/);
  });
});
```

- [ ] **Step 2: Run focused frontend tests and verify they fail**

Run:

```powershell
npx vitest run src/components/settings.test.ts src/styles/app-css.test.ts
```

Expected: Frosted controls/selectors are absent and the CSS contract fails.

- [ ] **Step 3: Update the frontend theme model and defaults**

In `src/types.ts`:

```ts
export type ThemePreset = "clear" | "frosted" | "blur" | "solid";
```

In `src/main.ts`, set the preview default to `theme: "frosted"` and extend `applyAppearance`:

```ts
app.style.setProperty("--card-opacity", `${Math.round(config.cardOpacity * 100)}%`);
app.style.setProperty("--frosted-opacity", `${Math.round(config.cardOpacity * 72)}%`);
app.style.setProperty("--blur-opacity", `${Math.round(config.cardOpacity * 58)}%`);
app.style.setProperty("--card-background", config.backgroundColor);
```

This maps the existing 82–100% opacity setting to a visibly translucent 59–72% Frosted wash and 48–58% Blur wash without changing the stored user value.

- [ ] **Step 4: Replace Acrylic with Frosted in settings**

Use this button in `settings.ts`:

```ts
<button type="button" class="theme-option" data-theme="frosted" aria-pressed="${config.theme === "frosted"}"><span class="theme-preview theme-preview--frosted" data-preview-theme="frosted"><i></i><i></i></span><strong>Frosted</strong></button>
```

Keep the existing Blur, Translucent gradient, and Solid buttons and instant event handlers.

- [ ] **Step 5: Add full-card CSS Frosted and Blur effects**

Replace the shared transparent Acrylic/Blur rule with:

```css
#app[data-theme="frosted"] .layer {
  background: color-mix(in srgb, var(--card-background) var(--frosted-opacity), transparent);
  -webkit-backdrop-filter: blur(18px) saturate(145%);
  backdrop-filter: blur(18px) saturate(145%);
  box-shadow: inset 0 1px 0 rgb(255 255 255 / 14%);
}

#app[data-theme="blur"] .layer {
  background: color-mix(in srgb, var(--card-background) var(--blur-opacity), transparent);
  -webkit-backdrop-filter: blur(12px);
  backdrop-filter: blur(12px);
}
```

Replace the preview selectors with:

```css
.theme-preview--frosted i {
  background: rgb(33 52 77 / 62%);
  -webkit-backdrop-filter: blur(9px) saturate(145%);
  backdrop-filter: blur(9px) saturate(145%);
  box-shadow: inset 0 1px 0 rgb(255 255 255 / 18%);
}
.theme-preview--blur i {
  background: rgb(24 40 62 / 48%);
  -webkit-backdrop-filter: blur(6px);
  backdrop-filter: blur(6px);
}
```

Do not put background, blur, shadow, or borders on `html`, `body`, `#app`, or `.layers`.

- [ ] **Step 6: Run frontend and accessibility tests**

Run:

```powershell
npm test
npm run build
```

Expected: all Vitest, accessibility, TypeScript, and Vite checks pass.

- [ ] **Step 7: Commit the CSS themes**

```powershell
git add src/types.ts src/main.ts src/components/settings.ts src/components/settings.test.ts src/a11y.test.ts src/state.test.ts src/styles/app.css src/styles/app-css.test.ts
git commit -m "feat: replace native materials with css glass cards"
```

### Task 3: Live verification and release 0.1.2

**Files:**
- Modify: `package.json`
- Modify: `package-lock.json`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`
- Modify: `src-tauri/tauri.conf.json`

- [ ] **Step 1: Build and launch the untouched packaged executable**

Run:

```powershell
npm run tauri build -- --no-bundle
Start-Process -FilePath "D:\GitHub\Personal\UsageTracker\src-tauri\target\release\usage-tracker-overlay.exe"
```

Expected: one borderless overlay appears with complete provider cards and no host rectangle.

- [ ] **Step 2: Verify Frosted and Blur live**

From the tray, open Settings and switch between Frosted and Blur. Confirm:

- Both effects cover each full card.
- Frosted is stronger and more saturated than Blur.
- Neither uses a gradient.
- The gap and exterior window area remain transparent.
- Background and Card opacity update immediately.
- If the desktop itself does not blur, record WebView2 compositor limitation; do not reintroduce native helper windows.

- [ ] **Step 3: Re-run lifecycle regression checks**

Confirm initial reveal, one polling cycle, in-card minimize/restore, settings close, and tray Show/Hide. Probe the main window style after each transition; expected style is `0x14000000` with caption, system menu, and resize frame all absent.

- [ ] **Step 4: Bump the release version to 0.1.2**

Change only the project version fields from `0.1.1` to `0.1.2` in all five version files, then run:

```powershell
cargo check --manifest-path src-tauri/Cargo.toml
git diff --check
```

Expected: Cargo updates only the local package entry in `src-tauri/Cargo.lock`.

- [ ] **Step 5: Run the complete release gate**

Run:

```powershell
npm test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
npm run tauri build
```

Expected: all checks pass and the NSIS installer exists at `src-tauri/target/release/bundle/nsis/Usage Tracker Overlay_0.1.2_x64-setup.exe`.

- [ ] **Step 6: Commit and push the release**

```powershell
git add package.json package-lock.json src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/tauri.conf.json
git commit -m "chore: release css glass themes as 0.1.2"
git push origin codex/per-card-native-material
```

- [ ] **Step 7: Update pull request verification**

Update PR #2 with the CSS-only Frosted/Blur summary, live borderless verification, test totals, installer path, and SHA-256. Do not claim desktop wallpaper blur if WebView2 does not visibly produce it.
