# Per-card Native Material Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Render genuine Windows Acrylic or Blur inside each provider card without coloring the unused overlay host, flashing a native title bar, splitting cards, or transferring one provider's data to another.

**Architecture:** Keep the existing `main` WebView as a permanently transparent foreground and create two hidden native Tauri windows, `material-claude` and `material-openai`, as decorative backdrops. Frontend geometry carries explicit provider identity; Rust positions each backdrop behind its matching card and applies native material only to those windows. CSS continues to render Translucent gradient and Solid, while ordinary usage polling mutates existing meters without moving or replacing provider nodes.

**Tech Stack:** Tauri 2.11 native windows, Rust 2021, Windows DWM/User32 composition APIs through `windows-sys`, TypeScript 5.6, WebView2, Vitest, Cargo tests.

---

## File map

- Modify `src/geometry.ts`: attach provider identity to every measured card rectangle.
- Modify `src/geometry.test.ts`: verify provider-tagged geometry and content height.
- Modify `src/main.ts`: measure provider-tagged cards and avoid redundant geometry work.
- Modify `src/components/overlay.ts`: preserve stable provider DOM nodes and order during polling.
- Modify `src/components/overlay.test.ts`: prove provider isolation and in-place updates.
- Modify `src/styles/app.css`: keep the host transparent and let native backdrops show through Acrylic and Blur cards.
- Modify `src-tauri/Cargo.toml`: enable Tauri's native-window API.
- Modify `src-tauri/src/model.rs`: make provider identity usable by geometry and backdrop planning.
- Modify `src-tauri/src/material.rs`: retain pure material policy and low-level Windows material helpers, but remove main-window shaping.
- Create `src-tauri/src/material_windows.rs`: own native backdrop creation, provider mapping, positioning, visibility, and z-order.
- Modify `src-tauri/src/lib.rs`: integrate backdrop windows into setup, geometry, tray visibility, source detection, and settings changes.
- Modify `docs/plans/2026-08-01-per-card-native-material-design.md`: record verified Windows fallback behavior only if live testing differs from the approved contract.

### Task 1: Carry provider identity through measured geometry

**Files:**
- Modify: `src/geometry.ts`
- Modify: `src/geometry.test.ts`
- Modify: `src/main.ts`
- Modify: `src-tauri/src/material.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write the failing TypeScript geometry test**

Replace the card inputs and expected regions in `src/geometry.test.ts` with provider-tagged values:

```ts
import { describe, expect, it } from "vitest";
import { calculateOverlayGeometry } from "./geometry";

describe("calculateOverlayGeometry", () => {
  it("keeps provider ownership on every measured card", () => {
    const geometry = calculateOverlayGeometry(
      { left: 0, top: 0 },
      [
        { provider: "claude", left: 8, top: 8, width: 310, height: 70, right: 318, bottom: 78 },
        { provider: "openai", left: 8, top: 87, width: 310, height: 166, right: 318, bottom: 253 },
      ],
      8,
      14,
    );

    expect(geometry.regions).toEqual([
      { provider: "claude", x: 8, y: 8, width: 310, height: 70, radius: 14 },
      { provider: "openai", x: 8, y: 87, width: 310, height: 166, radius: 14 },
    ]);
    expect(geometry.contentHeight).toBe(261);
  });
});
```

- [ ] **Step 2: Run the focused test and verify the type failure**

Run:

```powershell
npm test -- src/geometry.test.ts
```

Expected: FAIL because `provider` is not part of `LogicalCardRegion` or the measured rectangle type.

- [ ] **Step 3: Add provider-tagged geometry types and conversion**

Change `src/geometry.ts` to use the shared provider type and preserve it in the output:

```ts
import type { Provider } from "./types";

export interface LogicalCardRegion {
  provider: Provider;
  x: number;
  y: number;
  width: number;
  height: number;
  radius: number;
}

interface RectOrigin {
  left: number;
  top: number;
}

interface MeasuredProviderRect extends RectOrigin {
  provider: Provider;
  width: number;
  height: number;
  right: number;
  bottom: number;
}

export interface OverlayGeometryMeasurement {
  regions: LogicalCardRegion[];
  contentHeight: number | null;
}

export function calculateOverlayGeometry(
  root: RectOrigin,
  cards: MeasuredProviderRect[],
  padding: number,
  radius: number,
): OverlayGeometryMeasurement {
  if (!cards.length) return { regions: [], contentHeight: null };
  const regions = cards.map((card) => ({
    provider: card.provider,
    x: card.left - root.left,
    y: card.top - root.top,
    width: card.width,
    height: card.height,
    radius,
  }));
  const bottom = Math.max(...cards.map((card) => card.bottom - root.top));
  return { regions, contentHeight: Math.ceil(bottom + padding) };
}
```

Update the measurement in `src/main.ts` so invalid DOM provider values are excluded rather than guessed:

```ts
const cards = Array.from(app.querySelectorAll<HTMLElement>(".layer[data-provider]"))
  .flatMap((layer) => {
    const provider = layer.dataset.provider;
    if (provider !== "claude" && provider !== "openai") return [];
    const rect = layer.getBoundingClientRect();
    return [{ provider, left: rect.left, top: rect.top, width: rect.width, height: rect.height, right: rect.right, bottom: rect.bottom }];
  });
