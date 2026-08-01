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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackdropPlan {
    pub provider: crate::model::Provider,
    pub frame: Option<(i32, i32, i32, i32, i32)>,
    pub material: Option<NativeMaterialSpec>,
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

pub fn physical_card_regions(regions: &[LogicalCardRegion], scale_factor: f64) -> Vec<CardRegion> {
    regions
        .iter()
        .map(|region| CardRegion {
            provider: region.provider,
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
        "clear" => Material::Clear,
        "blur" => Material::Blur,
        "solid" => Material::Solid,
        _ => Material::Acrylic,
    }
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
    let material = (!minimized)
        .then(|| resolved_material(theme, blur_supported))
        .flatten();
    let plan_for = |provider| {
        let region = regions.iter().find(|region| region.provider == provider);
        let frame = region.map(|region| {
            (
                origin.0 + region.x,
                origin.1 + region.y,
                region.width,
                region.height,
                region.radius,
            )
        });
        let spec = material.and_then(|material| {
            frame.map(|_| NativeMaterialSpec {
                material,
                tint: parse_color(color, material_alpha(material, opacity)).unwrap_or((
                    7,
                    16,
                    31,
                    material_alpha(material, opacity),
                )),
            })
        });
        BackdropPlan {
            provider,
            frame,
            material: spec,
        }
    };
    [
        plan_for(crate::model::Provider::Claude),
        plan_for(crate::model::Provider::Openai),
    ]
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

pub fn should_apply_card_region(
    shape_changed: bool,
    frame_repaired: bool,
    surface_invalidated: bool,
) -> bool {
    shape_changed || frame_repaired || surface_invalidated
}

#[cfg(target_os = "windows")]
pub fn enforce_borderless(window: &tauri::WebviewWindow) -> Result<bool, String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW, SetWindowLongPtrW, SetWindowPos, GWL_STYLE, SWP_FRAMECHANGED,
        SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER,
    };

    let hwnd = window.hwnd().map_err(|error| error.to_string())?.0;
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        let stripped = borderless_style(style);
        if !frame_repair_required(style) {
            return Ok(false);
        }
        SetWindowLongPtrW(hwnd, GWL_STYLE, stripped as isize);
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
    }
    disable_non_client_rendering(window)?;
    window
        .set_shadow(false)
        .map_err(|error| error.to_string())?;
    Ok(true)
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
            CombineRgn(combined, combined, card, RGN_OR);
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
pub fn restore_window_surface(
    window: &tauri::WebviewWindow,
    current: &NativeWindowState,
    force_region: bool,
) -> Result<(), String> {
    let frame_repaired = enforce_borderless(window)?;
    if should_apply_card_region(false, frame_repaired, force_region) && !current.regions.is_empty()
    {
        apply_card_region(window, &current.regions)?;
    }
    Ok(())
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
    disable_non_client_rendering(window)?;
    let frame_repaired = enforce_borderless(window)?;
    if plan.resize_window {
        window
            .set_size(tauri::PhysicalSize::new(size.0, size.1))
            .map_err(|error| error.to_string())?;
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
    if should_apply_card_region(plan.reshape_window, frame_repaired, false) {
        apply_card_region(window, regions)?;
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
    fn maps_supported_materials_without_guessing() {
        assert_eq!(material_for_theme("clear"), Material::Clear);
        assert_eq!(material_for_theme("acrylic"), Material::Acrylic);
        assert_eq!(material_for_theme("blur"), Material::Blur);
        assert_eq!(material_for_theme("solid"), Material::Solid);
        assert_eq!(material_for_theme("unknown"), Material::Acrylic);
    }

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
            CardRegion {
                provider: crate::model::Provider::Openai,
                x: 8,
                y: 90,
                width: 310,
                height: 160,
                radius: 14,
            },
            CardRegion {
                provider: crate::model::Provider::Claude,
                x: 8,
                y: 8,
                width: 310,
                height: 70,
                radius: 14,
            },
        ];
        let plans = plan_backdrops("acrylic", false, &regions, (100, 200), "#07101f", 0.9, true);

        assert_eq!(plans[0].provider, crate::model::Provider::Claude);
        assert_eq!(plans[0].frame, Some((108, 208, 310, 70, 14)));
        assert_eq!(plans[1].provider, crate::model::Provider::Openai);
        assert_eq!(plans[1].frame, Some((108, 290, 310, 160, 14)));
    }

    #[test]
    fn minimized_overlay_hides_both_native_backdrops() {
        let regions = vec![CardRegion {
            provider: crate::model::Provider::Claude,
            x: 8,
            y: 8,
            width: 310,
            height: 70,
            radius: 14,
        }];
        let plans = plan_backdrops("blur", true, &regions, (0, 0), "#07101f", 0.9, true);
        assert!(plans.iter().all(|plan| plan.material.is_none()));
    }

    #[test]
    fn invalid_acrylic_color_uses_opacity_mapped_fallback_alpha() {
        let regions = vec![CardRegion {
            provider: crate::model::Provider::Claude,
            x: 8,
            y: 8,
            width: 310,
            height: 70,
            radius: 14,
        }];
        let plans = plan_backdrops("acrylic", false, &regions, (0, 0), "invalid", 1.0, true);

        assert_eq!(plans[0].material.unwrap().tint, (7, 16, 31, 128));
    }

    #[test]
    fn parses_card_color_and_clamps_alpha() {
        assert_eq!(parse_tint("#07101f", 0.84), Some((7, 16, 31, 214)));
        assert_eq!(parse_tint("#ffffff", 2.0), Some((255, 255, 255, 255)));
        assert_eq!(parse_tint("navy", 0.8), None);
    }

    #[test]
    fn unchanged_native_state_does_not_reset_the_material_or_shape() {
        let regions = vec![CardRegion {
            provider: crate::model::Provider::Claude,
            x: 8,
            y: 8,
            width: 310,
            height: 168,
            radius: 14,
        }];
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
    fn logical_card_measurements_follow_the_monitor_scale_factor() {
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

    #[test]
    fn native_non_client_rendering_is_disabled() {
        assert_eq!(non_client_rendering_policy(), 1);
    }

    #[test]
    fn acrylic_and_blur_use_translucenttb_accent_states() {
        assert_eq!(accent_policy(Material::Acrylic, (7, 16, 31, 214)).state, 4);
        assert_eq!(accent_policy(Material::Blur, (7, 16, 31, 214)).state, 3);
        assert_eq!(accent_policy(Material::Solid, (7, 16, 31, 214)).state, 0);
    }
}
