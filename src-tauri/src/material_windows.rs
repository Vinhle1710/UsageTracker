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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GeometryVisibilityTransaction {
    foreground_visible: bool,
    staged_enabled: [bool; 2],
    staged_providers: [bool; 2],
    geometry_complete: bool,
}

impl GeometryVisibilityTransaction {
    fn new(foreground_visible: bool) -> Self {
        Self {
            foreground_visible,
            staged_enabled: [false; 2],
            staged_providers: [false; 2],
            geometry_complete: false,
        }
    }

    fn stage_provider(&mut self, provider: Provider, enabled: bool) {
        let slot = provider_slot(provider);
        self.staged_enabled[slot] = enabled;
        self.staged_providers[slot] = true;
    }

    fn complete_geometry(&mut self) {
        self.geometry_complete = true;
    }

    fn revealable(&self) -> Option<[bool; 2]> {
        (self.geometry_complete
            && self.foreground_visible
            && self.staged_providers.iter().all(|staged| *staged))
        .then_some(self.staged_enabled)
    }
}

fn provider_slot(provider: Provider) -> usize {
    match provider {
        Provider::Claude => 0,
        Provider::Openai => 1,
    }
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

fn reveal_providers(app: &tauri::AppHandle, enabled: [bool; 2]) -> Result<(), String> {
    let mut first_error = None;
    for (index, provider) in PROVIDERS.iter().enumerate() {
        if !enabled[index] {
            continue;
        }
        match app.get_window(MaterialWindowStates::label_for(*provider)) {
            Some(window) => {
                if let Err(error) = window.show() {
                    remember_first_error(&mut first_error, error.to_string());
                }
            }
            None => remember_first_error(
                &mut first_error,
                format!(
                    "missing native backdrop window {}",
                    MaterialWindowStates::label_for(*provider)
                ),
            ),
        }
    }
    first_error.map_or(Ok(()), Err)
}

pub(crate) fn hide_all_unlocked(app: &tauri::AppHandle) -> Result<(), String> {
    hide_providers(app, &PROVIDERS)
}

fn remember_first_error(first_error: &mut Option<String>, error: String) {
    first_error.get_or_insert(error);
}

fn hide_overlay_unlocked(app: &tauri::AppHandle) -> Result<(), String> {
    let mut first_error = None;
    if let Err(error) = hide_all_unlocked(app) {
        remember_first_error(&mut first_error, error);
    }
    if let Some(window) = app.get_webview_window("main") {
        if let Err(error) = window.hide() {
            remember_first_error(&mut first_error, error.to_string());
        }
    }
    first_error.map_or(Ok(()), Err)
}

pub fn hide_overlay(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<crate::AppState>();
    let _lifecycle = match state.native_lifecycle.lock() {
        Ok(lifecycle) => lifecycle,
        Err(error) => {
            return error_with_overlay_cleanup(
                app,
                format!("native lifecycle lock poisoned: {error}"),
            )
        }
    };
    hide_overlay_unlocked(app)
}

pub fn hide_all(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<crate::AppState>();
    let _lifecycle = match state.native_lifecycle.lock() {
        Ok(lifecycle) => lifecycle,
        Err(error) => {
            return error_with_cleanup(app, format!("native lifecycle lock poisoned: {error}"))
        }
    };
    hide_all_unlocked(app)
}

pub fn set_always_on_top(app: &tauri::AppHandle, always_on_top: bool) -> Result<(), String> {
    let state = app.state::<crate::AppState>();
    let _lifecycle = state
        .native_lifecycle
        .lock()
        .map_err(|error| format!("native lifecycle lock poisoned: {error}"))?;
    let main = app
        .get_webview_window("main")
        .ok_or_else(|| "no main window".to_string())?;
    let claude = app
        .get_window(CLAUDE_LABEL)
        .ok_or_else(|| format!("missing native backdrop window {CLAUDE_LABEL}"))?;
    let openai = app
        .get_window(OPENAI_LABEL)
        .ok_or_else(|| format!("missing native backdrop window {OPENAI_LABEL}"))?;
    let previous = [
        main.is_always_on_top().map_err(|error| error.to_string())?,
        claude
            .is_always_on_top()
            .map_err(|error| error.to_string())?,
        openai
            .is_always_on_top()
            .map_err(|error| error.to_string())?,
    ];
    let transaction = AlwaysOnTopTransaction::new(previous, always_on_top);
    let staged = transaction.staged();
    let result = (|| {
        main.set_always_on_top(staged[0])
            .map_err(|error| error.to_string())?;
        claude
            .set_always_on_top(staged[1])
            .map_err(|error| error.to_string())?;
        openai
            .set_always_on_top(staged[2])
            .map_err(|error| error.to_string())?;
        Ok(())
    })();
    match result {
        Ok(()) => Ok(()),
        Err(error) => {
            let rollback_values = transaction.rollback();
            let rollback = [
                main.set_always_on_top(rollback_values[0]),
                claude.set_always_on_top(rollback_values[1]),
                openai.set_always_on_top(rollback_values[2]),
            ];
            let mut rollback_error = None;
            for result in rollback {
                if let Err(error) = result {
                    remember_first_error(&mut rollback_error, error.to_string());
                }
            }
            match rollback_error {
                Some(rollback_error) => Err(format!(
                    "{error}; always-on-top rollback failed: {rollback_error}"
                )),
                None => Err(error),
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AlwaysOnTopTransaction {
    previous: [bool; 3],
    desired: bool,
}

impl AlwaysOnTopTransaction {
    fn new(previous: [bool; 3], desired: bool) -> Self {
        Self { previous, desired }
    }

    fn staged(&self) -> [bool; 3] {
        [self.desired; 3]
    }

    fn rollback(&self) -> [bool; 3] {
        self.previous
    }
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

type NativeFrame = (i32, i32, i32, i32, i32);

#[derive(Debug, Clone, PartialEq, Eq)]
struct NativeRollbackSnapshot {
    states: MaterialWindowStates,
    visible: [bool; 2],
    native_frames: [Option<NativeFrame>; 2],
}

impl NativeRollbackSnapshot {
    fn new(states: MaterialWindowStates, visible: [bool; 2]) -> Self {
        let native_frames = PROVIDERS.map(|provider| states.get(provider).frame);
        Self {
            states,
            visible,
            native_frames,
        }
    }

    #[cfg(target_os = "windows")]
    fn capture(app: &tauri::AppHandle, states: &MaterialWindowStates) -> Result<Self, String> {
        let mut snapshot = Self::new(states.clone(), [false; 2]);
        for provider in PROVIDERS {
            let Some(window) = app.get_window(MaterialWindowStates::label_for(provider)) else {
                continue;
            };
            let slot = provider_slot(provider);
            snapshot.visible[slot] = window.is_visible().map_err(|error| error.to_string())?;
            let position = window.outer_position().map_err(|error| error.to_string())?;
            let size = window.outer_size().map_err(|error| error.to_string())?;
            let width = i32::try_from(size.width).map_err(|_| {
                format!(
                    "{} width is too large",
                    MaterialWindowStates::label_for(provider)
                )
            })?;
            let height = i32::try_from(size.height).map_err(|_| {
                format!(
                    "{} height is too large",
                    MaterialWindowStates::label_for(provider)
                )
            })?;
            snapshot.native_frames[slot] = Some((
                position.x,
                position.y,
                width,
                height,
                states.get(provider).radius.unwrap_or_default(),
            ));
        }
        Ok(snapshot)
    }
}

fn invalidate_cached_states(states: &mut MaterialWindowStates) {
    for provider in PROVIDERS {
        *states.get_mut(provider) = NativeWindowState::default();
    }
}

fn finalize_failed_native_transaction(
    current: &mut MaterialWindowStates,
    snapshot: &NativeRollbackSnapshot,
    error: String,
    rollback: Result<(), String>,
) -> Result<(), String> {
    match rollback {
        Ok(()) => {
            *current = snapshot.states.clone();
            Err(error)
        }
        Err(rollback_error) => {
            invalidate_cached_states(current);
            Err(format!("{error}; native rollback failed: {rollback_error}"))
        }
    }
}

fn error_with_cleanup(app: &tauri::AppHandle, error: String) -> Result<(), String> {
    match hide_all_unlocked(app) {
        Ok(()) => Err(error),
        Err(cleanup_error) => Err(format!("{error}; backdrop cleanup failed: {cleanup_error}")),
    }
}

fn error_with_overlay_cleanup(app: &tauri::AppHandle, error: String) -> Result<(), String> {
    match hide_overlay_unlocked(app) {
        Ok(()) => Err(error),
        Err(cleanup_error) => Err(format!("{error}; overlay cleanup failed: {cleanup_error}")),
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
    use windows_sys::Win32::UI::WindowsAndMessaging::{SetWindowPos, SWP_NOACTIVATE};

    let (x, y, width, height, _) = frame;
    if unsafe {
        SetWindowPos(
            backdrop_hwnd,
            main_hwnd,
            x,
            y,
            width,
            height,
            SWP_NOACTIVATE,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn restore_native_snapshot(
    app: &tauri::AppHandle,
    main_hwnd: windows_sys::Win32::Foundation::HWND,
    snapshot: &NativeRollbackSnapshot,
) -> Result<(), String> {
    let mut first_error = None;
    for provider in PROVIDERS {
        let slot = provider_slot(provider);
        let Some(window) = app.get_window(MaterialWindowStates::label_for(provider)) else {
            if snapshot.native_frames[slot].is_some() {
                remember_first_error(
                    &mut first_error,
                    format!(
                        "missing native backdrop window {} during rollback",
                        MaterialWindowStates::label_for(provider)
                    ),
                );
            }
            continue;
        };

        if let Err(error) = crate::material::restore_backdrop(
            &window,
            snapshot.states.get(provider),
            snapshot.native_frames[slot].map(|frame| {
                (
                    u32::try_from(frame.2).unwrap_or_default(),
                    u32::try_from(frame.3).unwrap_or_default(),
                )
            }),
        ) {
            remember_first_error(&mut first_error, error);
        }
        if let Some(frame) = snapshot.native_frames[slot] {
            match window.hwnd() {
                Ok(hwnd) => {
                    if let Err(error) = place_backdrop_behind_main(hwnd.0 as _, main_hwnd, frame) {
                        remember_first_error(&mut first_error, error);
                    }
                }
                Err(error) => remember_first_error(&mut first_error, error.to_string()),
            }
        }
        let visibility_result = if snapshot.visible[slot] {
            window.show()
        } else {
            window.hide()
        };
        if let Err(error) = visibility_result {
            remember_first_error(&mut first_error, error.to_string());
        }
    }
    first_error.map_or(Ok(()), Err)
}

#[cfg(target_os = "windows")]
pub(crate) fn apply_plans_unlocked(
    app: &tauri::AppHandle,
    plans: [BackdropPlan; 2],
    foreground_visible: bool,
) -> Result<(), String> {
    let main = match app.get_webview_window("main") {
        Some(window) => window,
        None => return error_with_cleanup(app, "no main window".to_string()),
    };
    let main_hwnd = match main.hwnd() {
        Ok(hwnd) => hwnd.0,
        Err(error) => return error_with_cleanup(app, error.to_string()),
    };
    let main_hwnd = main_hwnd as windows_sys::Win32::Foundation::HWND;
    let state = app.state::<crate::AppState>();
    let mut states = match state.material_windows.lock() {
        Ok(states) => states,
        Err(error) => {
            return error_with_cleanup(app, format!("material window state lock poisoned: {error}"))
        }
    };

    let snapshot = match NativeRollbackSnapshot::capture(app, &states) {
        Ok(snapshot) => snapshot,
        Err(error) => return error_with_cleanup(app, error),
    };

    let mut staged_states = MaterialWindowStates {
        claude: states.claude.clone(),
        openai: states.openai.clone(),
    };
    let mut visibility = GeometryVisibilityTransaction::new(foreground_visible);
    let result = (|| {
        hide_all_unlocked(app)?;
        for provider in PROVIDERS {
            let plan = plan_for(&plans, provider)?;
            let label = MaterialWindowStates::label_for(provider);
            let mut next = staged_state(staged_states.get(provider), plan);

            let Some(frame) = plan.frame else {
                visibility.stage_provider(provider, next.enabled);
                *staged_states.get_mut(provider) = next;
                continue;
            };
            let Some(material) = plan.material else {
                visibility.stage_provider(provider, next.enabled);
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
            let backdrop_hwnd = window.hwnd().map_err(|error| error.to_string())?.0;
            let backdrop_hwnd = backdrop_hwnd as windows_sys::Win32::Foundation::HWND;
            place_backdrop_behind_main(backdrop_hwnd, main_hwnd, frame)?;
            visibility.stage_provider(provider, next.enabled);
            *staged_states.get_mut(provider) = next;
        }
        visibility.complete_geometry();
        if let Some(enabled) = visibility.revealable() {
            reveal_providers(app, enabled)?;
        }
        Ok(())
    })();
    match commit_staged_states(&mut states, staged_states, result) {
        Ok(()) => Ok(()),
        Err(error) => {
            let rollback = restore_native_snapshot(app, main_hwnd, &snapshot);
            if rollback.is_err() {
                let cleanup = hide_all_unlocked(app);
                let result =
                    finalize_failed_native_transaction(&mut states, &snapshot, error, rollback);
                return match cleanup {
                    Ok(()) => result,
                    Err(cleanup_error) => result.map_err(|error| {
                        format!("{error}; fail-closed cleanup failed: {cleanup_error}")
                    }),
                };
            }
            finalize_failed_native_transaction(&mut states, &snapshot, error, rollback)
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn apply_plans_unlocked(
    app: &tauri::AppHandle,
    plans: [BackdropPlan; 2],
    _foreground_visible: bool,
) -> Result<(), String> {
    let _ = plans;
    hide_all_unlocked(app)
}

pub fn apply_plans(
    app: &tauri::AppHandle,
    plans: [BackdropPlan; 2],
    foreground_visible: bool,
) -> Result<(), String> {
    let state = app.state::<crate::AppState>();
    let _lifecycle = match state.native_lifecycle.lock() {
        Ok(lifecycle) => lifecycle,
        Err(error) => {
            return error_with_cleanup(app, format!("native lifecycle lock poisoned: {error}"))
        }
    };
    apply_plans_unlocked(app, plans, foreground_visible)
}

#[cfg(target_os = "windows")]
fn show_enabled_unlocked(app: &tauri::AppHandle) -> Result<(), String> {
    let main = match app.get_webview_window("main") {
        Some(window) => window,
        None => return error_with_cleanup(app, "no main window".to_string()),
    };
    let main_hwnd = match main.hwnd() {
        Ok(hwnd) => hwnd.0,
        Err(error) => return error_with_cleanup(app, error.to_string()),
    };
    let main_hwnd = main_hwnd as windows_sys::Win32::Foundation::HWND;
    let state = app.state::<crate::AppState>();
    let states = match state.material_windows.lock() {
        Ok(states) => states,
        Err(error) => {
            return error_with_cleanup(app, format!("material window state lock poisoned: {error}"))
        }
    };

    let result = (|| {
        hide_all_unlocked(app)?;
        let mut enabled = [false; 2];
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
            enabled[provider_slot(provider)] = true;
        }
        reveal_providers(app, enabled)?;
        Ok(())
    })();
    match result {
        Ok(()) => Ok(()),
        Err(error) => error_with_cleanup(app, error),
    }
}

#[cfg(not(target_os = "windows"))]
fn show_enabled_unlocked(_app: &tauri::AppHandle) -> Result<(), String> {
    Ok(())
}

pub fn show_enabled(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<crate::AppState>();
    let _lifecycle = match state.native_lifecycle.lock() {
        Ok(lifecycle) => lifecycle,
        Err(error) => {
            return error_with_cleanup(app, format!("native lifecycle lock poisoned: {error}"))
        }
    };
    show_enabled_unlocked(app)
}

pub fn reveal_overlay(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<crate::AppState>();
    let _lifecycle = match state.native_lifecycle.lock() {
        Ok(lifecycle) => lifecycle,
        Err(error) => {
            return error_with_overlay_cleanup(
                app,
                format!("native lifecycle lock poisoned: {error}"),
            )
        }
    };
    let main = match app.get_webview_window("main") {
        Some(window) => window,
        None => return error_with_overlay_cleanup(app, "no main window".to_string()),
    };
    let result = (|| {
        show_enabled_unlocked(app)?;
        main.show().map_err(|error| error.to_string())
    })();
    match result {
        Ok(()) => Ok(()),
        Err(error) => error_with_overlay_cleanup(app, error),
    }
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

    #[test]
    fn lifecycle_cleanup_retains_the_first_error_after_attempting_later_steps() {
        let mut first_error = None;
        remember_first_error(&mut first_error, "first hide failed".to_string());
        remember_first_error(&mut first_error, "second hide failed".to_string());

        assert_eq!(first_error.as_deref(), Some("first hide failed"));
    }

    #[test]
    fn always_on_top_transaction_stages_all_targets_and_can_restore_them() {
        let transaction = AlwaysOnTopTransaction::new([true, false, true], false);

        assert_eq!(transaction.staged(), [false, false, false]);
        assert_eq!(transaction.rollback(), [true, false, true]);
    }

    #[test]
    fn geometry_visibility_transaction_reveals_staged_providers_only_after_geometry() {
        let mut transaction = GeometryVisibilityTransaction::new(true);
        transaction.stage_provider(Provider::Openai, true);

        assert_eq!(transaction.revealable(), None);

        transaction.complete_geometry();

        assert_eq!(transaction.revealable(), None);

        transaction.stage_provider(Provider::Claude, false);

        assert_eq!(transaction.revealable(), Some([false, true]));
    }

    #[test]
    fn geometry_visibility_transaction_keeps_backdrops_hidden_with_foreground() {
        let mut transaction = GeometryVisibilityTransaction::new(false);
        transaction.stage_provider(Provider::Claude, true);
        transaction.stage_provider(Provider::Openai, true);
        transaction.complete_geometry();

        assert_eq!(transaction.revealable(), None);
    }

    #[test]
    fn native_rollback_snapshot_preserves_provider_state_and_visibility_isolation() {
        let mut states = MaterialWindowStates::default();
        states.claude.material = Some(crate::material::NativeMaterialSpec {
            material: crate::material::Material::Acrylic,
            tint: (7, 16, 31, 96),
        });
        states.claude.size = Some((300, 80));
        states.claude.radius = Some(12);
        states.claude.frame = Some((10, 20, 300, 80, 12));
        states.claude.enabled = true;
        states.openai.material = Some(crate::material::NativeMaterialSpec {
            material: crate::material::Material::Blur,
            tint: (27, 38, 49, 16),
        });
        states.openai.size = Some((320, 90));
        states.openai.radius = Some(14);
        states.openai.frame = Some((10, 110, 320, 90, 14));

        let snapshot = NativeRollbackSnapshot::new(states.clone(), [true, false]);

        assert_eq!(snapshot.states, states);
        assert_eq!(snapshot.visible, [true, false]);
        assert!(snapshot.states.claude.enabled);
        assert!(!snapshot.states.openai.enabled);
    }

    #[test]
    fn successful_native_rollback_restores_the_cached_snapshot() {
        let mut previous = MaterialWindowStates::default();
        previous.claude.enabled = true;
        previous.claude.frame = Some((10, 20, 300, 80, 12));
        let snapshot = NativeRollbackSnapshot::new(previous.clone(), [true, false]);

        let mut staged = MaterialWindowStates::default();
        staged.claude.enabled = true;
        staged.claude.frame = Some((30, 40, 320, 90, 14));

        let result = finalize_failed_native_transaction(
            &mut staged,
            &snapshot,
            "provider operation failed".to_string(),
            Ok(()),
        );

        assert_eq!(result, Err("provider operation failed".to_string()));
        assert_eq!(staged, previous);
    }

    #[test]
    fn failed_native_rollback_invalidates_both_cached_backdrops() {
        let mut previous = MaterialWindowStates::default();
        previous.claude.enabled = true;
        previous.claude.frame = Some((10, 20, 300, 80, 12));
        previous.openai.enabled = true;
        previous.openai.frame = Some((10, 110, 320, 90, 14));
        let snapshot = NativeRollbackSnapshot::new(previous, [true, true]);

        let mut staged = snapshot.states.clone();
        staged.claude.frame = Some((30, 40, 320, 90, 14));
        staged.openai.frame = Some((30, 140, 320, 100, 16));

        let result = finalize_failed_native_transaction(
            &mut staged,
            &snapshot,
            "provider operation failed".to_string(),
            Err("restore rounded region failed".to_string()),
        );

        assert!(result
            .unwrap_err()
            .contains("native rollback failed: restore rounded region failed"));
        assert_eq!(staged, MaterialWindowStates::default());
    }
}
