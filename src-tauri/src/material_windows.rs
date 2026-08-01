use crate::material::{BackdropPlan, NativeWindowState};
use crate::model::Provider;
use tauri::Manager;

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

    pub fn label_for(provider: Provider) -> &'static str {
        match provider {
            Provider::Claude => CLAUDE_LABEL,
            Provider::Openai => OPENAI_LABEL,
        }
    }
}

pub fn should_show_backdrop(enabled: bool, foreground_visible: bool) -> bool {
    enabled && foreground_visible
}

pub fn plan_is_enabled(plan: &BackdropPlan) -> bool {
    plan.frame.is_some() && plan.material.is_some()
}

pub fn create(app: &tauri::App) -> Result<(), String> {
    for provider in [Provider::Claude, Provider::Openai] {
        let label = MaterialWindowStates::label_for(provider);
        let window = tauri::WindowBuilder::new(app, label)
            .visible(false)
            .decorations(false)
            .transparent(true)
            .shadow(false)
            .resizable(false)
            .skip_taskbar(true)
            .always_on_top(true)
            .focused(false)
            .focusable(false)
            .position(-32000.0, -32000.0)
            .inner_size(1.0, 1.0)
            .build()
            .map_err(|error| error.to_string())?;
        window
            .set_ignore_cursor_events(true)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn hide_all(app: &tauri::AppHandle) -> Result<(), String> {
    for provider in [Provider::Claude, Provider::Openai] {
        if let Some(window) = app.get_window(MaterialWindowStates::label_for(provider)) {
            window.hide().map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

pub fn set_always_on_top(app: &tauri::AppHandle, always_on_top: bool) -> Result<(), String> {
    for provider in [Provider::Claude, Provider::Openai] {
        if let Some(window) = app.get_window(MaterialWindowStates::label_for(provider)) {
            window
                .set_always_on_top(always_on_top)
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn plan_for(plans: &[BackdropPlan; 2], provider: Provider) -> Result<&BackdropPlan, String> {
    plans
        .iter()
        .find(|plan| plan.provider == provider)
        .ok_or_else(|| format!("missing backdrop plan for {provider:?}"))
}

#[cfg(target_os = "windows")]
fn place_backdrop_behind_main(
    backdrop_hwnd: windows_sys::Win32::Foundation::HWND,
    main_hwnd: windows_sys::Win32::Foundation::HWND,
    frame: (i32, i32, i32, i32, i32),
) -> Result<(), String> {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, SWP_NOACTIVATE, SWP_SHOWWINDOW,
    };

    let (x, y, width, height, _) = frame;
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

#[cfg(target_os = "windows")]
pub fn apply_plans(
    app: &tauri::AppHandle,
    plans: [BackdropPlan; 2],
    foreground_visible: bool,
) -> Result<(), String> {
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| "no main window".to_string())?;
    let main_hwnd = main.hwnd().map_err(|error| error.to_string())?.0;
    let main_hwnd = main_hwnd as windows_sys::Win32::Foundation::HWND;
    let state = app.state::<crate::AppState>();
    let mut states = state
        .material_windows
        .lock()
        .map_err(|error| format!("material window state lock poisoned: {error}"))?;

    for provider in [Provider::Claude, Provider::Openai] {
        let plan = plan_for(&plans, provider)?;
        let label = MaterialWindowStates::label_for(provider);
        let enabled = plan_is_enabled(plan);
        let current = states.get_mut(provider);
        current.enabled = enabled;
        current.frame = plan.frame;

        let Some(frame) = plan.frame else {
            if let Some(window) = app.get_window(label) {
                window.hide().map_err(|error| error.to_string())?;
            }
            continue;
        };
        let Some(material) = plan.material else {
            if let Some(window) = app.get_window(label) {
                window.hide().map_err(|error| error.to_string())?;
            }
            continue;
        };
        let window = app
            .get_window(label)
            .ok_or_else(|| format!("missing native backdrop window {label}"))?;
        let size = (
            u32::try_from(frame.2).map_err(|_| "backdrop width is negative".to_string())?,
            u32::try_from(frame.3).map_err(|_| "backdrop height is negative".to_string())?,
        );
        crate::material::apply_to_backdrop(&window, material, size, frame.4, current)?;
        if should_show_backdrop(current.enabled, foreground_visible) {
            let backdrop_hwnd = window.hwnd().map_err(|error| error.to_string())?.0;
            let backdrop_hwnd = backdrop_hwnd as windows_sys::Win32::Foundation::HWND;
            place_backdrop_behind_main(backdrop_hwnd, main_hwnd, frame)?;
        } else {
            window.hide().map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn apply_plans(
    app: &tauri::AppHandle,
    plans: [BackdropPlan; 2],
    _foreground_visible: bool,
) -> Result<(), String> {
    let _ = plans;
    hide_all(app)
}

#[cfg(target_os = "windows")]
pub fn show_enabled(app: &tauri::AppHandle) -> Result<(), String> {
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| "no main window".to_string())?;
    let main_hwnd = main.hwnd().map_err(|error| error.to_string())?.0;
    let main_hwnd = main_hwnd as windows_sys::Win32::Foundation::HWND;
    let state = app.state::<crate::AppState>();
    let states = state
        .material_windows
        .lock()
        .map_err(|error| format!("material window state lock poisoned: {error}"))?;

    for provider in [Provider::Claude, Provider::Openai] {
        let current = match provider {
            Provider::Claude => &states.claude,
            Provider::Openai => &states.openai,
        };
        if !should_show_backdrop(current.enabled, true) {
            continue;
        }
        let Some(frame) = current.frame else {
            continue;
        };
        let label = MaterialWindowStates::label_for(provider);
        let window = app
            .get_window(label)
            .ok_or_else(|| format!("missing native backdrop window {label}"))?;
        let backdrop_hwnd = window.hwnd().map_err(|error| error.to_string())?.0;
        let backdrop_hwnd = backdrop_hwnd as windows_sys::Win32::Foundation::HWND;
        place_backdrop_behind_main(backdrop_hwnd, main_hwnd, frame)?;
    }
    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn show_enabled(_app: &tauri::AppHandle) -> Result<(), String> {
    Ok(())
}

pub fn legacy_blur_supported(build: u32) -> bool {
    build != 0 && build <= 22000
}

#[cfg(target_os = "windows")]
pub fn current_windows_build() -> u32 {
    #[repr(C)]
    struct RtlOsVersionInfoExW {
        dw_os_version_info_size: u32,
        dw_major_version: u32,
        dw_minor_version: u32,
        dw_build_number: u32,
        dw_platform_id: u32,
        sz_csd_version: [u16; 128],
        w_service_pack_major: u16,
        w_service_pack_minor: u16,
        w_suite_mask: u16,
        w_product_type: u8,
        w_reserved: u8,
    }

    type RtlGetVersion = unsafe extern "system" fn(*mut RtlOsVersionInfoExW) -> i32;

    let ntdll = unsafe {
        windows_sys::Win32::System::LibraryLoader::GetModuleHandleA(c"ntdll.dll".as_ptr().cast())
    };
    if ntdll.is_null() {
        return 0;
    }
    let Some(symbol) = (unsafe {
        windows_sys::Win32::System::LibraryLoader::GetProcAddress(
            ntdll,
            c"RtlGetVersion".as_ptr().cast(),
        )
    }) else {
        return 0;
    };

    let rtl_get_version: RtlGetVersion = unsafe { std::mem::transmute(symbol) };
    let mut version = RtlOsVersionInfoExW {
        dw_os_version_info_size: std::mem::size_of::<RtlOsVersionInfoExW>() as u32,
        dw_major_version: 0,
        dw_minor_version: 0,
        dw_build_number: 0,
        dw_platform_id: 0,
        sz_csd_version: [0; 128],
        w_service_pack_major: 0,
        w_service_pack_minor: 0,
        w_suite_mask: 0,
        w_product_type: 0,
        w_reserved: 0,
    };
    if unsafe { rtl_get_version(&mut version) } != 0 {
        return 0;
    }
    version.dw_build_number
}

#[cfg(not(target_os = "windows"))]
pub fn current_windows_build() -> u32 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_labels_are_fixed() {
        assert_eq!(CLAUDE_LABEL, "material-claude");
        assert_eq!(OPENAI_LABEL, "material-openai");
        assert_eq!(
            MaterialWindowStates::label_for(crate::model::Provider::Claude),
            CLAUDE_LABEL
        );
        assert_eq!(
            MaterialWindowStates::label_for(crate::model::Provider::Openai),
            OPENAI_LABEL
        );
    }

    #[test]
    fn provider_state_isolation_is_explicit() {
        let mut states = MaterialWindowStates::default();
        states.get_mut(crate::model::Provider::Claude).enabled = true;
        states.get_mut(crate::model::Provider::Openai).radius = Some(18);

        assert!(states.claude.enabled);
        assert!(!states.openai.enabled);
        assert_eq!(states.claude.radius, None);
        assert_eq!(states.openai.radius, Some(18));
    }

    #[test]
    fn legacy_blur_support_is_bounded_by_windows_build() {
        assert!(legacy_blur_supported(19045));
        assert!(legacy_blur_supported(22000));
        assert!(!legacy_blur_supported(22621));
        assert!(!legacy_blur_supported(0));
    }

    #[test]
    fn hidden_foreground_keeps_an_enabled_backdrop_hidden() {
        assert!(!should_show_backdrop(true, false));
        assert!(should_show_backdrop(true, true));
        assert!(!should_show_backdrop(false, true));
    }

    #[test]
    fn provider_removal_disables_the_removed_backdrop_plan() {
        let regions = [crate::material::CardRegion {
            provider: Provider::Claude,
            x: 8,
            y: 8,
            width: 310,
            height: 70,
            radius: 14,
        }];
        let plans = crate::material::plan_backdrops(
            "acrylic",
            false,
            &regions,
            (0, 0),
            "#07101f",
            0.9,
            true,
        );

        let claude_plan = plans
            .iter()
            .find(|plan| plan.provider == Provider::Claude)
            .unwrap();
        let openai_plan = plans
            .iter()
            .find(|plan| plan.provider == Provider::Openai)
            .unwrap();
        assert!(plan_is_enabled(claude_plan));
        assert!(!plan_is_enabled(openai_plan));
        assert_eq!(openai_plan.frame, None);
        assert_eq!(openai_plan.material, None);
    }
}
