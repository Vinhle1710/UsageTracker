#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Material {
    Clear,
    Acrylic,
    Blur,
    Solid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeMaterialSpec {
    pub material: Material,
    pub tint: (u8, u8, u8, u8),
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NativeWindowState {
    pub material: Option<NativeMaterialSpec>,
    pub regions: Vec<CardRegion>,
    pub size: Option<(u32, u32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeUpdatePlan {
    pub reapply_material: bool,
    pub reshape_window: bool,
    pub resize_window: bool,
    pub enforce_borderless: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AccentPolicy {
    pub state: u32,
    pub flags: u32,
    pub gradient_color: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CardRegion {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub radius: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogicalCardRegion {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub radius: f64,
}

pub fn physical_card_regions(regions: &[LogicalCardRegion], scale_factor: f64) -> Vec<CardRegion> {
    regions
        .iter()
        .map(|region| CardRegion {
            x: (region.x * scale_factor).round() as i32,
            y: (region.y * scale_factor).round() as i32,
            width: (region.width * scale_factor).round() as i32,
            height: (region.height * scale_factor).round() as i32,
            radius: (region.radius * scale_factor).round() as i32,
        })
        .collect()
}

pub fn non_client_rendering_policy() -> i32 {
    1
}

pub fn material_for_theme(theme: &str) -> Material {
    match theme {
        "solid" => Material::Solid,
        "clear" | "frosted" | "blur" => Material::Clear,
        _ => Material::Clear,
    }
}

pub fn parse_tint(color: &str, opacity: f32) -> Option<(u8, u8, u8, u8)> {
    let hex = color.strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let red = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let green = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let blue = u8::from_str_radix(&hex[4..6], 16).ok()?;
    let alpha = (opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
    Some((red, green, blue, alpha))
}

pub fn accent_policy(material: Material, tint: (u8, u8, u8, u8)) -> AccentPolicy {
    let state = match material {
        Material::Acrylic => 4,
        Material::Blur => 3,
        Material::Clear | Material::Solid => 0,
    };
    AccentPolicy {
        state,
        flags: if material == Material::Acrylic { 0 } else { 2 },
        gradient_color: (tint.0 as u32)
            | ((tint.1 as u32) << 8)
            | ((tint.2 as u32) << 16)
            | ((tint.3 as u32) << 24),
    }
}

pub fn plan_native_update(
    current: &NativeWindowState,
    material: NativeMaterialSpec,
    regions: &[CardRegion],
    size: (u32, u32),
) -> NativeUpdatePlan {
    NativeUpdatePlan {
        reapply_material: current.material != Some(material),
        reshape_window: current.regions != regions,
        resize_window: current.size != Some(size),
        enforce_borderless: true,
    }
}

pub fn borderless_style(style: u32) -> u32 {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            WS_CAPTION, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_SYSMENU, WS_THICKFRAME,
        };
        style & !(WS_CAPTION | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_SYSMENU)
    }
    #[cfg(not(target_os = "windows"))]
    {
        style
    }
}

pub fn frame_repair_required(style: u32) -> bool {
    borderless_style(style) != style
}

fn checked_window_style(value: isize, last_error: u32) -> Result<u32, String> {
    if value == 0 && last_error != 0 {
        return Err(std::io::Error::from_raw_os_error(last_error as i32).to_string());
    }
    Ok(value as u32)
}

pub fn should_apply_card_region(
    shape_changed: bool,
    frame_repaired: bool,
    surface_invalidated: bool,
) -> bool {
    shape_changed || frame_repaired || surface_invalidated
}

pub fn should_restore_cached_region(
    label: &str,
    surface_invalidated: bool,
    regions: &[CardRegion],
) -> bool {
    label == "main" && surface_invalidated && !regions.is_empty()
}

#[cfg(target_os = "windows")]
pub fn enforce_borderless(window: &tauri::WebviewWindow) -> Result<bool, String> {
    use windows_sys::Win32::Foundation::{GetLastError, SetLastError};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_STYLE, SWP_FRAMECHANGED,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
    };

    let hwnd = window.hwnd().map_err(|error| error.to_string())?.0;
    let mut frame_repaired = false;
    unsafe {
        SetLastError(0);
        let style = checked_window_style(GetWindowLongPtrW(hwnd, GWL_STYLE), GetLastError())?;
        let stripped = borderless_style(style);
        if frame_repair_required(style) {
            SetLastError(0);
            let previous = SetWindowLongPtrW(hwnd, GWL_STYLE, stripped as isize);
            if previous == 0 {
                let error = GetLastError();
                if error != 0 {
                    return Err(std::io::Error::from_raw_os_error(error as i32).to_string());
                }
            }
            if SetWindowPos(
                hwnd,
                std::ptr::null_mut(),
                0,
                0,
                0,
                0,
                SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            ) == 0
            {
                return Err(std::io::Error::last_os_error().to_string());
            }
            frame_repaired = true;
        }
    }
    disable_non_client_rendering(window)?;
    window
        .set_shadow(false)
        .map_err(|error| error.to_string())?;
    Ok(frame_repaired)
}

#[cfg(target_os = "windows")]
fn disable_non_client_rendering(window: &tauri::WebviewWindow) -> Result<(), String> {
    use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_NCRENDERING_POLICY};

    let hwnd = window.hwnd().map_err(|error| error.to_string())?.0;
    let policy = non_client_rendering_policy();
    let result = unsafe {
        DwmSetWindowAttribute(
            hwnd,
            DWMWA_NCRENDERING_POLICY as u32,
            (&policy as *const i32).cast(),
            std::mem::size_of_val(&policy) as u32,
        )
    };
    if result < 0 {
        return Err(format!(
            "DwmSetWindowAttribute failed with HRESULT {result:#x}"
        ));
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn apply_card_region(window: &tauri::WebviewWindow, regions: &[CardRegion]) -> Result<(), String> {
    use windows_sys::Win32::Graphics::Gdi::{
        CombineRgn, CreateRectRgn, CreateRoundRectRgn, DeleteObject, SetWindowRgn, RGN_OR,
    };

    let hwnd = window.hwnd().map_err(|error| error.to_string())?.0;
    unsafe {
        let combined = CreateRectRgn(0, 0, 0, 0);
        if combined.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        for region in regions {
            let card = CreateRoundRectRgn(
                region.x,
                region.y,
                region.x + region.width,
                region.y + region.height,
                region.radius * 2,
                region.radius * 2,
            );
            if card.is_null() {
                let _ = DeleteObject(combined);
                return Err(std::io::Error::last_os_error().to_string());
            }
            if CombineRgn(combined, combined, card, RGN_OR) == 0 {
                let error = std::io::Error::last_os_error();
                let _ = DeleteObject(card);
                let _ = DeleteObject(combined);
                return Err(error.to_string());
            }
            let _ = DeleteObject(card);
        }
        if SetWindowRgn(hwnd, combined, 1) == 0 {
            let _ = DeleteObject(combined);
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn repair_window_surface_windows(
    window: &tauri::WebviewWindow,
    label: &str,
    regions: &[CardRegion],
    force_region: bool,
) -> Result<(), String> {
    let frame_repaired = enforce_borderless(window)?;
    if should_restore_cached_region(label, frame_repaired || force_region, regions) {
        apply_card_region(window, regions)?;
    }
    Ok(())
}

pub fn repair_window_surface(
    window: &tauri::WebviewWindow,
    label: &str,
    regions: &[CardRegion],
    force_region: bool,
) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        repair_window_surface_windows(window, label, regions, force_region)
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = (window, label, regions, force_region);
        Ok(())
    }
}

#[cfg(target_os = "windows")]
pub fn restore_window_surface(
    window: &tauri::WebviewWindow,
    current: &NativeWindowState,
    force_region: bool,
) -> Result<(), String> {
    repair_window_surface(window, "main", &current.regions, force_region)
}

pub fn card_regions(
    size: (u32, u32),
    layout: &str,
    expanded_provider_count: usize,
    bubble_count: usize,
    scale: f32,
) -> Vec<CardRegion> {
    let padding = (8.0 * scale).round() as i32;
    let gap = (9.0 * scale).round() as i32;
    let radius = (14.0 * scale).round() as i32;
    let width = size.0 as i32 - padding * 2;
    let top = if bubble_count > 0 {
        (57.0 * scale).round() as i32
    } else {
        padding
    };
    let height = size.1 as i32 - top - padding;
    let count = expanded_provider_count.min(2);
    if count == 0 {
        return Vec::new();
    }
    if count == 1 {
        return vec![CardRegion {
            x: padding,
            y: top,
            width,
            height,
            radius,
        }];
    }

    if layout == "provider-columns" {
        let available = width - gap;
        let first = (available + 1) / 2;
        vec![
            CardRegion {
                x: padding,
                y: top,
                width: first,
                height,
                radius,
            },
            CardRegion {
                x: padding + first + gap,
                y: top,
                width: available - first,
                height,
                radius,
            },
        ]
    } else {
        let available = height - gap;
        let first = (available + 1) / 2;
        vec![
            CardRegion {
                x: padding,
                y: top,
                width,
                height: first,
                radius,
            },
            CardRegion {
                x: padding,
                y: top + first + gap,
                width,
                height: available - first,
                radius,
            },
        ]
    }
}

pub fn bubble_regions(
    size: (u32, u32),
    bubble_count: usize,
    expanded_provider_count: usize,
    scale: f32,
    corner: &str,
) -> Vec<CardRegion> {
    let count = bubble_count.min(2) as i32;
    if count == 0 {
        return Vec::new();
    }
    let padding = if expanded_provider_count == 0 {
        0
    } else {
        (8.0 * scale).round() as i32
    };
    let gap = (8.0 * scale).round() as i32;
    let diameter = (48.0 * scale).round() as i32;
    let row_width = count * diameter + (count - 1) * gap;
    let left = if corner.ends_with("left") {
        padding
    } else {
        size.0 as i32 - padding - row_width
    };
    (0..count)
        .map(|index| CardRegion {
            x: left + index * (diameter + gap),
            y: 0,
            width: diameter,
            height: diameter,
            radius: diameter / 2,
        })
        .collect()
}

#[cfg(target_os = "windows")]
pub fn apply_to_window(
    window: &tauri::WebviewWindow,
    desired: NativeMaterialSpec,
    regions: &[CardRegion],
    size: (u32, u32),
    current: &mut NativeWindowState,
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
    let plan = plan_native_update(current, desired, regions, size);
    let frame_repaired = enforce_borderless(window)?;
    if plan.resize_window {
        window
            .set_size(tauri::PhysicalSize::new(size.0, size.1))
            .map_err(|error| error.to_string())?;
    }
    if plan.reshape_window || frame_repaired {
        apply_card_region(window, regions)?;
    }
    let hwnd = window.hwnd().map_err(|error| error.to_string())?.0;
    if plan.reapply_material {
        let policy = accent_policy(desired.material, desired.tint);
        let mut native_policy = NativeAccentPolicy {
            state: policy.state,
            flags: policy.flags,
            gradient_color: policy.gradient_color,
            animation_id: 0,
        };
        let mut data = CompositionAttributeData {
            attribute: 0x13,
            data: &mut native_policy as *mut _ as _,
            size: std::mem::size_of::<NativeAccentPolicy>(),
        };
        use windows_sys::Win32::System::LibraryLoader::{GetModuleHandleA, GetProcAddress};

        type SetWindowCompositionAttributeFn =
            unsafe extern "system" fn(*mut std::ffi::c_void, *mut CompositionAttributeData) -> i32;

        let user32 = unsafe { GetModuleHandleA(c"user32.dll".as_ptr().cast()) };
        if user32.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let Some(symbol) =
            (unsafe { GetProcAddress(user32, c"SetWindowCompositionAttribute".as_ptr().cast()) })
        else {
            return Err("SetWindowCompositionAttribute is unavailable".to_string());
        };
        let set_window_composition_attribute: SetWindowCompositionAttributeFn =
            unsafe { std::mem::transmute(symbol) };
        let result = unsafe { set_window_composition_attribute(hwnd, &mut data) };
        if result == 0 {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    current.material = Some(desired);
    current.regions = regions.to_vec();
    current.size = Some(size);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_public_themes_to_native_materials() {
        assert_eq!(material_for_theme("clear"), Material::Clear);
        assert_eq!(material_for_theme("frosted"), Material::Clear);
        assert_eq!(material_for_theme("blur"), Material::Clear);
        assert_eq!(material_for_theme("solid"), Material::Solid);
        assert_eq!(material_for_theme("unknown"), Material::Clear);
    }

    #[test]
    fn parses_card_color_and_clamps_alpha() {
        assert_eq!(parse_tint("#07101f", 0.84), Some((7, 16, 31, 214)));
        assert_eq!(parse_tint("#ffffff", 2.0), Some((255, 255, 255, 255)));
        assert_eq!(parse_tint("navy", 0.8), None);
    }

    #[test]
    fn shapes_vertical_cards_without_covering_the_gap() {
        assert_eq!(
            card_regions((326, 360), "stacked-compact", 2, 0, 1.0),
            vec![
                CardRegion {
                    x: 8,
                    y: 8,
                    width: 310,
                    height: 168,
                    radius: 14
                },
                CardRegion {
                    x: 8,
                    y: 185,
                    width: 310,
                    height: 167,
                    radius: 14
                },
            ]
        );
    }

    #[test]
    fn shapes_horizontal_cards_and_bubble_row_regions() {
        assert_eq!(
            card_regions((620, 184), "provider-columns", 2, 0, 1.0).len(),
            2
        );
        assert_eq!(
            bubble_regions((104, 48), 2, 0, 1.0, "bottom-right"),
            vec![
                CardRegion {
                    x: 0,
                    y: 0,
                    width: 48,
                    height: 48,
                    radius: 24
                },
                CardRegion {
                    x: 56,
                    y: 0,
                    width: 48,
                    height: 48,
                    radius: 24
                },
            ]
        );
    }

    #[test]
    fn mixed_fallback_places_the_card_below_the_bubble_row() {
        assert_eq!(
            card_regions((326, 239), "stacked-compact", 1, 1, 1.0),
            vec![CardRegion {
                x: 8,
                y: 57,
                width: 310,
                height: 174,
                radius: 14,
            }]
        );
        assert_eq!(
            bubble_regions((326, 239), 1, 1, 1.0, "top-right"),
            vec![CardRegion {
                x: 270,
                y: 0,
                width: 48,
                height: 48,
                radius: 24,
            }]
        );
    }

    #[test]
    fn unchanged_native_state_does_not_reset_the_material_or_shape() {
        let regions = card_regions((326, 360), "stacked-compact", 2, 0, 1.0);
        let state = NativeWindowState {
            material: Some(NativeMaterialSpec {
                material: Material::Acrylic,
                tint: (7, 16, 31, 214),
            }),
            regions: regions.clone(),
            size: Some((326, 360)),
        };
        let plan = plan_native_update(
            &state,
            NativeMaterialSpec {
                material: Material::Acrylic,
                tint: (7, 16, 31, 214),
            },
            &regions,
            (326, 360),
        );
        assert!(!plan.reapply_material);
        assert!(!plan.reshape_window);
        assert!(!plan.resize_window);
        assert!(plan.enforce_borderless);
    }

    #[test]
    fn borderless_style_removes_every_native_caption_control() {
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            WS_CAPTION, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_SYSMENU, WS_THICKFRAME,
        };
        let native_frame =
            WS_CAPTION | WS_THICKFRAME | WS_MINIMIZEBOX | WS_MAXIMIZEBOX | WS_SYSMENU;

        assert_eq!(borderless_style(native_frame), 0);
        assert!(frame_repair_required(native_frame));
        assert!(!frame_repair_required(borderless_style(native_frame)));
    }

    #[test]
    fn a_frame_repair_restores_the_cached_card_region_last() {
        assert!(should_apply_card_region(false, true, false));
        assert!(should_apply_card_region(false, false, true));
        assert!(!should_apply_card_region(false, false, false));
    }

    #[test]
    fn cached_main_card_region_is_restored_after_repair_but_settings_stays_rectangular() {
        let cached = vec![CardRegion {
            x: 8,
            y: 8,
            width: 310,
            height: 168,
            radius: 14,
        }];

        assert!(should_restore_cached_region("main", true, &cached));
        assert!(!should_restore_cached_region("settings", true, &cached));
    }

    #[test]
    fn logical_card_measurements_follow_the_monitor_scale_factor() {
        let logical = vec![LogicalCardRegion {
            x: 6.4,
            y: 6.4,
            width: 248.0,
            height: 56.0,
            radius: 11.2,
        }];

        assert_eq!(
            physical_card_regions(&logical, 1.25),
            vec![CardRegion {
                x: 8,
                y: 8,
                width: 310,
                height: 70,
                radius: 14,
            }]
        );
    }

    #[test]
    fn native_non_client_rendering_is_disabled() {
        assert_eq!(non_client_rendering_policy(), 1);
    }

    #[test]
    fn zero_window_style_is_only_an_error_when_last_error_is_set() {
        assert_eq!(checked_window_style(0, 0), Ok(0));
        assert!(checked_window_style(0, 5).is_err());
        assert_eq!(checked_window_style(0x1234, 5), Ok(0x1234));
    }

    #[test]
    fn acrylic_and_blur_use_translucenttb_accent_states() {
        assert_eq!(accent_policy(Material::Acrylic, (7, 16, 31, 214)).state, 4);
        assert_eq!(accent_policy(Material::Blur, (7, 16, 31, 214)).state, 3);
        assert_eq!(accent_policy(Material::Solid, (7, 16, 31, 214)).state, 0);
    }
}
