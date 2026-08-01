use crate::material::NativeWindowState;
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

pub fn hide_all(app: &tauri::App) -> Result<(), String> {
    for provider in [Provider::Claude, Provider::Openai] {
        if let Some(window) = app.get_window(MaterialWindowStates::label_for(provider)) {
            window.hide().map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

pub fn set_always_on_top(app: &tauri::App) -> Result<(), String> {
    for provider in [Provider::Claude, Provider::Openai] {
        if let Some(window) = app.get_window(MaterialWindowStates::label_for(provider)) {
            window
                .set_always_on_top(true)
                .map_err(|error| error.to_string())?;
        }
    }
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
}