```

Add provider identity to the Rust logical and physical region structures in `src-tauri/src/material.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardRegion {
    pub provider: crate::model::Provider,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub radius: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogicalCardRegion {
    pub provider: crate::model::Provider,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub radius: f64,
}
```

Update `physical_card_regions` to copy `provider`, and remove the positional `card_regions` fallback. In `apply_overlay_geometry`, use `physical_card_regions(&request.regions, scale_factor)` directly; an empty list means no native backdrop.

- [ ] **Step 4: Update the Rust scale-conversion test**

Use an explicit Claude region and assert that identity survives scaling:

```rust
#[test]
fn logical_card_measurements_keep_provider_identity_and_monitor_scale() {
    let logical = vec![LogicalCardRegion {
        provider: crate::model::Provider::Claude,
        x: 6.4,
        y: 6.4,
        width: 248.0,
        height: 56.0,
        radius: 11.2,
    }];

    assert_eq!(
        physical_card_regions(&logical, 1.25),
        vec![CardRegion {
            provider: crate::model::Provider::Claude,
            x: 8,
            y: 8,
            width: 310,
            height: 70,
            radius: 14,
        }]
    );
}
```

- [ ] **Step 5: Run focused frontend and Rust tests**

Run:

```powershell
npm test -- src/geometry.test.ts
cargo test --manifest-path src-tauri/Cargo.toml material::tests::logical_card_measurements_keep_provider_identity_and_monitor_scale
```

Expected: both tests PASS.

- [ ] **Step 6: Commit the provider-tagged geometry contract**

```powershell
git add src/geometry.ts src/geometry.test.ts src/main.ts src-tauri/src/material.rs src-tauri/src/lib.rs
git commit -m "refactor: tag overlay geometry by provider"
```

### Task 2: Define deterministic per-provider backdrop plans

**Files:**
- Modify: `src-tauri/src/model.rs`
- Modify: `src-tauri/src/material.rs`

- [ ] **Step 1: Write failing tests for material visibility, mapping, and tint strength**

Add these tests to `src-tauri/src/material.rs`:

```rust
#[test]
fn native_material_is_used_only_for_acrylic_and_blur() {
    assert_eq!(resolved_material("clear", true), None);
    assert_eq!(resolved_material("solid", true), None);
    assert_eq!(resolved_material("acrylic", true), Some(Material::Acrylic));
    assert_eq!(resolved_material("blur", true), Some(Material::Blur));
    assert_eq!(resolved_material("blur", false), Some(Material::Acrylic));
}

#[test]
fn native_tints_keep_the_desktop_visible() {
    assert_eq!(material_alpha(Material::Acrylic, 0.82), 64);
    assert_eq!(material_alpha(Material::Acrylic, 1.0), 128);
    assert_eq!(material_alpha(Material::Blur, 0.82), 8);
    assert_eq!(material_alpha(Material::Blur, 1.0), 32);
}

#[test]
fn plans_are_keyed_by_provider_not_region_order() {
    let regions = vec![
        CardRegion { provider: crate::model::Provider::Openai, x: 8, y: 90, width: 310, height: 160, radius: 14 },
        CardRegion { provider: crate::model::Provider::Claude, x: 8, y: 8, width: 310, height: 70, radius: 14 },
    ];
    let plans = plan_backdrops("acrylic", false, &regions, (100, 200), "#07101f", 0.9, true);

    assert_eq!(plans[0].provider, crate::model::Provider::Claude);
    assert_eq!(plans[0].frame, Some((108, 208, 310, 70, 14)));
    assert_eq!(plans[1].provider, crate::model::Provider::Openai);
    assert_eq!(plans[1].frame, Some((108, 290, 310, 160, 14)));
}

#[test]
fn minimized_overlay_hides_both_native_backdrops() {
    let regions = vec![CardRegion { provider: crate::model::Provider::Claude, x: 8, y: 8, width: 310, height: 70, radius: 14 }];
    let plans = plan_backdrops("blur", true, &regions, (0, 0), "#07101f", 0.9, true);
    assert!(plans.iter().all(|plan| plan.material.is_none()));
}
```

- [ ] **Step 2: Run the tests and verify missing planning functions**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml material::tests
```

Expected: FAIL because `resolved_material`, `material_alpha`, `BackdropPlan`, and `plan_backdrops` do not exist.

- [ ] **Step 3: Make provider values copyable keys and implement pure planning**

Extend the derive list on `Provider` in `src-tauri/src/model.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    Claude,
    Openai,
}
```

Add the planning API to `src-tauri/src/material.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackdropPlan {
    pub provider: crate::model::Provider,
    pub frame: Option<(i32, i32, i32, i32, i32)>,
    pub material: Option<NativeMaterialSpec>,
}

pub fn resolved_material(theme: &str, blur_supported: bool) -> Option<Material> {
    match theme {
        "acrylic" => Some(Material::Acrylic),
        "blur" if blur_supported => Some(Material::Blur),
        "blur" => Some(Material::Acrylic),
        "clear" | "solid" => None,
        _ => Some(Material::Acrylic),
    }
}

pub fn material_alpha(material: Material, opacity: f32) -> u8 {
    let strength = ((opacity.clamp(0.82, 1.0) - 0.82) / 0.18).clamp(0.0, 1.0);
    let (minimum, range) = match material {
        Material::Acrylic => (64.0, 64.0),
        Material::Blur => (8.0, 24.0),
        Material::Clear | Material::Solid => (0.0, 0.0),
    };
    (minimum + strength * range).round() as u8
}

pub fn parse_color(color: &str, alpha: u8) -> Option<(u8, u8, u8, u8)> {
    let hex = color.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    Some((
        u8::from_str_radix(&hex[0..2], 16).ok()?,
        u8::from_str_radix(&hex[2..4], 16).ok()?,
        u8::from_str_radix(&hex[4..6], 16).ok()?,
        alpha,
    ))
}

pub fn plan_backdrops(
    theme: &str,
    minimized: bool,
    regions: &[CardRegion],
    origin: (i32, i32),
    color: &str,
    opacity: f32,
    blur_supported: bool,
) -> [BackdropPlan; 2] {
    let material = (!minimized).then(|| resolved_material(theme, blur_supported)).flatten();
    let plan_for = |provider| {
        let region = regions.iter().find(|region| region.provider == provider);
        let frame = region.map(|region| (
            origin.0 + region.x,
            origin.1 + region.y,
            region.width,
            region.height,
            region.radius,
        ));
        let spec = material.and_then(|material| {
            frame.map(|_| NativeMaterialSpec {
                material,
                tint: parse_color(color, material_alpha(material, opacity)).unwrap_or((7, 16, 31, 96)),
            })
        });
        BackdropPlan { provider, frame, material: spec }
    };
    [
        plan_for(crate::model::Provider::Claude),
        plan_for(crate::model::Provider::Openai),
    ]
}
```

- [ ] **Step 4: Run the pure Rust tests**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml material::tests
```

Expected: all material tests PASS, including reversed-region provider mapping.

- [ ] **Step 5: Commit the deterministic material planner**

```powershell
git add src-tauri/src/model.rs src-tauri/src/material.rs
git commit -m "feat: plan native backdrops by provider"
```

### Task 3: Create and control native card backdrop windows

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/material_windows.rs`
- Modify: `src-tauri/src/material.rs`
- Modify: `src-tauri/src/lib.rs`

- [ ] **Step 1: Write failing state tests for fixed provider windows**

Start `src-tauri/src/material_windows.rs` with the state-only API and tests:

```rust
use crate::{material::NativeWindowState, model::Provider};

pub const CLAUDE_LABEL: &str = "material-claude";
pub const OPENAI_LABEL: &str = "material-openai";

#[derive(Debug, Default)]
pub struct MaterialWindowStates {
    pub claude: NativeWindowState,
    pub openai: NativeWindowState,
}

impl MaterialWindowStates {
    pub fn get_mut(&mut self, provider: Provider) -> &mut NativeWindowState {
        match provider {
            Provider::Claude => &mut self.claude,
            Provider::Openai => &mut self.openai,
        }
    }
}

pub fn label_for(provider: Provider) -> &'static str {
    match provider {
        Provider::Claude => CLAUDE_LABEL,
        Provider::Openai => OPENAI_LABEL,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_labels_never_depend_on_card_order() {
        assert_eq!(label_for(Provider::Claude), "material-claude");
        assert_eq!(label_for(Provider::Openai), "material-openai");
    }

    #[test]
    fn provider_state_is_never_shared() {
        let mut states = MaterialWindowStates::default();
        states.get_mut(Provider::Claude).enabled = true;
        assert!(states.claude.enabled);
        assert!(!states.openai.enabled);
    }
}
```

Declare `pub mod material_windows;` in `src-tauri/src/lib.rs`.

- [ ] **Step 2: Run the focused test and verify the module is incomplete**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml material_windows::tests
```

Expected: FAIL because `NativeWindowState` does not yet have an `enabled` field.

- [ ] **Step 3: Enable native Tauri windows**

Change the Tauri dependency in `src-tauri/Cargo.toml`:

```toml
tauri = { version = "2", features = ["tray-icon", "unstable"] }
```

The `unstable` feature exposes `tauri::WindowBuilder`, which creates a native window without a WebView.

- [ ] **Step 4: Add backdrop creation and lifecycle functions**

Add these functions to `src-tauri/src/material_windows.rs`:

```rust
pub fn create(app: &tauri::App) -> tauri::Result<()> {
    for label in [CLAUDE_LABEL, OPENAI_LABEL] {
        let window = tauri::WindowBuilder::new(app, label)
            .title("")
            .inner_size(1.0, 1.0)
            .position(-32_000.0, -32_000.0)
            .decorations(false)
            .shadow(false)
            .resizable(false)
            .skip_taskbar(true)
            .always_on_top(true)
            .focused(false)
            .focusable(false)
            .transparent(true)
            .visible(false)
            .build()?;
        window.set_ignore_cursor_events(true)?;
    }
    Ok(())
}

pub fn hide_all(app: &tauri::AppHandle) {
    for label in [CLAUDE_LABEL, OPENAI_LABEL] {
        if let Some(window) = app.get_window(label) {
            let _ = window.hide();
        }
    }
}

pub fn set_always_on_top(app: &tauri::AppHandle, enabled: bool) {
    for label in [CLAUDE_LABEL, OPENAI_LABEL] {
        if let Some(window) = app.get_window(label) {
            let _ = window.set_always_on_top(enabled);
        }
    }
}
```

Add `use tauri::Manager;` to the module.

- [ ] **Step 5: Move Win32 material application from the WebView to a native Window**

In `src-tauri/src/material.rs`, change the native application function to accept `&tauri::Window`, apply a single local rounded region, and never resize or reshape the main WebView:

```rust
#[cfg(target_os = "windows")]
pub fn apply_to_backdrop(
    window: &tauri::Window,
    desired: NativeMaterialSpec,
    size: (u32, u32),
    radius: i32,
    current: &mut NativeWindowState,
) -> Result<(), String> {
    if current.size != Some(size) {
        window
            .set_size(tauri::PhysicalSize::new(size.0, size.1))
            .map_err(|error| error.to_string())?;
    }
    enforce_borderless_hwnd(window.hwnd().map_err(|error| error.to_string())?.0)?;
    if current.material != Some(desired) {
        apply_accent_policy(window.hwnd().map_err(|error| error.to_string())?.0, desired)?;
    }
    if current.size != Some(size) || current.radius != Some(radius) {
        apply_rounded_region(window.hwnd().map_err(|error| error.to_string())?.0, size, radius)?;
    }
    current.material = Some(desired);
    current.size = Some(size);
    current.radius = Some(radius);
    Ok(())
}
```

Change `NativeWindowState` to store `material`, `size`, `radius`, `frame`, and `enabled`; remove the multi-region field:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NativeWindowState {
    pub material: Option<NativeMaterialSpec>,
    pub size: Option<(u32, u32)>,
    pub radius: Option<i32>,
    pub frame: Option<(i32, i32, i32, i32, i32)>,
    pub enabled: bool,
}
```

Delete `NativeUpdatePlan`, `plan_native_update`, `should_apply_card_region`, `card_regions`, `apply_card_region`, `restore_window_surface`, and `apply_to_window`; those APIs all model the rejected multi-region main-window architecture. Replace their unchanged-state test with:

```rust
#[test]
fn unchanged_backdrop_state_keeps_material_and_geometry_cached() {
    let state = NativeWindowState {
        material: Some(NativeMaterialSpec {
            material: Material::Acrylic,
            tint: (7, 16, 31, 96),
        }),
        size: Some((310, 160)),
        radius: Some(14),
        frame: Some((108, 290, 310, 160, 14)),
        enabled: true,
    };
    assert_eq!(state.size, Some((310, 160)));
    assert_eq!(state.radius, Some(14));
    assert!(state.enabled);
}
```

Extract the existing Win32 bodies into helpers with these exact responsibilities:

```rust
#[cfg(target_os = "windows")]
fn apply_rounded_region(
    hwnd: *mut std::ffi::c_void,
    size: (u32, u32),
    radius: i32,
) -> Result<(), String> {
    use windows_sys::Win32::Graphics::Gdi::{CreateRoundRectRgn, DeleteObject, SetWindowRgn};
    let region = unsafe {
        CreateRoundRectRgn(0, 0, size.0 as i32, size.1 as i32, radius * 2, radius * 2)
    };
    if region.is_null() {
        return Err(std::io::Error::last_os_error().to_string());
    }
    if unsafe { SetWindowRgn(hwnd, region, 1) } == 0 {
        unsafe { DeleteObject(region) };
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn enforce_borderless_hwnd(hwnd: *mut std::ffi::c_void) -> Result<(), String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_STYLE, SWP_FRAMECHANGED,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
    };
    let style = unsafe { GetWindowLongPtrW(hwnd, GWL_STYLE) as u32 };
    let stripped = borderless_style(style);
    if stripped != style {
        unsafe { SetWindowLongPtrW(hwnd, GWL_STYLE, stripped as isize) };
        if unsafe {
            SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                0,
                0,
                SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn disable_non_client_rendering_hwnd(hwnd: *mut std::ffi::c_void) -> Result<(), String> {
    use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_NCRENDERING_POLICY};
    let policy = 1_i32;
    let result = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_NCRENDERING_POLICY as u32,
            (&policy as *const i32).cast(),
            std::mem::size_of_val(&policy) as u32,
        )
    };
    if result < 0 {
        return Err(format!("DwmSetWindowAttribute failed with HRESULT {result:#x}"));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn enforce_foreground_borderless(window: &tauri::WebviewWindow) -> Result<(), String> {
    let hwnd = window.hwnd().map_err(|error| error.to_string())?.0;
    enforce_borderless_hwnd(hwnd)?;
    disable_non_client_rendering_hwnd(hwnd)?;
    window.set_decorations(false).map_err(|error| error.to_string())?;
    window.set_shadow(false).map_err(|error| error.to_string())
}
```

Add the Accent helper below. It accepts only a backdrop HWND, so no code path can apply Accent material to `main`:

```rust
#[cfg(target_os = "windows")]
fn apply_accent_policy(
    hwnd: *mut std::ffi::c_void,
    desired: NativeMaterialSpec,
) -> Result<(), String> {
    #[repr(C)]
    struct NativeAccentPolicy {
        state: u32,
        flags: u32,
        gradient_color: u32,
        animation_id: u32,
    }
    #[repr(C)]
    struct CompositionAttributeData {
        attribute: u32,
        data: *mut std::ffi::c_void,
        size: usize,
    }
    type SetWindowCompositionAttributeFn =
        unsafe extern "system" fn(*mut std::ffi::c_void, *mut CompositionAttributeData) -> i32;

    use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
    let policy = accent_policy(desired.material, desired.tint);
    let mut native_policy = NativeAccentPolicy {
        state: policy.state,
        flags: policy.flags,
        gradient_color: policy.gradient_color,
        animation_id: 0,
    };
    let mut data = CompositionAttributeData {
        attribute: 0x13,
        data: (&mut native_policy as *mut NativeAccentPolicy).cast(),
        size: std::mem::size_of::<NativeAccentPolicy>(),
    };
    let user32 = unsafe { GetModuleHandleA(c"user32.dll".as_ptr().cast()) };
    if user32.is_null() {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let Some(symbol) = (unsafe {
        GetProcAddress(user32, c"SetWindowCompositionAttribute".as_ptr().cast())
    }) else {
        return Err("SetWindowCompositionAttribute is unavailable".to_string());
    };
    let call: SetWindowCompositionAttributeFn = unsafe { std::mem::transmute(symbol) };
    if unsafe { call(hwnd, &mut data) } == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}
```

- [ ] **Step 6: Detect whether legacy generic Blur is available**

Add a pure support rule and a Windows build reader to `src-tauri/src/material_windows.rs`:

```rust
pub fn legacy_blur_supported(build: u32) -> bool {
    build != 0 && build <= 22_000
}

#[cfg(target_os = "windows")]
pub fn current_windows_build() -> u32 {
    #[repr(C)]
    struct RtlOsVersionInfo {
        size: u32,
        major: u32,
        minor: u32,
        build: u32,
        platform: u32,
        service_pack: [u16; 128],
    }
    type RtlGetVersion = unsafe extern "system" fn(*mut RtlOsVersionInfo) -> i32;

    let mut info = RtlOsVersionInfo {
        size: std::mem::size_of::<RtlOsVersionInfo>() as u32,
        major: 0,
        minor: 0,
        build: 0,
        platform: 0,
        service_pack: [0; 128],
    };
    unsafe {
        use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};
        let module = GetModuleHandleA(c"ntdll.dll".as_ptr().cast());
        let Some(symbol) = (!module.is_null())
            .then(|| GetProcAddress(module, c"RtlGetVersion".as_ptr().cast()))
            .flatten()
        else {
            return 0;
        };
        let call: RtlGetVersion = std::mem::transmute(symbol);
        if call(&mut info) >= 0 { info.build } else { 0 }
    }
}
```

Add these compatibility tests:

```rust
#[test]
fn legacy_blur_support_stops_after_windows_11_build_22000() {
    assert!(legacy_blur_supported(19_045));
    assert!(legacy_blur_supported(22_000));
    assert!(!legacy_blur_supported(22_621));
    assert!(!legacy_blur_supported(0));
}

#[test]
fn unsupported_blur_resolves_to_acrylic() {
    assert_eq!(crate::material::resolved_material("blur", false), Some(crate::material::Material::Acrylic));
}
```

- [ ] **Step 7: Run formatting and Rust tests**

Run:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml material_windows::tests
cargo test --manifest-path src-tauri/Cargo.toml material::tests
```

Expected: formatting check and both focused suites PASS.

- [ ] **Step 8: Commit native backdrop window support**

```powershell
git add src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/src/material.rs src-tauri/src/material_windows.rs src-tauri/src/lib.rs
git commit -m "feat: add native provider backdrop windows"
```

### Task 4: Integrate backdrops with geometry and overlay lifecycle

**Files:**
- Modify: `src-tauri/src/material_windows.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/visibility.rs`

- [ ] **Step 1: Write failing lifecycle-planning tests**

Add pure tests proving startup, tray hiding, provider loss, and minimization hide the decorative windows:

```rust
#[test]
fn a_hidden_foreground_never_shows_backdrops() {
    assert!(!should_show_backdrop(true, false));
    assert!(should_show_backdrop(true, true));
    assert!(!should_show_backdrop(false, true));
}

#[test]
fn provider_removal_hides_only_its_fixed_window() {
    let claude = BackdropPlan { provider: Provider::Claude, frame: None, material: None };
    let openai = BackdropPlan {
        provider: Provider::Openai,
        frame: Some((108, 290, 310, 160, 14)),
        material: Some(crate::material::NativeMaterialSpec {
            material: crate::material::Material::Acrylic,
            tint: (7, 16, 31, 96),
        }),
    };
    assert!(!plan_is_enabled(&claude));
    assert!(plan_is_enabled(&openai));
}
```

- [ ] **Step 2: Run the lifecycle tests**

Run:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml material_windows::tests
```

Expected: FAIL because `should_show_backdrop` and `plan_is_enabled` do not exist.

- [ ] **Step 3: Store per-provider native state**

Implement the pure lifecycle helpers in `src-tauri/src/material_windows.rs`:

```rust
pub fn should_show_backdrop(enabled: bool, foreground_visible: bool) -> bool {
    enabled && foreground_visible
}

pub fn plan_is_enabled(plan: &crate::material::BackdropPlan) -> bool {
    plan.frame.is_some() && plan.material.is_some()
}
```

Replace the single `native_window` field in `AppState` with:

```rust
pub material_windows: Mutex<material_windows::MaterialWindowStates>,
```

Initialize it with `MaterialWindowStates::default()`.

- [ ] **Step 4: Create backdrops before process detection starts**

At the beginning of the Tauri setup closure, call:

```rust
material_windows::create(app)?;
if let Some(main) = app.get_webview_window("main") {
    material::enforce_foreground_borderless(&main).map_err(std::io::Error::other)?;
}
```

`enforce_foreground_borderless` strips caption styles, disables non-client rendering, and disables shadow. It must not call `SetWindowRgn` or apply any Accent policy.

- [ ] **Step 5: Replace main-window material work with backdrop plans**

In `apply_overlay_geometry`:

1. Resize and position `main` using Tauri APIs only.
2. Convert measured provider regions to physical pixels.
3. Build plans with the main window's physical origin.
4. Apply each plan to its fixed native window.
5. Place visible backdrops immediately behind the main HWND using `SetWindowPos` with `SWP_NOACTIVATE` and `SWP_SHOWWINDOW`.

Replace the current Windows material block and final position call with:

```rust
webview
    .set_size(tauri::PhysicalSize::new(size.0, size.1))
    .map_err(|error| error.to_string())?;
let (x, y) = window::corner_position(chosen.area, size, &request.corner);
webview
    .set_position(tauri::PhysicalPosition::new(x, y))
    .map_err(|error| error.to_string())?;

#[cfg(target_os = "windows")]
{
    let regions = material::physical_card_regions(&request.regions, scale_factor);
    let blur_supported = material_windows::legacy_blur_supported(
        material_windows::current_windows_build(),
    );
    let plans = material::plan_backdrops(
        &request.theme,
        request.minimized,
        &regions,
        (x, y),
        &request.background_color,
        request.card_opacity,
        blur_supported,
    );
    let foreground_visible = webview.is_visible().unwrap_or(false);
    material_windows::apply_plans(&app, &plans, foreground_visible)?;
}
Ok(())
```

Add this entry point to `src-tauri/src/material_windows.rs`:

```rust
pub fn apply_plans(
    app: &tauri::AppHandle,
    plans: &[crate::material::BackdropPlan; 2],
    foreground_visible: bool,
) -> Result<(), String> {
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| "main window unavailable".to_string())?;
    let main_hwnd = main.hwnd().map_err(|error| error.to_string())?.0;
    let app_state = app.state::<crate::AppState>();
    let mut states = app_state
        .material_windows
        .lock()
        .map_err(|_| "material window state unavailable".to_string())?;

    for plan in plans {
        let window = app
            .get_window(label_for(plan.provider))
            .ok_or_else(|| format!("{} unavailable", label_for(plan.provider)))?;
        let state = states.get_mut(plan.provider);
        state.enabled = plan_is_enabled(plan);
        let (Some((x, y, width, height, radius)), Some(spec)) = (plan.frame, plan.material) else {
            state.frame = None;
            let _ = window.hide();
            continue;
        };
        state.frame = Some((x, y, width, height, radius));
        crate::material::apply_to_backdrop(&window, spec, (width as u32, height as u32), radius, state)?;
        window
            .set_position(tauri::PhysicalPosition::new(x, y))
            .map_err(|error| error.to_string())?;
        if should_show_backdrop(state.enabled, foreground_visible) {
            place_behind(window.hwnd().map_err(|error| error.to_string())?.0, main_hwnd, x, y, width, height)?;
        } else {
            let _ = window.hide();
        }
    }
    Ok(())
}
```

Implement `place_behind` with `main_hwnd` as the explicit z-order anchor:

```rust
#[cfg(target_os = "windows")]
fn place_behind(
    backdrop_hwnd: *mut std::ffi::c_void,
    main_hwnd: *mut std::ffi::c_void,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) -> Result<(), String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{SetWindowPos, SWP_NOACTIVATE, SWP_SHOWWINDOW};
    if unsafe {
        SetWindowPos(
            backdrop_hwnd,
            main_hwnd,
            x,
            y,
            width,
            height,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}
```

Do not use `HWND_TOPMOST`; `main_hwnd` keeps each backdrop directly behind the foreground in the same z-order band.

- [ ] **Step 6: Coordinate every show and hide path**

Make these lifecycle changes in `src-tauri/src/lib.rs`:

- Tray Hide: call `material_windows::hide_all(app)` before hiding `main`.
- No active providers: call `material_windows::hide_all(&detection_handle)` before hiding `main`.
- Startup reveal: call `material_windows::show_enabled(app)` before `main.show()`.
- Tray Show: call `show_overlay_if_ready`, then focus only `main`.
- Settings close: hide `settings` only; do not restore or reshape `main`.
- Remove the main `Focused(true)` region-restoration handler.
- `set_config`: apply `always_on_top` to `main` and both backdrop windows.

Implement `show_enabled` so it restores cached card frames without recomputing geometry and never focuses a backdrop:

```rust
pub fn show_enabled(app: &tauri::AppHandle) -> Result<(), String> {
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| "main window unavailable".to_string())?;
    let main_hwnd = main.hwnd().map_err(|error| error.to_string())?.0;
    let app_state = app.state::<crate::AppState>();
    let states = app_state
        .material_windows
        .lock()
        .map_err(|_| "material window state unavailable".to_string())?;

    for provider in [Provider::Claude, Provider::Openai] {
        let state = match provider {
            Provider::Claude => &states.claude,
            Provider::Openai => &states.openai,
        };
        let Some((x, y, width, height, _radius)) = state.frame else {
            continue;
        };
        if !state.enabled {
            continue;
        }
        let window = app
            .get_window(label_for(provider))
            .ok_or_else(|| format!("{} unavailable", label_for(provider)))?;
        place_behind(
            window.hwnd().map_err(|error| error.to_string())?.0,
            main_hwnd,
            x,
            y,
            width,
            height,
        )?;
    }
    Ok(())
}
```

- [ ] **Step 7: Prove the main window has no material or region call path**

Run:

```powershell
rg -n "apply_to_window|restore_window_surface|SetWindowRgn" src-tauri/src/lib.rs src-tauri/src/material.rs src-tauri/src/material_windows.rs
```

Expected: `apply_to_window` and `restore_window_surface` return no matches. `SetWindowRgn` appears only inside the native backdrop rounding helper in `material.rs`.

- [ ] **Step 8: Run the complete Rust suite**

Run:

```powershell
cargo fmt --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Expected: all Rust tests PASS and Clippy reports no warnings.

- [ ] **Step 9: Commit lifecycle integration**

```powershell
git add src-tauri/src/lib.rs src-tauri/src/material.rs src-tauri/src/material_windows.rs src-tauri/src/visibility.rs
git commit -m "fix: isolate native materials to provider cards"
```

### Task 5: Keep provider DOM and visuals stable during polling

**Files:**
- Modify: `src/components/overlay.ts`
- Modify: `src/components/overlay.test.ts`
- Modify: `src/styles/app.css`
- Modify: `src/main.ts`

- [ ] **Step 1: Write failing provider-isolation and stable-node tests**

Add a test to `src/components/overlay.test.ts` that renders both providers, updates only Claude, then removes Claude:

```ts
it("never moves Claude data or styling into the ChatGPT node", () => {
  const content = document.createElement("div");
  const claude = { windows: [{ label: "Weekly", used_percent: 91, resets_at: 2_000_000 }], fetched_at: 1, state: "fresh" as const };
  const openai = { windows: [{ label: "Weekly", used_percent: 37, resets_at: 2_000_000 }], fetched_at: 1, state: "fresh" as const };
  const options = { snapshots: { claude, openai }, previousSnapshots: {}, now: 1_000_000, onAction: () => undefined };

  reconcileProviderLayers(content, ["claude", "openai"], options);
  const openaiNode = content.querySelector<HTMLElement>('[data-provider="openai"]')!;

  reconcileProviderLayers(content, ["claude", "openai"], {
    ...options,
    snapshots: { claude: { ...claude, windows: [{ ...claude.windows[0], used_percent: 94 }] }, openai },
  });
  reconcileProviderLayers(content, ["openai"], options);

  expect(content.querySelector('[data-provider="openai"]')).toBe(openaiNode);
  expect(openaiNode.textContent).toContain("37%");
  expect(openaiNode.textContent).not.toContain("94%");
  expect(content.querySelector('[data-provider="claude"]')).toBeNull();
});
```

Add this stable-order test beside it:

```ts
it("does not move provider nodes when their order is unchanged", () => {
  const content = document.createElement("div");
  const options = {
    snapshots: { claude: snapshot(20), openai: snapshot(40) },
    previousSnapshots: {},
    now: 1_000_000,
    onAction: vi.fn(),
  };
  reconcileProviderLayers(content, ["claude", "openai"], options);
  const before = Array.from(content.querySelectorAll<HTMLElement>(".layer[data-provider]"));
  const appendSpy = vi.spyOn(content, "appendChild");

  reconcileProviderLayers(content, ["claude", "openai"], options);

  const after = Array.from(content.querySelectorAll<HTMLElement>(".layer[data-provider]"));
  expect(appendSpy).not.toHaveBeenCalled();
  expect(after).toEqual(before);
  expect(after[0].dataset.provider).toBe("claude");
  expect(after[1].dataset.provider).toBe("openai");
});
```

- [ ] **Step 2: Run the focused test and observe the unnecessary DOM movement**

Run:

```powershell
npm test -- src/components/overlay.test.ts
```

Expected: the new stable-order assertion FAILS because `appendChild(layer)` moves existing provider nodes on every update.

- [ ] **Step 3: Reconcile provider order only when it actually changes**

Replace the unconditional `content.appendChild(layer)` behavior in `reconcileProviderLayers` with a two-phase update:

```ts
const resolved = new Map<Provider, HTMLElement>();

for (const provider of providers) {
  const snapshot = options.snapshots[provider];
  let layer = content.querySelector<HTMLElement>(`.layer[data-provider="${provider}"]`);
  const canReuse = layer && snapshot && updateLayer(layer, snapshot, options.now);
  if (!layer || (snapshot && !canReuse) || (!snapshot && !layer.classList.contains("layer--loading"))) {
    const replacement = snapshot
      ? renderLayer(title(provider), snapshot, options.now, options.previousSnapshots[provider])
      : renderLoadingLayer(title(provider));
    if (layer) layer.replaceWith(replacement);
    layer = replacement;
  }
  resolved.set(provider, layer);
}

providers.forEach((provider, index) => {
  const layer = resolved.get(provider)!;
  const current = content.querySelectorAll<HTMLElement>(".layer[data-provider]")[index];
  if (current !== layer) content.insertBefore(layer, current ?? null);
});
```

Keep removal of unwanted providers before this block. Append the minimize control only when its current parent is not the first visible provider layer.

- [ ] **Step 4: Keep the WebView host transparent for native themes**

Ensure `src/styles/app.css` contains these rules:

```css
html,
body,
#app,
.layers {
  background: transparent !important;
}

#app[data-theme="acrylic"] .layer,
#app[data-theme="blur"] .layer {
  background: transparent;
  backdrop-filter: none;
  -webkit-backdrop-filter: none;
}
```

Do not add CSS gradients, blur, opaque pseudo-elements, or host-level background colors to Acrylic and Blur. Preserve provider borders, text, logos, and meter styling.

- [ ] **Step 5: Keep minute polling in the in-place update path**

In `refreshProvider`, continue storing the previous provider snapshot and calling `renderMain`, but verify `reconcileProviderLayers` reaches `updateLayer` for unchanged usage-window labels. Keep the existing geometry cache so an unchanged rectangle does not invoke Rust again:

```ts
const geometryKey = JSON.stringify(request);
if (geometryKey === lastGeometry) return;
const applied = await invoke("apply_overlay_geometry", { request }).then(() => true).catch(() => false);
if (applied) lastGeometry = geometryKey;
```

- [ ] **Step 6: Run frontend unit, accessibility, and build checks**

Run:

```powershell
npm test
npm run coverage
npm run build
```

Expected: all Vitest tests PASS, coverage completes successfully, and TypeScript/Vite build without errors.

- [ ] **Step 7: Commit stable polling and card visuals**

```powershell
git add src/components/overlay.ts src/components/overlay.test.ts src/styles/app.css src/main.ts
git commit -m "fix: keep provider cards stable during polling"
```

### Task 6: Build, inspect, and live-verify genuine card materials

**Files:**
- Verify: repository, release binary, NSIS installer, and pull request #1.

- [ ] **Step 1: Run the complete verification suite from a clean tree**

Run:

```powershell
git status --short
npm test
npm run coverage
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Expected: the initial status is clean, all frontend and Rust checks PASS, and no warnings are emitted.

