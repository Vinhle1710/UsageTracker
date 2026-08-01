use crate::material::{BackdropPlan, NativeWindowState};
use crate::model::Provider;
use tauri::Manager;

pub const CLAUDE_LABEL: &str = "material-claude";
pub const OPENAI_LABEL: &str = "material-openai";
const PROVIDERS: [Provider; 2] = [Provider::Claude, Provider::Openai];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MaterialWindowStates {
    pub claude: NativeWindowState,
    pub openai: NativeWindowState,
}

impl MaterialWindowStates {
    pub fn get(&self, provider: Provider) -> &NativeWindowState {
        match provider {
            Provider::Claude => &self.claude,
            Provider::Openai => &self.openai,
        }
    }

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
    for provider in PROVIDERS {
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

fn hide_providers(app: &tauri::AppHandle, providers: &[Provider]) -> Result<(), String> {
    let mut first_error = None;
    for provider in providers {
        if let Some(window) = app.get_window(MaterialWindowStates::label_for(*provider)) {
            if let Err(error) = window.hide() {
                first_error.get_or_insert_with(|| error.to_string());
            }
        }
    }
    first_error.map_or(Ok(()), Err)
}

fn hide_all_unlocked(app: &tauri::AppHandle) -> Result<(), String> {
    hide_providers(app, &PROVIDERS)
}

pub fn hide_all(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<crate::AppState>();
    let _states = state
        .material_windows
        .lock()
        .map_err(|error| format!("material window state lock poisoned: {error}"))?;
    hide_all_unlocked(app)
}

pub fn set_always_on_top(app: &tauri::AppHandle, always_on_top: bool) -> Result<(), String> {
    let state = app.state::<crate::AppState>();
    let _states = state
        .material_windows
        .lock()
        .map_err(|error| format!("material window state lock poisoned: {error}"))?;
    for provider in PROVIDERS {
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

fn staged_state(current: &NativeWindowState, plan: &BackdropPlan) -> NativeWindowState {
    let mut next = current.clone();
    next.enabled = plan_is_enabled(plan);
    next.frame = plan.frame;
    next
}

fn error_with_cleanup(app: &tauri::AppHandle, error: String) -> Result<(), String> {
    match hide_all_unlocked(app) {
        Ok(()) => Err(error),
        Err(cleanup_error) => Err(format!("{error}; backdrop cleanup failed: {cleanup_error}")),
    }
}

fn commit_staged_states(
    current: &mut MaterialWindowStates,
    staged: MaterialWindowStates,
    result: Result<(), String>,
) -> Result<(), String> {
    if result.is_ok() {
        *current = staged;
    }
    result
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

    let mut staged_states = MaterialWindowStates {
        claude: states.claude.clone(),
        openai: states.openai.clone(),
    };
    let result = (|| {
        for provider in PROVIDERS {
            let plan = plan_for(&plans, provider)?;
            let label = MaterialWindowStates::label_for(provider);
            let mut next = staged_state(staged_states.get(provider), plan);

            let Some(frame) = plan.frame else {
                if let Some(window) = app.get_window(label) {
                    window.hide().map_err(|error| error.to_string())?;
                }
                *staged_states.get_mut(provider) = next;
                continue;
            };
            let Some(material) = plan.material else {
                if let Some(window) = app.get_window(label) {
                    window.hide().map_err(|error| error.to_string())?;
                }
                *staged_states.get_mut(provider) = next;
                continue;
            };
            let window = app
                .get_window(label)
                .ok_or_else(|| format!("missing native backdrop window {label}"))?;
            let size = (
                u32::try_from(frame.2).map_err(|_| "backdrop width is negative".to_string())?,
                u32::try_from(frame.3).map_err(|_| "backdrop height is negative".to_string())?,
            );
            crate::material::apply_to_backdrop(&window, material, size, frame.4, &mut next)?;
            if should_show_backdrop(next.enabled, foreground_visible) {
                let backdrop_hwnd = window.hwnd().map_err(|error| error.to_string())?.0;
                let backdrop_hwnd = backdrop_hwnd as windows_sys::Win32::Foundation::HWND;
                place_backdrop_behind_main(backdrop_hwnd, main_hwnd, frame)?;
            } else {
                window.hide().map_err(|error| error.to_string())?;
            }
            *staged_states.get_mut(provider) = next;
        }
        Ok(())
    })();
    match commit_staged_states(&mut states, staged_states, result) {
        Ok(()) => Ok(()),
        Err(error) => error_with_cleanup(app, error),
    }
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

    let result = (|| {
        for provider in PROVIDERS {
            let current = states.get(provider);
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
    })();
    match result {
        Ok(()) => Ok(()),
        Err(error) => error_with_cleanup(app, error),
    }
}

#[cfg(not(target_os = "windows"))]
pub fn show_enabled(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<crate::AppState>();
    let _states = state
        .material_windows
        .lock()
        .map_err(|error| format!("material window state lock poisoned: {error}"))?;
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

    #[test]
    fn staging_a_plan_keeps_cached_state_unchanged_until_commit() {
        let cached = NativeWindowState {
            material: Some(crate::material::NativeMaterialSpec {
                material: crate::material::Material::Acrylic,
                tint: (7, 16, 31, 96),
            }),
            size: Some((300, 80)),
            radius: Some(12),
            frame: Some((10, 20, 300, 80, 12)),
            enabled: true,
        };
        let plan = BackdropPlan {
            provider: Provider::Claude,
            frame: Some((30, 40, 320, 90, 14)),
            material: None,
        };

        let staged = staged_state(&cached, &plan);

        assert!(cached.enabled);
        assert_eq!(cached.frame, Some((10, 20, 300, 80, 12)));
        assert!(!staged.enabled);
        assert_eq!(staged.frame, plan.frame);
        assert_eq!(staged.material, cached.material);
        assert_eq!(staged.size, cached.size);
        assert_eq!(staged.radius, cached.radius);
    }

    #[test]
    fn staged_provider_states_commit_all_or_nothing() {
        let mut original = MaterialWindowStates::default();
        original.claude.enabled = true;
        original.claude.frame = Some((10, 20, 300, 80, 12));
        original.openai.enabled = true;
        original.openai.frame = Some((10, 110, 300, 120, 12));

        let mut staged = original.clone();
        staged.claude.frame = Some((30, 40, 320, 90, 14));
        staged.openai.frame = Some((30, 140, 320, 140, 14));

        let mut current = original.clone();
        assert!(commit_staged_states(
            &mut current,
            staged.clone(),
            Err("provider operation failed".to_string()),
        )
        .is_err());
        assert_eq!(current, original);

        assert!(commit_staged_states(&mut current, staged.clone(), Ok(())).is_ok());
        assert_eq!(current, staged);
    }
}