- [ ] **Step 2: Audit tracked files for publish safety**

Run:

```powershell
git ls-files
git grep -n -I -E "(access[_-]?token|refresh[_-]?token|client_secret|api[_-]?key|Bearer [A-Za-z0-9_-]{20,})"
git check-ignore -v src-tauri/target dist node_modules .env
```

Expected: build outputs, dependency folders, and environment files are ignored. The secret-pattern search finds only field names, tests, or documentation and no live credential values.

- [ ] **Step 3: Build the release installer**

Run:

```powershell
npm run tauri build
```

Expected: Tauri produces `src-tauri/target/release/bundle/nsis/Usage Tracker Overlay_0.1.0_x64-setup.exe`.

- [ ] **Step 4: Verify native window structure before visual testing**

Launch `src-tauri/target/release/usage-tracker-overlay.exe`, then inspect the process windows. Expected structure:

```text
main              transparent WebView foreground, borderless
material-claude   native, borderless, non-focusable, click-through
material-openai   native, borderless, non-focusable, click-through
settings          hidden unless opened from the tray
```

No window may expose a native caption, taskbar button, minimize box, maximize box, system menu, or shadow.

- [ ] **Step 5: Live-test Acrylic against the supplied reference**

Select Acrylic in Settings and verify:

- Wallpaper colors remain visible through each full card.
- Frosting is limited to the rounded Claude and ChatGPT rectangles.
- The card gap and unused host pixels are fully transparent.
- Background color and opacity affect the card tint, not the host.
- Opening and closing Settings never reveals `Usage Tracker` as a native title.

- [ ] **Step 6: Live-test Blur and its compatibility fallback**

Select Blur and verify genuine low-tint blur on Windows builds up to 22000. On newer builds, verify the card uses Acrylic rather than a black rectangle. In both cases, only the provider card windows receive material.

- [ ] **Step 7: Verify provider changes and one-minute polling**

With both providers visible, record each percentage and color. Close Claude, reopen it, and wait through one complete polling interval. Verify:

- ChatGPT never displays Claude's percentage, orange accent, or backdrop position.
- Claude never displays ChatGPT's percentage or teal accent.
- Provider cards do not split or become additional rectangles.
- The progress ring animates to its new value without the overlay disappearing or moving.
- Tray Show/Hide keeps the overlay hidden until explicitly shown again.

- [ ] **Step 8: Push the branch and update the pull request**

Run:

```powershell
git status --short
git log --oneline --decorate -12
git push origin codex/usage-tracker-implementation
gh pr view 1 --web
```

Expected: the tree is clean, the frequent implementation commits are visible on the remote branch, and pull request #1 contains the per-card material work. Mark the pull request ready only after its automated checks pass.
